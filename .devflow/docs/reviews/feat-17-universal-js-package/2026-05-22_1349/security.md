# Security Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### MEDIUM

**TOCTOU race between lstat and readFile in symlink check** - `packages/mds/src/util/module-scanner.ts:111-123`
**Confidence**: 85%
- Problem: The symlink rejection uses `lstat()` (line 111) to check if a path is a symlink, then separately calls `readFile()` (line 123). Between these two calls, an attacker with write access to the filesystem could replace a regular file with a symlink (classic time-of-check/time-of-use race). This could allow reading files outside the project root by creating a symlink after the lstat check but before readFile is called.
- Impact: In practice, this requires the attacker to have write access to the project directory at exactly the right moment. The window is narrow but non-zero for server-side use cases where untrusted users trigger compilation.
- Fix: Open the file with `open()` from `node:fs/promises`, then use `fstat()` on the file descriptor to check if it's a symlink, and finally read from that same descriptor. This eliminates the race window:
  ```typescript
  import { open } from 'node:fs/promises';
  
  const fh = await open(absolutePath, 'r');
  try {
    const stats = await fh.stat();
    if (stats.isSymbolicLink()) {
      throw new Error(`security: symlink detected at ${absolutePath}`);
    }
    const content = await fh.readFile('utf-8');
    // ... continue
  } finally {
    await fh.close();
  }
  ```
  Note: `fh.stat()` on an opened file handle will NOT detect symlinks since the fd already resolved the link. The proper approach is to use `O_NOFOLLOW` flag (not natively exposed in Node.js `fs.open`). Alternatively, use `lstat` then `readFile` with `O_NOFOLLOW` via the `fs.open` flags parameter, or use `realpath()` and compare against the expected path:
  ```typescript
  import { realpath, readFile, lstat } from 'node:fs/promises';

  const stats = await lstat(absolutePath);
  if (stats.isSymbolicLink()) {
    throw new Error(`security: symlink detected at ${absolutePath}`);
  }
  // Double-check via realpath comparison
  const resolved = await realpath(absolutePath);
  if (resolved !== absolutePath) {
    throw new Error(`security: path resolved differently (possible symlink race)`);
  }
  const content = await readFile(absolutePath, 'utf-8');
  ```

---

**Project root scope is too narrow — only entry file's parent directory** - `packages/mds/src/util/module-scanner.ts:97`
**Confidence**: 82%
- Problem: `projectRoot` is set to `dirname(absoluteEntry)` — the immediate parent directory of the entry file. If the project has a nested directory structure (e.g., `src/templates/main.mds` importing `src/shared/common.mds`), legitimate imports from sibling directories outside the entry file's parent would be rejected. More critically for security: if the entry file is in a deeply nested path, the project root could be very narrow, but if the entry file is at the filesystem root (e.g., `/main.mds`), then `projectRoot` becomes `/`, effectively disabling the path traversal check.
- Impact: If an entry path resolves to a file at the filesystem root or a very high-level directory, the traversal guard becomes permissive. A path like `/tmp/main.mds` would set projectRoot to `/tmp`, which is reasonable, but an entry at `/main.mds` would set projectRoot to `/` — any path on the system would pass the check.
- Fix: Consider accepting an explicit `projectRoot` parameter rather than inferring it solely from the entry file's parent, or assert that the resolved `projectRoot` is not a filesystem root:
  ```typescript
  if (projectRoot === '/' || projectRoot === '') {
    throw new Error('security: project root cannot be filesystem root');
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**MDS_BACKEND environment variable not validated against enum** - `packages/mds/src/node.ts:10`
**Confidence**: 80%
- Problem: `process.env['MDS_BACKEND']` is cast directly to `BackendType | undefined` without validation. While the current code only branches on `=== 'wasm'` and `=== 'native'` (so arbitrary values fall through to the default path), the type assertion makes it appear validated when it is not. If future code relies on this typed value being strictly `'native' | 'wasm' | undefined`, unexpected behavior could result.
- Impact: Low immediate risk since the branching logic only acts on known values. However, the cast suppresses TypeScript's type safety and could mask bugs in future changes.
- Fix: Validate the env var value explicitly:
  ```typescript
  const rawBackend = process.env['MDS_BACKEND'];
  const forceBackend: BackendType | undefined =
    rawBackend === 'native' || rawBackend === 'wasm' ? rawBackend : undefined;
  if (rawBackend !== undefined && forceBackend === undefined) {
    console.warn(`@mds/mds: ignoring unrecognized MDS_BACKEND="${rawBackend}" (expected "native" or "wasm")`);
  }
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing `realpath` comparison after symlink check** - `packages/mds/src/util/module-scanner.ts:117` (Confidence: 70%) — The `startsWith(projectRoot + '/')` check uses the path returned by `resolve()` which normalizes `..` segments but does not resolve symlinks in parent directory components. If a parent directory in the path is itself a symlink, `resolve()` would not catch it. Adding a `realpath()` comparison would close this gap.

- **No rate limiting / timeout on recursive import scanning** - `packages/mds/src/util/module-scanner.ts:144` (Confidence: 65%) — While module count and aggregate size are bounded, deeply nested imports that individually stay within limits but involve many filesystem operations could be used for denial-of-service via I/O exhaustion. The 256-module limit provides reasonable protection, but a wall-clock timeout on the overall scan would add defense-in-depth.

- **WASM module path candidates include relative filesystem traversal** - `packages/mds/src/backend/wasm.ts:62` (Confidence: 62%) — The `new URL('../../../../crates/mds-wasm/pkg/mds_wasm.js', import.meta.url).pathname` candidate traverses up four directories from the source file's location. In a production deployment this path would not exist, but in compromised environments where directory structure is attacker-controlled, this could load a malicious module. Low practical risk since Node.js `require()` is already a trust boundary.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The security posture is solid for a pre-release developer tool. The path traversal protections (null byte rejection, segment counting, `..` escape prevention, project root prefix check) and resource limits (module count, aggregate size) are well-implemented. The WASM boundary enforces input size limits correctly with panic isolation.

The two blocking MEDIUM findings are the TOCTOU race in symlink detection and the edge case where project root could be filesystem root. Both have narrow exploitation windows (requiring write access to the project directory at runtime, or a user explicitly invoking the tool on a file at `/`), but should be addressed before any server-side deployment where untrusted users can trigger compilation.

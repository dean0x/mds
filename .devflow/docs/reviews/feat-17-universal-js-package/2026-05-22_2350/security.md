# Security Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Aggregate-size race condition with parallel scan — partial mitigation** - `packages/mds/src/util/module-scanner.ts:176`
**Confidence**: 82%
- Problem: The `aggregateSize` variable is a shared mutable number updated by concurrent `scan()` calls (spawned via `Promise.all` at line 191). Although the diff comment says "pre-reserve file size... before reading content so that parallel scan calls cannot each pass the check independently and collectively overshoot the limit," the `aggregateSize += stats.size` and subsequent `if` check on lines 176-181 are not atomic. Two concurrent scan calls could both read `aggregateSize` before either writes the incremented value, allowing the aggregate to exceed the limit. JavaScript is single-threaded so this is safe within a single microtask tick, but each `scan()` call `await`s `lstat` and `realpath` before reaching line 176 — the interleaving across `Promise.all` children means two siblings could each pass their own aggregate check before seeing the other's addition, collectively overshooting by up to one file's worth. The practical impact is limited (one file overshoot, not unbounded), so this is HIGH not CRITICAL.
- Fix: Accept this as a known limitation (document it), or switch to sequential child scanning to make the limit exact:
```typescript
// Sequential alternative (trades parallelism for exact limit):
for (const importPath of importPaths) {
  const childAbsolute = validateImportPath(importPath, absoluteDir);
  const childVirtualKey = normalizeVirtualKey(virtualKey, importPath);
  await scan(childAbsolute, childVirtualKey);
}
```

### MEDIUM

**TOCTOU gap between `realpath` and `readFile`** - `packages/mds/src/util/module-scanner.ts:158-183`
**Confidence**: 80%
- Problem: The code performs `lstat` (line 151), then `realpath` (line 158), then `readFile` (line 183) as three separate async operations. An attacker who can manipulate the filesystem could swap a regular file for a symlink between the `realpath` check and the `readFile` call. The existing TOCTOU mitigation (comparing `realpath` to `absolutePath`) is good defense-in-depth but cannot fully close this gap in userspace — the only fully race-free approach is opening the file descriptor once and operating on it. That said, exploiting this requires local filesystem access with precise timing, making practical exploitation difficult.
- Fix: Use `fs.open()` to get a file handle, then `fstat` the handle and read from it, so all operations use the same underlying inode:
```typescript
import { open } from 'node:fs/promises';
const handle = await open(absolutePath, 'r');
try {
  const stats = await handle.stat();
  if (stats.isSymbolicLink()) { /* reject */ }
  const content = await handle.readFile('utf-8');
} finally {
  await handle.close();
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Warning message leaks raw env var value to console** - `packages/mds/src/node.ts:14`
**Confidence**: 80%
- Problem: `console.warn(\`...ignoring unknown MDS_BACKEND value "${rawBackend}"...\`)` interpolates the raw environment variable directly into the warning. While environment variables are not typically attacker-controlled, in shared hosting or CI environments, env vars can be set by upstream processes. The value is logged to stderr (not rendered in HTML), so the impact is log injection — an attacker could insert newlines or control characters to forge log entries. Severity is MEDIUM because the attack surface is narrow (env vars, not user input) and the output is stderr, not a security-sensitive channel.
- Fix: Sanitize or truncate the value before logging:
```typescript
const sanitized = rawBackend.replace(/[^\x20-\x7E]/g, '?').slice(0, 50);
console.warn(`@mds/mds: ignoring unknown MDS_BACKEND value "${sanitized}"; expected "native" or "wasm"`);
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`varsOpt` passes user-supplied vars object by reference** - `packages/mds/src/util/options.ts:11` (Confidence: 65%) -- The `vars` object from user code is passed directly to the native/WASM backend without shallow-copying. If the backend mutates the object, it could affect the caller's state. Low risk since the backend is compiled Rust (napi/WASM) and unlikely to mutate JS objects.

- **`initFailures` counter is module-level mutable state without reset** - `packages/mds/src/backend/wasm.ts:32` (Confidence: 70%) -- The `initFailures` counter increments on each failed init attempt but has no way to reset. If a transient failure causes 3 retries early in application lifetime, the WASM backend becomes permanently unusable for the process lifetime even if the underlying issue is resolved. This is a reliability concern more than security, but permanent denial-of-service to the WASM backend from transient failures has security implications in fallback scenarios.

- **`validateImportPath` does not reject absolute import paths** - `packages/mds/src/util/module-scanner.ts:114-133` (Confidence: 65%) -- If `importPath` is an absolute path (e.g. `/etc/passwd`), `resolve(absoluteDir, importPath)` returns the absolute path unchanged, which would then fail the `startsWith(projectRoot)` check. This is correctly caught by the path-escape guard, but rejecting absolute paths explicitly would be clearer defense-in-depth.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The security posture of this PR is strong. The module scanner implements multiple layers of defense: symlink rejection, path traversal guards, null byte checks, filesystem root guard, TOCTOU detection, and resource limits (module count, aggregate size). These are well-structured and correctly ordered.

The one blocking HIGH issue (aggregate-size race condition) has limited practical impact — concurrent scans can overshoot the aggregate size limit by at most one file's worth due to JavaScript's cooperative scheduling. The TOCTOU gap between `realpath` and `readFile` is a known limitation of userspace symlink defense and is partially mitigated. The env var log injection is low-risk given the narrow attack surface.

**Conditions for approval**: Acknowledge the aggregate-size race as a known limitation (comment or accept) — no code change strictly required given the bounded overshoot.

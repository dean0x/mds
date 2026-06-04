# Performance Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-24
**PR**: #29

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**File descriptor leak risk on validation failure in openAndValidateModule** - `packages/mds/src/util/module-scanner.ts:172-197`
**Confidence**: 85%
- Problem: The `openAndValidateModule` function opens a file descriptor at line 162, then inside the `try/finally` block performs `handle.stat()`, `realpath()`, and validation checks (lines 172-192). If `realpath(absolutePath)` throws (e.g., ENOENT race, permission denied), the `finally` block correctly closes the handle. However, the `handle.readFile()` call at line 194 occurs *after* the size is known but *before* the caller checks the aggregate size limit. The old approach (`statAndValidateModule` + separate `readFile`) allowed the caller to check the size limit *before* reading the content, avoiding unnecessary I/O for files that would exceed the limit. In the new approach, the file content is always read (consuming memory and I/O bandwidth) even if the aggregate size check at line 232 will immediately reject it.
- Impact: For malicious or pathological inputs with many large files near the aggregate size limit, the system reads all file contents before rejecting -- wasting I/O and memory. With 256 modules allowed and a 10 MiB aggregate limit, worst case reads up to ~2.5 GB (256 files * 10 MiB each) before the aggregate check kicks in at the caller level.
- Fix: Split the function into two phases: (1) open + stat (return size, keep handle open), (2) read content after the caller validates aggregate size. Or check the aggregate size inside `openAndValidateModule` by passing the current aggregate size and limit as parameters.

```typescript
// Option A: Pass aggregate state into the validator
async function openAndValidateModule(
  absolutePath: string,
  currentAggregateSize: number,
  maxAggregateSize: number,
): Promise<{ size: number; content: string }> {
  // ... open + stat as before ...
  // Check aggregate limit BEFORE reading content:
  if (currentAggregateSize + stats.size > maxAggregateSize) {
    throw new Error(`resource limit: aggregate module size exceeds maximum of ${maxAggregateSize} bytes`);
  }
  const content = await handle.readFile({ encoding: 'utf-8' });
  return { size: stats.size, content };
}
```

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Unused `lstat` import after refactor** - `packages/mds/src/util/module-scanner.ts:1`
**Confidence**: 95%
- Problem: `lstat` is imported from `node:fs/promises` but never called in the refactored code. The old `statAndValidateModule` used `lstat`; the new `openAndValidateModule` uses `handle.stat()` (fstat on the fd) instead. The import is dead code.
- Impact: Minor -- no runtime cost, but adds noise and may confuse future maintainers into thinking lstat is still part of the validation chain. Tree-shaking at the module boundary does not apply to Node.js built-in imports.
- Fix: Remove `lstat` from the import.

```typescript
import { open, realpath } from 'node:fs/promises';
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **U-PF0 test measures subprocess spawn time, not import time** - `packages/mds/__test__/perf.spec.mjs:26-34` (Confidence: 70%) -- The test uses `Date.now()` around `execFileSync`, which includes Node.js subprocess spawn/teardown overhead (~50-200ms). The 5000ms threshold is generous enough that this does not produce false negatives, but it also means TLA regressions under ~4.8s would pass. The FEATURE_KNOWLEDGE notes this as a best-effort check (U-PF0), so the generous threshold is intentional.

- **`wrapWithFileOps` creates a new object via spread on every `init()` call** - `packages/mds/src/node.ts:62-63` (Confidence: 65%) -- `{ ...base, async compileFile(...) {...}, async checkFile(...) {...} }` creates a new object each time. Since `init()` is called once per process lifetime and the spread is over a 3-property object, the cost is negligible. Mentioning only for completeness.

- **Browser `initWasmBrowser` has no retry exhaustion counter (unlike Node.js `initWasmNode`)** - `packages/mds/src/backend/wasm.ts:206-215` (Confidence: 75%) -- `initWasmBrowser` clears `cachedBrowserPromise` on failure, allowing unlimited retries. Unlike `initWasmNode` which caps at `MAX_INIT_RETRIES=3`, a browser app stuck in a retry loop calling `init()` would re-attempt the dynamic import indefinitely. This may be intentional (the comment says "simpler than Node.js"), but in a pathological case (e.g., polling `init()` in a `setInterval`), it could generate unbounded network requests to fetch the WASM module.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The refactor achieves its primary performance goal: removing TLA from `node.ts` so that module import completes synchronously without blocking I/O. The `DEFAULT_COMPILE_OPTS` freeze pattern correctly prevents WASM FFI mutation of shared state (as noted in FEATURE_KNOWLEDGE). The lazy-init with promise deduplication is well-structured and avoids double-init races.

The single blocking issue is the aggregate-size-before-read ordering change in `module-scanner.ts`. The old code validated file size *before* reading content (two separate calls: `statAndValidateModule` then `readFile`). The new `openAndValidateModule` combines stat+read into a single function, which is better for TOCTOU elimination but loses the ability to reject oversized aggregates before committing I/O and memory to read the content. This should be addressed before merge.

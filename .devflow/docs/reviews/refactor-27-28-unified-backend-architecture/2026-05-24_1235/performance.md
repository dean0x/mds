# Performance Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24
**Scope**: Incremental review (4 commits: c57685c...HEAD)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**File handle leak on aggregate size limit rejection** - `packages/mds/src/util/module-scanner.ts:247-252`
**Confidence**: 85%
- Problem: When the aggregate size check fails at line 247, the code calls `await handle.close()` then throws. However, the `openAndValidateModule` function now returns the handle for the caller to manage. The close-then-throw at line 248 is correct for this error path, but the handle was obtained at line 239 outside of any try/finally. If any code between line 239 and line 255 (the try/finally that reads and closes) throws unexpectedly (e.g., an OOM on the `aggregateSize += fileSize` line is unlikely but the gap exists), the handle leaks. The entire post-`openAndValidateModule` section (lines 239-259) should be wrapped in a single try/finally to ensure the handle is always closed.
- Impact: Under adversarial conditions (many files near the aggregate limit), leaked file descriptors accumulate, potentially exhausting the process's fd limit and causing EMFILE errors on subsequent opens.
- Fix: Wrap the handle usage in a single try/finally:
  ```typescript
  const { handle, size: fileSize } = await openAndValidateModule(absolutePath);
  try {
    aggregateSize += fileSize;
    if (aggregateSize > maxAggregateSize) {
      throw new Error(
        `resource limit: aggregate module size exceeds maximum of ${maxAggregateSize} bytes`,
      );
    }
    content = await handle.readFile({ encoding: 'utf-8' });
  } finally {
    await handle.close();
  }
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Sequential candidate loading in _initNode** - `packages/mds/src/backend/wasm.ts:192-203`
**Confidence**: 82%
- Problem: The candidate loading loop in `_initNode` tries each WASM candidate path sequentially. Currently there are only 2 candidates (workspace path, npm path), so the cost is minimal. However, if the first candidate exists but has a slow `wasmMod.default()` initialization (e.g., large WASM binary), the second candidate cannot be attempted until the first completes or fails. This is architectural -- noting for future awareness, not a current bottleneck.
- Impact: Negligible with 2 candidates. Would matter if the candidate list grows.

## Suggestions (Lower Confidence)

- **`Promise.all` for stat + realpath in openAndValidateModule** - `packages/mds/src/util/module-scanner.ts:187-190` (Confidence: 70%) -- The parallel `Promise.all([handle.stat(), realpath(absolutePath)])` is a good micro-optimization over sequential calls. Already implemented correctly; no action needed. Noting as a positive pattern.

- **Redundant `validateWasmShape` call after successful `import('mds-wasm')`** - `packages/mds/src/backend/wasm.ts:269-270` (Confidence: 65%) -- In `_initBrowser`, after `validateWasmShape(imported)` passes at line 269, the code checks `typeof wasmMod.default !== 'function'` at line 272. Since `default` is an optional field on WasmModule (not checked by `validateWasmShape`), this is correct and not redundant. But combining both checks into `validateWasmShape` (with an optional `requireDefault` parameter) would avoid walking the module exports twice.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The changes are performance-positive overall:

1. **Aggregate size check before readFile** (module-scanner.ts:241-252): Checking `fstat` size metadata before loading content into memory prevents allocation of data that will be rejected. This is a meaningful improvement -- under the old code, a malicious 9MB file would be fully read into memory before the aggregate check rejected it.

2. **`openNoFollow` extraction**: Moving the try/catch to a module-level helper reduces nesting in `openAndValidateModule` without adding overhead (no closure capture, same number of syscalls).

3. **`validateWasmShape` consolidation**: The loop-based shape check is marginally cleaner than the old inline `if` chain but performs identically (3 `typeof` checks either way). No performance impact.

The one blocking condition is the file handle leak potential in the gap between `openAndValidateModule` return and the try/finally around `readFile`. Wrapping the entire post-open section in a single try/finally closes this gap with zero performance cost.

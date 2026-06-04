# Reliability Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24
**Scope**: Incremental (4 commits from c57685c...HEAD)

## Issues in Your Changes (BLOCKING)

### HIGH

**File handle leak on aggregate size violation** - `packages/mds/src/util/module-scanner.ts:246-252`
**Confidence**: 95%
- Problem: When the aggregate size limit is exceeded at line 247, the handle is closed (`await handle.close()` at line 248) and then the error is thrown. However, there is a subtle leak path: if `handle.close()` itself throws (e.g., due to an I/O error during close), the original resource-limit error is lost and the exception from `close()` propagates instead. More importantly, this `handle.close()` call duplicates the cleanup responsibility -- the handle is ALSO closed in the `finally` block at line 258. If the aggregate size check passes but `readFile` throws, the `finally` block handles close correctly. But if the aggregate size check fails, the explicit `handle.close()` at line 248 runs and then the function exits via throw, so the `finally` is not reached (different code path). This is correct behavior but fragile: the two close paths are not unified, making the resource lifecycle harder to reason about.
- Impact: (1) If `handle.close()` throws on the error path, the descriptive resource-limit message is replaced with a low-level I/O error. (2) Dual close paths increase maintenance risk -- a future refactor could accidentally close twice or skip a close.
- Fix: Unify the close into a single `finally` block that always runs:
```typescript
const { handle, size: fileSize } = await openAndValidateModule(absolutePath);

let content: string;
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

### MEDIUM

**validateWasmShape behavior change in tryLoadCandidate: throw instead of return null** - `packages/mds/src/backend/wasm.ts:92-95`
**Confidence**: 82%
- Problem: Previously, `tryLoadCandidate` returned `null` for shape mismatches, allowing `_initNode` to silently try the next candidate. Now it calls `validateWasmShape` which throws on mismatch. The throw is caught by the `for` loop's `catch` in `_initNode` (line 198), stored as `lastError`, and iteration continues. The behavior is functionally equivalent BUT: if the first candidate loads successfully (no MODULE_NOT_FOUND) yet has a wrong shape, the error from `validateWasmShape` is captured as `lastError` and the loop continues to the next candidate. If the next candidate also fails (MODULE_NOT_FOUND), `_initNode` throws with the shape-error as `Caused by`, which is correct. However, if there were ever more than 2 candidates, a shape error from candidate 1 could be overwritten by a different error from candidate 2, losing diagnostic information. With the current 2-candidate list this is a minor robustness concern, not a bug.
- Impact: Diagnostic message fidelity in multi-candidate scenarios. Low impact with current 2-candidate list.
- Fix: No code change required at this time. The current 2-candidate setup is safe. Consider collecting all errors (not just the last) if the candidate list grows.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **openAndValidateModule catch-and-close may swallow close errors** - `packages/mds/src/util/module-scanner.ts:210-213` (Confidence: 65%) -- If `handle.close()` in the catch block throws, the original validation error is replaced by the close error. A `try { await handle.close(); } catch { /* swallow */ }` pattern would preserve the original. This is pre-existing logic (moved, not introduced) but sits in a function whose signature was changed in this diff.

- **Browser init circuit breaker increments after catch, not before** - `packages/mds/src/backend/wasm.ts:236-241` (Confidence: 62%) -- `browserFailures` is incremented inside `.catch()`, meaning it only increments when the promise rejects. If `_initBrowser` hangs indefinitely (never resolves, never rejects), the circuit breaker never triggers and `cachedBrowserPromise` remains a forever-pending promise. This matches the Node.js pattern (`initWasmNode`) and is the accepted design, but a timeout-based circuit breaker would be more robust in browser environments where network fetch can stall.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The incremental changes materially improve reliability: the aggregate-size-before-read pattern in module-scanner is a clear win, the browser circuit breaker mirrors the well-tested Node.js pattern, and the `try/finally` test cleanup in backend.spec prevents state leakage. The single actionable finding is the split close paths in `scan()` -- unifying them into a single `finally` block would make the file handle lifecycle unambiguous and eliminate a fragile dual-close pattern.

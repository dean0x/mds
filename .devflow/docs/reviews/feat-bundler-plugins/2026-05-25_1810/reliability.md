# Reliability Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10
**Cycle**: 2 (incremental — Cycle 1 resolved 18/20 issues including poisoned-promise patterns)

## Issues in Your Changes (BLOCKING)

### HIGH

**Non-null assertion after await relies on fragile invariant** - `packages/webpack-loader/src/index.ts:37`
**Confidence**: 82%
- Problem: `return transformer!` after `await initPromise` is correct only because the `.then()` callback sets `transformer` synchronously before the promise chain resolves. However, if anyone refactors the `.then()` into an `async` callback, adds an `await` inside it, or reorders the chain, `transformer` could be null when the assertion fires. The non-null assertion silently masks this — there is no runtime guard.
- Impact: A future refactor could introduce a silent null dereference that bypasses TypeScript's type checker entirely. The `!` assertion is effectively an unchecked precondition.
- Fix: Add a runtime assertion after the await to fail loudly instead of silently:
```typescript
await initPromise;
if (transformer === null) {
  throw new Error('Invariant violation: transformer not initialized after init resolved');
}
return transformer;
```

### MEDIUM

**`_resetForTesting` production guard is bypassable and incomplete** - `packages/webpack-loader/src/index.ts:63-68`
**Confidence**: 83%
- Problem: The production guard checks `process.env['NODE_ENV'] === 'production'`, but `NODE_ENV` is not set in many deployment environments (serverless, Docker without explicit env). Any environment where `NODE_ENV` is absent (undefined) bypasses the guard. Additionally, `_resetForTesting` is exported publicly from the package — any consumer can import and call it.
- Impact: In a non-standard deployment where `NODE_ENV` is unset, accidental use of `_resetForTesting` could null out the singleton mid-build, causing subsequent loader calls to re-initialize and potentially lose in-flight state.
- Fix: Invert the check to allowlist test environments rather than denylist production:
```typescript
export function _resetForTesting(): void {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_resetForTesting is only allowed when NODE_ENV=test');
  }
  transformer = null;
  initPromise = null;
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Concurrent rejection fan-out in ensureInit** - `packages/bundler-utils/src/transform.ts:36-41` (Confidence: 65%) — When multiple concurrent callers await the same `initPromise` and it rejects, all callers receive the rejection, but only the first rejection handler resets `initPromise = null`. Subsequent callers then retry correctly, but in the window between rejection and retry, the `initialized` flag remains `false` while `initPromise` is `null` — this is the correct state for retry, but the lack of an explicit assertion like `assert(!initialized)` at the top of the retry path means a future bug that sets `initialized = true` before `initPromise` resolves would cause silent corruption.

- **Dynamic import has no timeout** - `packages/webpack-loader/src/index.ts:24`, `packages/rollup-plugin/src/index.ts:27`, `packages/vite-plugin/src/index.ts:32` (Confidence: 62%) — `await import('@mds/mds')` has no timeout or abort signal. If the WASM backend hangs during initialization (e.g., due to a corrupted binary or blocked I/O), the build will hang indefinitely. In bundler contexts this is generally acceptable since the bundler itself has build timeouts, but it is worth noting as a reliability gap for production WASM workloads.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions
1. Replace the `transformer!` non-null assertion in webpack-loader with a runtime invariant check (HIGH).

### Notes
The Cycle 1 poisoned-promise fixes in both `ensureInit` (bundler-utils) and `ensureTransformer` (webpack-loader) are correctly implemented — the rejection handler nulls the cached promise and re-throws, allowing subsequent callers to retry. The `initialized` boolean flag in bundler-utils and the `transformer !== null` early-return in webpack-loader both serve as correct double-checks that prevent stale state from persisting after a failed init. Resource cleanup (file handle in frontmatter.ts) uses try/finally correctly. No unbounded loops, no excessive indirection, no unsafe metaprogramming.

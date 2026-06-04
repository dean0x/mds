# Reliability Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25

## Cross-Cycle Awareness

Prior resolutions addressed: non-null assertion replaced with runtime invariant (webpack `ensureTransformer`), `_resetForTesting` guard inversion fixed, poisoned-promise style unified. These are verified as resolved in the current code. Webpack init/retry logic duplication was deferred to tech debt -- confirmed still present but out of scope for this review cycle.

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Missing environment guard on `_setTransformerForTesting` in rollup-plugin and vite-plugin** -- `packages/rollup-plugin/src/index.ts:34`, `packages/vite-plugin/src/index.ts:40`
**Confidence**: 85%
- Problem: `_setTransformerForTesting` in rollup-plugin and vite-plugin has no `NODE_ENV` guard, unlike the webpack-loader which throws unless `NODE_ENV=test`. Both functions mutate module-level state (`_testTransformer`) that overrides the real transformer for every plugin instance created from that module. If called accidentally in production (e.g., test code leaking into a build, or a downstream library importing and calling it), all `.mds` transforms would silently use the mock transformer with no error or warning.
- Impact: In a production build, the mock transformer would produce whatever output the test injected (or `null` if set to `null`), silently corrupting the build output. The webpack-loader's guard is the correct pattern -- it was fixed in a prior cycle. The same defense should apply to the other two plugins.
- Fix: Add the same `NODE_ENV` guard used in webpack-loader:
```typescript
// rollup-plugin/src/index.ts and vite-plugin/src/index.ts
export function _setTransformerForTesting(t: ReturnType<typeof createMdsTransformer> | null): void {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  _testTransformer = t;
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Silent error swallowing in `shouldTransform` for `.md` files** -- `packages/bundler-utils/src/frontmatter.ts:63` (Confidence: 65%) -- The `.catch(() => false)` on the file-open promise silently treats all I/O errors (permission denied, disk failure) as "not an MDS file". For `.mds` files this is fine (synchronous `true` return), but for `.md` files with frontmatter detection, a transient I/O error would silently skip compilation rather than surfacing the problem. Consider logging or at least distinguishing `ENOENT` (expected) from other errors.

- **No upper bound on `result.dependencies` / `result.warnings` iteration** -- `packages/vite-plugin/src/index.ts:74-79`, `packages/rollup-plugin/src/index.ts:67-72`, `packages/webpack-loader/src/index.ts:46-51` (Confidence: 60%) -- The `for...of` loops over `result.dependencies` and `result.warnings` have no upper bound guard. If the compiler ever returns an extremely large dependency or warning array (e.g., due to a bug in circular import resolution), these loops would call `addWatchFile`/`addDependency` and `warn`/`emitWarning` an unbounded number of times. In practice the compiler controls these arrays, so this is low-probability, but a defensive cap (e.g., warn once and truncate after 1000) would match the bounded-iteration principle.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The init/retry patterns are solid: poisoned-promise reset on failure, runtime invariant in webpack's `ensureTransformer`, and proper resource cleanup in `shouldTransform` (try/finally on file handles). The one blocking issue is the missing environment guard on `_setTransformerForTesting` in rollup-plugin and vite-plugin -- this is a consistency gap with the webpack-loader that was fixed in a prior review cycle, and the same defense should be applied to the other two plugins to prevent accidental mutation of module-level state in production.

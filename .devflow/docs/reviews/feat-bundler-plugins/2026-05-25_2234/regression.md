# Regression Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T22:34
**Diff**: `git diff b1d6b6a1610fe7664d53cbddad2e943209ff61fb...HEAD`
**Tests**: 80/80 passing (bundler-utils 48, vite-plugin 14, rollup-plugin 10, webpack-loader 8)

## Issues in Your Changes (BLOCKING)

### HIGH

**Webpack ensureTransformer: .then(a,b) loses error recovery for onFulfilled failures** - `packages/webpack-loader/src/index.ts:28-31`
**Confidence**: 85%
- Problem: The refactor from `.then().catch()` to `.then(onFulfilled, onRejected)` changes error handling semantics. With `.then(a).catch(b)`, the `.catch()` catches errors from both the import AND `createMdsTransformer(mds, options)`. With `.then(a, b)`, the `onRejected` handler only catches errors from the import -- if `createMdsTransformer` throws inside `onFulfilled`, the promise rejects but `initPromise` is never reset to `null`, permanently poisoning the singleton for all subsequent loader calls in the same webpack build.
- Impact: If `createMdsTransformer` ever throws (e.g., due to a future validation step in the constructor), the webpack loader becomes permanently broken for the rest of the build with no retry path. The old `.catch()` pattern allowed recovery.
- Fix: Restore the `.then().catch()` chain, or use a single `.then()` with explicit try/catch inside onFulfilled:
```typescript
initPromise = import('@mds/mds').then(
  (mds) => { transformer = createMdsTransformer(mds, options); },
).catch(
  (err: unknown) => { initPromise = null; throw err; },
);
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Inconsistent NODE_ENV guards across _setTransformerForTesting** - `packages/vite-plugin/src/index.ts:40-42`, `packages/rollup-plugin/src/index.ts:34-36`, `packages/webpack-loader/src/index.ts:77-80`
**Confidence**: 82%
- Problem: The webpack-loader's `_setTransformerForTesting` has a `NODE_ENV !== 'test'` guard that throws in non-test environments. The vite-plugin and rollup-plugin versions of the same function have no guard at all -- they silently accept calls in any environment. This inconsistency means the same testing helper has different safety guarantees depending on which package you use.
- Impact: If `_setTransformerForTesting` is accidentally called in production code (via a bad import or bundling error), the webpack-loader version will throw while the vite/rollup versions will silently corrupt the module-level state. All three should behave the same way.
- Fix: Either add the same guard to vite-plugin and rollup-plugin:
```typescript
export function _setTransformerForTesting(t: ReturnType<typeof createMdsTransformer> | null): void {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  _testTransformer = t;
}
```
Or accept the inconsistency as intentional since vite/rollup use a scoped `_testTransformer` variable (per-plugin-instance) while webpack uses a module-level singleton, making the webpack case more dangerous.

## Pre-existing Issues (Not Blocking)

No pre-existing regression issues found.

## Suggestions (Lower Confidence)

- **_resetForTesting guard tightening may break development-mode usage** - `packages/webpack-loader/src/index.ts:64` (Confidence: 65%) -- The guard changed from `NODE_ENV === 'production'` (blocks production) to `NODE_ENV !== 'test'` (blocks everything except test). If anyone called `_resetForTesting` during development (e.g., in a dev script or REPL), it will now throw. This is pre-release software with zero users, so likely non-issue.

- **webpack-loader does not call shouldTransform -- relies entirely on webpack config** - `packages/webpack-loader/src/index.ts:45` (Confidence: 62%) -- Unlike vite/rollup plugins which call `shouldTransform(cleanId(id))`, the webpack loader calls `t.transform(this.resourcePath)` directly without checking if the file should be transformed. This is by design (webpack `test` regex handles filtering), but means `.md` files with `type: mds` frontmatter cannot be loaded via the webpack-loader unless the user explicitly adds `.md` to the webpack config rule. The vite and rollup plugins handle this transparently.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Cross-Cycle Awareness**: Prior cycle 2 resolved 23/27 issues. The `cleanId` removal from `shouldTransform`/`transform` and `isMdsError` mock removal were flagged and resolved in that cycle -- verified here as correctly completed with all callers properly updated. No regression from those resolutions.

**Regression Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

One HIGH blocking issue: the `.then(a,b)` refactor in webpack-loader's `ensureTransformer` silently removes error recovery for `createMdsTransformer` failures. While the practical risk is low (the factory is unlikely to throw today), this is a loss of defensive safety that the original code intentionally provided. The fix is minimal -- restore the `.catch()` chain.

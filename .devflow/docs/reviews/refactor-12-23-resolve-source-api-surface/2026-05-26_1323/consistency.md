# Consistency Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23

## Issues in Your Changes (BLOCKING)

### HIGH

**`_setTransformerForTesting` signature diverges across bundler plugins** - `packages/webpack-loader/src/index.ts:73`
**Confidence**: 90%
- Problem: The webpack-loader's `_setTransformerForTesting` now has a different signature from the vite-plugin and rollup-plugin versions. Webpack's version is `async (t: Transformer): Promise<void>` and does not accept `null`, while vite-plugin (line 42) and rollup-plugin (line 36) both use `(t: Transformer | null): void`. The three plugins form a cohesive bundler integration surface and their test helpers previously shared the same signature shape.
- Impact: Consumers and test authors familiar with the vite/rollup pattern will be surprised that the webpack variant is async and does not accept `null` for teardown. This is a pattern violation across sibling packages that serve the same purpose.
- Fix: Either (a) align webpack-loader to accept `Transformer | null` with `null` resetting state (matching vite/rollup), or (b) document the intentional divergence with a comment explaining why the webpack pattern differs. The async nature is justified by the LazyInit pre-resolve, but the `null` omission is not. Suggested minimal fix:

```typescript
export async function _setTransformerForTesting(t: Transformer | null): Promise<void> {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  if (t === null) {
    lazy?.reset();
    lazy = null;
    return;
  }
  lazy = new LazyInit(async () => t);
  await lazy.get();
}
```

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

_No issues found._

## Suggestions (Lower Confidence)

- **Vite/Rollup plugins do not use LazyInit for transformer init** - `packages/vite-plugin/src/index.ts:62`, `packages/rollup-plugin/src/index.ts:55` (Confidence: 65%) -- The webpack-loader now uses `LazyInit` for its transformer singleton, but vite-plugin and rollup-plugin still use a manual `let transformer: Transformer | null` pattern with eager init in `buildStart`. This is an acceptable divergence since vite/rollup have a lifecycle hook (`buildStart`) that guarantees single-call initialization, whereas webpack-loader is a stateless per-file function needing lazy dedup. Noting for awareness only.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The branch demonstrates strong consistency overall. The `&Path` to `&str` migration is applied uniformly across all four public functions in `lib.rs` with a shared `path_to_str` helper. The `resolve_base_dir` boundary conversion is clean. The `Transformer` type alias was introduced consistently in all three bundler plugins (vite, rollup, webpack). The `LazyInit` extraction centralizes the dedup/retry pattern previously duplicated between `transform.ts` and `webpack-loader`. Test assertion patterns (`expect_err` vs `is_err`) follow the existing mixed style already present in the codebase. The single blocking issue is the `_setTransformerForTesting` signature divergence across sibling bundler packages.

**Cross-Cycle Note**: Prior cycle 1 resolved the `Transformer` type alias consistency for vite + rollup plugins. This cycle confirms those fixes are in place and the alias is now used consistently across all three plugin packages.

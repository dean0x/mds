# TypeScript Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26
**Cycle**: 2 (incremental after Cycle 1 resolved 6/6 issues)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none -- no items met the 60% threshold)

## Observations (Not Issues)

The following are positive observations about the TypeScript quality of this changeset:

1. **LazyInit<T> generic design is well-typed.** The `resolved` boolean flag avoids the classic footgun where `T = void | null` would be indistinguishable from "uninitialized" if using `instance !== undefined`. The single `as T` assertion on line 20 (`lazy-init.ts`) is justified: it is guarded by the `this.resolved` flag, making the narrowing safe.

2. **No `any` types anywhere.** All error parameters use `unknown` (e.g., `err: unknown` in the rejection handler at `lazy-init.ts:31` and throughout catch blocks). Strict mode is enabled (`strict: true`, `noUncheckedIndexedAccess: true` in `tsconfig.base.json`).

3. **Consistent `Transformer` type alias.** All three consumers (vite-plugin, rollup-plugin, webpack-loader) now use `type Transformer = ReturnType<typeof createMdsTransformer>` instead of repeating the verbose `ReturnType<...>` inline. This was a Cycle 1 fix and is verified correct.

4. **`_setTransformerForTesting` async signature (webpack-loader).** The change from sync to `async` is correct -- it pre-resolves the `LazyInit` so subsequent `get()` calls return synchronously. The `await lazy.get()` call ensures the promise settles before the function returns, preventing test races. This was a Cycle 1 fix (fire-and-forget) and is verified correct.

5. **Generation counter for TOCTOU safety.** The `++this.generation` pattern in both `get()` and `reset()` is a clean approach to invalidating stale in-flight resolutions. This was a Cycle 1 fix and is verified correct.

6. **Structural typing for external APIs.** The `LoaderContext`, `PluginContext`, `RollupPlugin`, and `VitePlugin` interfaces use structural subsets rather than importing full framework types. This is idiomatic TypeScript -- structural typing enforces compatibility without coupling to upstream type churn.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**TypeScript Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This is a clean TypeScript changeset. The `LazyInit<T>` generic class is well-designed with proper type parameterization, the `resolved` flag handles `void`/`null` edge cases correctly, and the single type assertion is justified by a runtime guard. All six issues from Cycle 1 have been verified as properly resolved: the TOCTOU race fix (generation counter), the fire-and-forget fix (`async _setTransformerForTesting`), and the `Transformer` type alias consistency are all in place. No `any` types, no escape hatches (`@ts-ignore`, `@ts-expect-error`, non-null assertions), and strict mode is enforced project-wide. The only reason this is not 10/10 is that test files are `.mjs` rather than `.ts`, which means the tests themselves do not benefit from type checking -- but that is a pre-existing project convention, not something introduced by this PR.

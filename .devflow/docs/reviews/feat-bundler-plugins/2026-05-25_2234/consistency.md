# Consistency Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T22:34

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent `_setTransformerForTesting` guard pattern across plugins** - `packages/rollup-plugin/src/index.ts:34`, `packages/vite-plugin/src/index.ts:40`, `packages/webpack-loader/src/index.ts:77`
**Confidence**: 92%
- Problem: The webpack-loader's `_setTransformerForTesting` includes a `NODE_ENV !== 'test'` runtime guard that throws if called outside test environments (lines 78-79). The rollup-plugin and vite-plugin versions have no such guard -- they silently accept any call. The webpack-loader also has this same guard on `_resetForTesting`. This creates an inconsistent safety model: webpack prevents accidental production use, but rollup and vite do not.
- Fix: Either add the `NODE_ENV` guard to all three plugins, or remove it from webpack-loader. Given that all three serve the same purpose and the guard is a reasonable safety net, adding it consistently is recommended:
```typescript
// In rollup-plugin/src/index.ts and vite-plugin/src/index.ts:
export function _setTransformerForTesting(t: ReturnType<typeof createMdsTransformer> | null): void {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  _testTransformer = t;
}
```

**Inconsistent `_setTransformerForTesting` signature: nullable vs non-nullable parameter** - `packages/webpack-loader/src/index.ts:77` vs `packages/rollup-plugin/src/index.ts:34`, `packages/vite-plugin/src/index.ts:40`
**Confidence**: 90%
- Problem: The rollup-plugin and vite-plugin accept `| null` in the parameter type (allowing callers to reset the test transformer to null), while webpack-loader does not accept null. In test code, rollup and vite tests call `_setTransformerForTesting(null)` in their `finally` blocks to clean up, but the webpack-loader test never calls it with null (it relies on `_resetForTesting` instead). This means the three packages have divergent cleanup patterns for the same conceptual operation.
- Fix: Align the webpack-loader signature to also accept `| null`, or align rollup/vite to use a separate `_resetForTesting` export. The simplest alignment is to make all three accept `| null`:
```typescript
// In webpack-loader/src/index.ts:
export function _setTransformerForTesting(t: ReturnType<typeof createMdsTransformer> | null): void {
```

### MEDIUM

**Inconsistent JSDoc on `_setTransformerForTesting` across plugins** - `packages/rollup-plugin/src/index.ts:27-31`, `packages/vite-plugin/src/index.ts:33-37`, `packages/webpack-loader/src/index.ts:71-75`
**Confidence**: 85%
- Problem: The rollup-plugin and vite-plugin JSDoc says "FOR TESTING ONLY -- does not affect production builds." while the webpack-loader JSDoc says "FOR TESTING ONLY -- throws in production environments." and the `_resetForTesting` says "throws unless NODE_ENV=test." The wording divergence reflects the guard behavior inconsistency above but also introduces documentation drift even if guards are not aligned.
- Fix: Once the guard behavior is aligned (see HIGH issue above), use the same JSDoc wording across all three. If guards are added everywhere, use "FOR TESTING ONLY -- throws unless NODE_ENV=test." consistently.

**Structural type rationale comment uses different phrasing in webpack-loader** - `packages/webpack-loader/src/index.ts:4-7`
**Confidence**: 82%
- Problem: Rollup and Vite plugins use the pattern "Structural subset of {X}. We intentionally keep narrow interfaces rather than importing..." with a numbered list of reasons and a closing sentence about TypeScript catching drift. The webpack-loader uses a different structure: "Hand-rolled rather than `import type`..." without the numbered list or closing sentence. While the intent is the same and the PR description mentions "same rationale comment across vite/rollup", the webpack-loader was not updated to match.
- Fix: Align the webpack-loader comment to follow the same structure. For example:
```typescript
// Structural subset of webpack's LoaderContext. We intentionally keep a narrow
// interface rather than importing `LoaderContext` from 'webpack' because:
//   1. webpack uses a CJS `export =` shape that is awkward to import in a
//      pure-ESM package and the full type is a large intersection of ~10 interfaces.
//   2. The structural subset below captures exactly what this loader uses.
// If webpack's LoaderContext API surface changes in a breaking way, TypeScript
// will catch it at build time via structural checking.
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing `_resetForTesting` in rollup-plugin and vite-plugin** - `packages/rollup-plugin/src/index.ts`, `packages/vite-plugin/src/index.ts` (Confidence: 65%) -- The webpack-loader exports both `_resetForTesting` (clears singleton state) and `_setTransformerForTesting` (injects mock). Rollup and vite plugins only export `_setTransformerForTesting` and rely on passing `null` to reset. While this works due to different initialization patterns (closure vs module singleton), having two different reset idioms across the suite is a minor consistency gap.

- **Test mock warning strings use different naming per plugin** - `packages/rollup-plugin/__test__/plugin.spec.mjs:107`, `packages/vite-plugin/__test__/plugin.spec.mjs:152`, `packages/webpack-loader/__test__/loader.spec.mjs:128` (Confidence: 62%) -- Rollup tests use `'rollup warning one'`, Vite tests use `'compiler warning one'`, and webpack tests use `'first warning'`. While test data names are not production code, using plugin-specific prefixes in rollup but generic names in vite and webpack is a minor readability inconsistency. Consider using a consistent naming scheme (either all generic or all plugin-prefixed).

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The three bundler plugins share a clear architectural pattern (structural types, transformer injection, warning/dependency forwarding) and the core transform logic is well-factored into bundler-utils. However, the `_setTransformerForTesting` utility has divergent guard behavior and signatures between webpack-loader (guarded, non-nullable) and vite/rollup (unguarded, nullable). The structural type rationale comments also diverge in style. These are the primary consistency gaps to resolve before merge.

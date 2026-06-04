# Architecture Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T22:34
**Cycle**: 3 (prior cycles resolved 23/27 issues; 3 FP, 1 deferred)

## Issues in Your Changes (BLOCKING)

### HIGH

**Module-level mutable singleton in vite-plugin and rollup-plugin (`_testTransformer`)** - `packages/vite-plugin/src/index.ts:39`, `packages/rollup-plugin/src/index.ts:33`
**Confidence**: 85%
- Problem: `_testTransformer` is a module-level mutable variable shared across all callers. In vite-plugin and rollup-plugin it lacks the `NODE_ENV` guard that webpack-loader has (lines 77-79). Any consumer -- not just tests -- can call `_setTransformerForTesting()` to replace the transformer. While each JSDoc says "FOR TESTING ONLY", webpack-loader enforces this with a runtime guard (`if (process.env['NODE_ENV'] !== 'test') throw ...`) whereas vite-plugin and rollup-plugin do not. This inconsistency means the API surface is wider than intended for two of the three plugins.
- Impact: Production code could accidentally (or intentionally) call `_setTransformerForTesting` in vite/rollup plugins without getting an error, silently replacing the real compiler.
- Fix: Add the same `NODE_ENV` guard used in webpack-loader:
```typescript
export function _setTransformerForTesting(t: ReturnType<typeof createMdsTransformer> | null): void {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  _testTransformer = t;
}
```

### MEDIUM

**Duplicate structural type definitions across plugins** - `packages/vite-plugin/src/index.ts:13-31`, `packages/rollup-plugin/src/index.ts:11-25`, `packages/webpack-loader/src/index.ts:8-14`
**Confidence**: 82%
- Problem: Each plugin defines its own structural interfaces for the bundler context (`PluginTransformContext`, `PluginContext`, `LoaderContext`) and plugin shape (`VitePlugin`, `RollupPlugin`). While this is intentionally documented (avoiding heavy type imports), the three `PluginContext`-like interfaces share overlapping members (`warn`, `addWatchFile`) with no shared abstraction. If the bundler-utils `TransformResult` shape or the error-handling contract changes, each plugin must update independently with no type-system help.
- Impact: Low risk today (the interfaces are small), but as the plugin API grows this duplication will diverge silently. Prior cycle documented this as ARCH-2 (resolved by adding comments), so the design decision is deliberate -- but the duplication cost grows with each new hook.
- Fix: Consider extracting a shared `BundlerContext` interface into `@mds/bundler-utils/types.ts` for the common subset (`warn`, `addWatchFile`). Each plugin can extend it with bundler-specific members. This keeps the "no heavy imports" goal while centralizing the shared contract:
```typescript
// In @mds/bundler-utils/types.ts
export interface BundlerContext {
  warn(msg: string): void;
  addWatchFile(id: string): void;
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`shouldTransform` contract relies on caller discipline, not the type system** - `packages/bundler-utils/src/frontmatter.ts:32-34`
**Confidence**: 82%
- Problem: The comment on line 33 says "id is expected to be pre-cleaned by the caller" after removing the internal `cleanId()` call (PERF-1 from prior cycle). This creates a precondition that is enforced only by convention. If a new plugin (e.g., esbuild) calls `shouldTransform` with a raw id containing `?query`, it will silently fail to match `.mds` or `.md` extensions and return `false`.
- Impact: The `createMdsTransformer` factory returns `shouldTransform` directly (`shouldTransform: checkTransform` on line 67 of transform.ts), so any consumer of the transformer gets the uncleaned version. Today all three plugins clean the id before calling, but the contract is implicit.
- Fix: Two options, in order of preference:
  1. **Type-level safety**: Create a branded type `CleanId` that `cleanId()` returns, and make `shouldTransform` accept only `CleanId`. This makes the precondition compile-time enforced.
  2. **Defensive fallback**: Re-add a cheap fast-path check (`id.includes('?') || id.includes('#')`) that calls `cleanId` only when needed, keeping the performance win for the common case.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Webpack loader uses module-level singleton while Vite/Rollup use per-plugin-instance closures** - `packages/webpack-loader/src/index.ts:16-17` vs `packages/vite-plugin/src/index.ts:51`
**Confidence**: 85%
- Problem: The webpack loader stores `transformer` and `initPromise` at module scope (lines 16-17), making it a true singleton -- one transformer per Node.js process. Vite and Rollup plugins create a `transformer` per `mdsPlugin()` call via closure. This is architecturally inconsistent: webpack uses singleton because loaders are stateless functions (documented on line 22-27), while Vite/Rollup use factory closures because plugins are objects. The comment at line 25-26 acknowledges this: "Multiple compiler instances with different options are not supported by a module-level singleton."
- Impact: This is a known design constraint, not a bug. It is correctly documented. However, the `_resetForTesting()` function (webpack-only) exists solely to work around the singleton, and the architectural asymmetry means webpack-loader has a different failure mode (shared state across test runs) than the other plugins.
- Note: Prior cycle deferred webpack init/retry duplication to a separate PR. This singleton pattern is the underlying reason for that duplication.

## Suggestions (Lower Confidence)

- **HMR only handles `.mds` extension, not `.md` with frontmatter** - `packages/vite-plugin/src/index.ts:101-105` (Confidence: 70%) -- `handleHotUpdate` checks `isMdsExtension(clean)` but does not check for `.md` files with `type: mds` frontmatter. Editing such a `.md` file would not trigger a full-reload. This may be intentional for v0.1.0 (HMR for `.md` requires async frontmatter detection which `handleHotUpdate` doesn't support well synchronously), but it creates an asymmetry between build-time and dev-time behavior.

- **`ensureInit` in transform.ts duplicates init-once pattern from webpack loader** - `packages/bundler-utils/src/transform.ts:55-64` vs `packages/webpack-loader/src/index.ts:19-38` (Confidence: 65%) -- Both implement the same promise-caching init-once pattern with rejection-reset. The bundler-utils version guards `mds.init()` while the webpack version guards `import('@mds/mds')`. Extracting a generic `initOnce(factory)` utility could eliminate this duplication.

- **No `.md` frontmatter detection in Rollup watch mode** - `packages/rollup-plugin/src/index.ts` (Confidence: 62%) -- Rollup's `addWatchFile` is called for dependencies but there is no `watchChange` hook to handle `.md` files with frontmatter that get edited. Unlike Vite which has `handleHotUpdate`, Rollup relies on the module graph -- if a `.md` file is not yet in the graph, edits to it won't trigger a rebuild.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

The bundler plugin architecture is well-designed. The layered approach -- `@mds/bundler-utils` as shared transform layer with three thin plugin adapters -- follows clean separation of concerns. Key architectural strengths:

1. **Correct dependency direction**: plugins depend on bundler-utils, bundler-utils depends on nothing except the `MdsApi` interface (DIP satisfied).
2. **Structural typing**: avoids pulling in heavy bundler type dependencies while maintaining type safety.
3. **Init-once with rejection reset**: the promise-caching pattern correctly handles transient failures.
4. **Clean public API surface**: `createMdsTransformer`, `formatMdsError`, `cleanId`, `shouldTransform` -- each has a single responsibility.

The one blocking condition is the inconsistent `NODE_ENV` guard on `_setTransformerForTesting` between webpack-loader (guarded) and vite/rollup plugins (unguarded). This is a straightforward fix.

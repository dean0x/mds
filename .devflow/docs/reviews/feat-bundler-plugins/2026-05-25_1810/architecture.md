# Architecture Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10
**Diff**: f58fa41f...HEAD (7 commits)
**Prior Resolutions**: Cycle 1 resolved 18/20 issues; 2 documented (webpack singleton options drift intentional, MdsApi narrower interface intentional).

## Issues in Your Changes (BLOCKING)

### HIGH

**Webpack loader duplicates init/retry logic already encapsulated by createMdsTransformer** - `packages/webpack-loader/src/index.ts:15-38`
**Confidence**: 85%
- Problem: The webpack loader maintains its own module-level `transformer` singleton and `initPromise` with poisoned-promise recovery logic (lines 15-38). Meanwhile, `createMdsTransformer` in `bundler-utils/src/transform.ts:30-42` already encapsulates `ensureInit()` with the same poisoned-promise pattern. The Vite and Rollup plugins both delegate initialization to `buildStart()` — one line of `await import('@mds/mds')` plus `createMdsTransformer(mds, options)` — and let the transformer manage its own init state. The webpack loader reimplements lazy init at the module level because loaders lack a `buildStart` hook, but the result is two layers of promise deduplication (one in `ensureTransformer`, one inside the transformer's `ensureInit`). This is a DIP/SRP concern: the loader knows too much about init orchestration that the shared utility already handles.
- Impact: If the poisoned-promise recovery pattern in `createMdsTransformer` changes, the webpack loader's parallel implementation diverges silently. The dual-layer init deduplication obscures which layer is actually responsible for retry.
- Fix: Extract a `createLazyMdsTransformer(options)` factory into `bundler-utils` that handles the dynamic `import('@mds/mds')` + `createMdsTransformer` pattern. This gives the webpack loader a single call site and eliminates the duplicated init logic:
```typescript
// bundler-utils/src/lazy.ts
export function createLazyMdsTransformer(options?: MdsPluginOptions) {
  let transformer: ReturnType<typeof createMdsTransformer> | null = null;
  let importPromise: Promise<void> | null = null;

  async function ensure(): Promise<ReturnType<typeof createMdsTransformer>> {
    if (transformer !== null) return transformer;
    if (importPromise === null) {
      importPromise = import('@mds/mds')
        .then((mds) => { transformer = createMdsTransformer(mds, options); })
        .catch((err: unknown) => { importPromise = null; throw err; });
    }
    await importPromise;
    return transformer!;
  }

  return { ensure };
}
```
Then `webpack-loader/src/index.ts` simplifies to:
```typescript
import { createLazyMdsTransformer } from '@mds/bundler-utils';
const lazy = createLazyMdsTransformer(/* options from first call */);
// in mdsLoader:
const t = await lazy.ensure();
```
And Vite/Rollup `buildStart` becomes `transformer = await lazy.ensure()`.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Vite and Rollup plugins define their own interface types instead of importing from bundler frameworks** - `packages/vite-plugin/src/index.ts:4-22`, `packages/rollup-plugin/src/index.ts:4-18`
**Confidence**: 82%
- Problem: Both plugins hand-roll `VitePlugin`, `PluginTransformContext`, `RollupPlugin`, and `PluginContext` interface types that mirror the real Vite/Rollup plugin interfaces. While this avoids a hard dependency on the bundler at import time (the bundler is a peerDependency), it creates a maintenance coupling risk: if Vite 7 or Rollup 5 changes their plugin hook signatures, these hand-rolled types will silently go stale. The bundlers already export their types as dev/peer dependencies and both packages already list `vite` and `rollup` in `devDependencies`.
- Impact: Type drift between the hand-rolled interfaces and the real bundler types. For example, the `handleHotUpdate` ctx type in Vite has grown more complex across Vite versions; the hand-rolled version captures only `file` and `server.ws.send`.
- Fix: Import the real types from the bundler packages (available via devDependencies) and use `import type` to avoid runtime dependency:
```typescript
// vite-plugin/src/index.ts
import type { Plugin as VitePlugin } from 'vite';
// rollup-plugin/src/index.ts
import type { Plugin as RollupPlugin } from 'rollup';
```

**Webpack loader's `LoaderContext` interface diverges from webpack's real type** - `packages/webpack-loader/src/index.ts:4-10`
**Confidence**: 80%
- Problem: Same pattern as above. The `LoaderContext` interface is hand-rolled. Webpack exports `LoaderContext` from `webpack` that could be used via `import type`.
- Impact: If webpack 6 changes the loader API, this hand-rolled type won't catch it at compile time.
- Fix: Use `import type { LoaderContext } from 'webpack'` and parameterize it appropriately.

### LOW

**Inconsistent initialization strategy across bundler plugins** - `packages/vite-plugin/src/index.ts:31-33`, `packages/rollup-plugin/src/index.ts:26-28`, `packages/webpack-loader/src/index.ts:15-38`
**Confidence**: 80%
- Problem: Vite and Rollup use per-plugin-instance `transformer` (closure-scoped), while webpack uses a module-level singleton. This is a documented intentional choice (webpack loaders are stateless functions), but the asymmetry means the three plugins cannot share a unified initialization pattern. This is an acceptable consequence of webpack's loader model, but worth noting as an architectural limitation.
- Impact: Low. The asymmetry is well-documented in the code comments (lines 18-23 of webpack-loader). Testing the webpack loader requires an exported `_resetForTesting` function that doesn't exist in the other plugins.
- Fix: No immediate action required. The `createLazyMdsTransformer` suggestion from the BLOCKING section above would unify this. Document the architectural rationale in a README or ADR if not already covered.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **HMR strategy is coarse-grained** - `packages/vite-plugin/src/index.ts:65-72` (Confidence: 65%) -- `handleHotUpdate` always triggers a full page reload for any `.mds` file change. For large apps, fine-grained HMR (invalidating only the importing module) could improve DX. This is acceptable for v0.1.0 but worth noting for future improvement.

- **`mds.d.ts` ambient module declaration is narrow** - `packages/bundler-utils/mds.d.ts:1-5` (Confidence: 70%) -- The ambient module declaration only covers `*.mds` files but `shouldTransform` also handles `.md` files with `type: mds` frontmatter. Users importing `.md` files would not get type coverage from this declaration.

- **No source map support** - `packages/vite-plugin/src/index.ts:50`, `packages/rollup-plugin/src/index.ts:45` (Confidence: 65%) -- All three plugins return `map: null`. For debugging compiled MDS content in browser devtools, source maps would improve developer experience. Fine for v0.1.0.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The four-package decomposition (`bundler-utils` as shared core, three thin adapter packages) follows the Hexagonal Architecture / Ports-and-Adapters pattern well. The shared transformer factory (`createMdsTransformer`) is a deep module with a clean interface. The `MdsApi` structural typing approach correctly applies ISP by exposing only the subset of `@mds/mds` that bundler plugins need.

The primary architectural concern is the duplicated lazy-init logic in the webpack loader. This is not blocking given the code comments and documented intentional choice from Cycle 1, but it should be consolidated into `bundler-utils` before the plugin count grows (e.g., esbuild, rspack, farm, turbopack). The hand-rolled bundler type interfaces are a moderate concern -- using real types from peer dependencies is standard practice for bundler plugins and would catch API drift at compile time.

The `escapeForJs` rewrite from O(n^2) manual loop to regex-based replacement, the poisoned-promise recovery, and the `MdsApi` interface narrowing (removing `isMdsError`) from Cycle 1 resolutions are all architecturally sound improvements.

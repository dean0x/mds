# Architecture Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25T17:35

## Issues in Your Changes (BLOCKING)

### HIGH

**Webpack loader uses module-level singleton state — breaks multi-config isolation** - `packages/webpack-loader/src/index.ts:12-27`
**Confidence**: 90%
- Problem: The `transformer` and `initPromise` variables are module-level singletons. In Webpack, a single Node process can run multiple compiler instances (e.g., separate client and server configs in `webpack-multi-compiler`). The first config's `getOptions()` call captures `options` into the singleton, and all subsequent configs silently inherit those options — even if they passed different `vars`. This is a correctness bug rooted in an architectural choice. Contrast with the Vite and Rollup plugins, which correctly scope `transformer` inside the plugin factory closure.
- Fix: Move the `transformer` and `initPromise` state inside the loader function's scope using a `WeakMap` keyed on the compiler instance, or adopt the same closure-scoped pattern used by the Vite/Rollup plugins. Since Webpack loaders are stateless functions, the cleanest approach is a `Map<string, transformer>` keyed on a serialized options hash:

```typescript
const transformerCache = new Map<string, {
  transformer: ReturnType<typeof createMdsTransformer>;
  promise: Promise<void>;
}>();

function getOptionsKey(options: MdsPluginOptions): string {
  return JSON.stringify(options.vars ?? {});
}

async function ensureTransformer(options: MdsPluginOptions): Promise<...> {
  const key = getOptionsKey(options);
  let entry = transformerCache.get(key);
  if (entry === undefined) {
    const promise = import('@mds/mds').then((mds) => {
      entry!.transformer = createMdsTransformer(mds, options);
    });
    entry = { transformer: null!, promise };
    transformerCache.set(key, entry);
  }
  await entry.promise;
  return entry.transformer;
}
```

Alternatively, document that the singleton is intentional and the first `getOptions()` call wins. But that would be surprising behavior for multi-config users.

### MEDIUM

**MdsApi interface in bundler-utils diverges from the actual @mds/mds module API** - `packages/bundler-utils/src/types.ts:1-5`
**Confidence**: 85%
- Problem: The `MdsApi` interface defines `init(): Promise<void>`, `compileFile(...)`, and `isMdsError(...)` as instance-like methods. But `@mds/mds` exports these as top-level module functions (`export function init(...)`, `export function compileFile(...)`, `export function isMdsError(...)`). This works today because a dynamic `import()` of a module returns an object with those functions as properties. However, this is a structural mismatch — `MdsApi` models a module namespace as if it were an object with methods, which means:
  1. The interface does not reflect the actual API contract (e.g., `init` in `@mds/mds` accepts optional `InitOptions`, but `MdsApi.init()` accepts nothing).
  2. Adding a new backend (e.g., a browser backend that uses `MdsBaseBackend` instead of `MdsNodeBackend`) would require a different interface, but `MdsApi` is locked to the file-ops shape.
  3. TypeScript cannot verify that `import('@mds/mds')` actually satisfies `MdsApi` — the type safety is manually maintained.
- Fix: Either (a) export a proper `MdsApi`-satisfying interface from `@mds/mds` itself (so the contract is enforced at the source), or (b) add a type assertion at the `import()` call site with a comment explaining the structural typing relationship:

```typescript
// In each plugin's buildStart:
const mds: MdsApi = await import('@mds/mds') as unknown as MdsApi;
```

This at least makes the cast explicit rather than relying on duck typing.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`_resetForTesting` export exposes internal state mutation in production API surface** - `packages/webpack-loader/src/index.ts:52-55`
**Confidence**: 82%
- Problem: The `_resetForTesting` function is exported as part of the public module surface and mutates module-level singleton state. While it follows the `_` convention used by `@mds/mds` itself, it is included in the `dist/` output and the package `exports` map, making it callable by any consumer. The Vite and Rollup plugins do not need this because their state is closure-scoped (a direct benefit of that architectural choice).
- Fix: If the singleton pattern is kept, consider using `node:test`'s `mock.module()` or a conditional export gated behind `NODE_ENV === 'test'`. Alternatively, resolving the singleton issue (see BLOCKING issue above) eliminates the need for this function entirely.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Committed dist/ artifacts may drift from source** - `packages/bundler-utils/dist/`, `packages/vite-plugin/dist/`, etc. (Confidence: 70%) — All four packages commit their compiled `dist/` directories. If a contributor modifies a `.ts` source but forgets to rebuild, the committed JS will be stale. Consider adding a CI check that verifies `dist/` is up to date, or exclude `dist/` from version control and build in CI.

- **Vite plugin re-imports `cleanId` and `isMdsExtension` separately from bundler-utils** - `packages/vite-plugin/src/index.ts:2` (Confidence: 65%) — The transformer already has `shouldTransform` which internally calls `cleanId` and `isMdsExtension`. The Vite plugin imports `cleanId` and `isMdsExtension` directly for use in `handleHotUpdate`, bypassing the transformer abstraction. This is likely fine for a thin adapter, but it means `handleHotUpdate` duplicates knowledge about what constitutes an MDS file rather than delegating to the shared utility's `shouldTransform`.

- **No shared plugin contract or base class across bundler adapters** - (Confidence: 62%) — The Vite, Rollup, and Webpack plugins each define their own inline interfaces for their respective plugin contexts (`PluginTransformContext`, `PluginContext`, `LoaderContext`). These are appropriate for thin adapters, but as the plugin surface grows (e.g., adding `resolveId` or `load` hooks), there may be value in a shared adapter pattern. Not blocking for the current scope.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The overall architecture is well-designed: a shared `bundler-utils` core with thin, bundler-specific adapters is exactly the right decomposition. The `createMdsTransformer` factory cleanly encapsulates init-once semantics, escaping, and code generation. The Vite and Rollup plugins correctly scope their state inside the factory closure.

The primary concern is the Webpack loader's module-level singleton, which breaks isolation in multi-compiler setups and necessitates the `_resetForTesting` escape hatch. Aligning it with the closure-scoped pattern used by the other two plugins would resolve both the correctness issue and the testing concern. The secondary concern is the implicit structural typing between `MdsApi` and the `@mds/mds` module namespace, which works but is not compiler-verified.

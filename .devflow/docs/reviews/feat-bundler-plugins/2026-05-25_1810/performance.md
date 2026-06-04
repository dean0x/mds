# Performance Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10
**Cycle**: 2 (incremental — see Prior Resolutions below)

**Prior Resolutions**: Cycle 1 fixed 18 of 20 issues including escapeForJs O(n^2) rewrite and poisoned promise fixes. This cycle reviews only new/remaining issues.

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Redundant `cleanId` calls — triple invocation per transform** — `packages/vite-plugin/src/index.ts:38`, `packages/rollup-plugin/src/index.ts:33`, `packages/bundler-utils/src/transform.ts:49`
**Confidence**: 85%
- Problem: In the vite-plugin and rollup-plugin, the `transform` hook calls `cleanId(id)` at line 38/33, then passes the clean id to `transformer.shouldTransform(clean)`. Inside `shouldTransform` (frontmatter.ts:31), `cleanId(id)` is called again. Then `transformer.transform(clean)` is called, which internally calls `cleanId(id)` a third time (transform.ts:49). Each file processed runs `cleanId` three times on the same string. The function is cheap (indexOf + slice), so this is not a hotpath bottleneck, but it signals a confused API boundary — callers should not need to pre-clean when the internal functions also clean.
- Fix: Remove the internal `cleanId` call from `createMdsTransformer.transform()` at transform.ts:49, since both callers (vite-plugin and rollup-plugin) already pass a cleaned id. The webpack-loader passes `this.resourcePath` which never has query/hash suffixes, so it is also safe. This eliminates one redundant call and clarifies the contract: callers are responsible for cleaning.

```typescript
// transform.ts — remove redundant cleanId inside transform()
async transform(id: string): Promise<TransformResult> {
  await ensureInit();
  // id is trusted — sourced from the bundler's module resolution pipeline
  const result = await mds.compileFile(
    id,
    options?.vars !== undefined ? { vars: options.vars } : undefined,
  );
  // ...
}
```

Alternatively, `shouldTransform` (frontmatter.ts:30) also calls `cleanId` internally. If `transform` stops cleaning, then `shouldTransform` should too — and the contract becomes "callers always clean first." Either convention is fine; pick one and enforce it consistently.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Vite `handleHotUpdate` triggers full-reload for every `.mds` file change** — `packages/vite-plugin/src/index.ts:68`
**Confidence**: 82%
- Problem: Every `.mds` file change sends `{ type: 'full-reload' }` to the browser, which reloads the entire page. For projects with frequent `.mds` edits during development (e.g., prompt engineering workflows), this defeats Vite's HMR advantage and creates a poor DX. Vite's `handleHotUpdate` can return module-level updates instead.
- Fix: This is acceptable as a first-pass implementation since `.mds` imports produce string exports (not React components or stateful modules), making targeted HMR complex. However, the code should include a comment explaining why full-reload was chosen and that targeted HMR is a future optimization. Consider accepting `self` and invalidating only the importing modules:

```typescript
handleHotUpdate({ file, server, modules }) {
  const clean = cleanId(file);
  if (isMdsExtension(clean)) {
    // .mds files export plain strings — no component-level HMR.
    // Invalidate the changed module; Vite propagates to importers.
    return modules; // lets Vite handle granular updates
  }
  return undefined;
},
```

Note: This requires updating the `VitePlugin` interface to include `modules` in the `handleHotUpdate` context type.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`JSON.stringify` for metadata on every transform** — `packages/bundler-utils/src/transform.ts:57` (Confidence: 65%) — Each transform call serializes `{ warnings, dependencies }` via `JSON.stringify`. For files with many transitive dependencies, this re-serializes the same dependency paths repeatedly. Not a concern at current scale, but if dependency lists grow large, consider caching the serialized metadata when the dependency set is unchanged.

- **`new Error(warning)` allocation per warning in webpack-loader** — `packages/webpack-loader/src/index.ts:50` (Confidence: 60%) — Each warning string is wrapped in a `new Error()` just to satisfy `emitWarning`'s signature. For files that produce many warnings, this creates unnecessary Error objects with stack traces. This is a Webpack API constraint and not easily avoidable, but worth noting.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The escapeForJs O(n^2) issue from cycle 1 has been properly resolved — the new `String.replace` with a precompiled regex and lookup map is O(n) and idiomatic. The `ensureInit` poisoned-promise fix is also correct. The remaining performance concerns are minor: redundant `cleanId` calls (a clarity issue more than a hot-path bottleneck) and the full-reload HMR strategy (acceptable for v1 but worth revisiting). No critical or high-severity performance issues remain. The condition for approval is addressing the redundant `cleanId` to clarify the API contract.

# Consistency Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10
**Prior Cycle**: Cycle 1 fixed 18/20 issues (import ordering, package.json formatting, dist artifacts, JSDoc). This review covers only new/remaining findings.

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Redundant `cleanId` calls in vite-plugin and rollup-plugin -- inconsistent with webpack-loader** - `packages/vite-plugin/src/index.ts:38`, `packages/rollup-plugin/src/index.ts:33`
**Confidence**: 82%
- Problem: Both `vite-plugin` and `rollup-plugin` call `cleanId(id)` in their `transform` method, then pass the already-cleaned `clean` value to `transformer.transform(clean)`. However, `createMdsTransformer.transform()` in `bundler-utils/src/transform.ts:49` also calls `cleanId(id)` internally on whatever argument it receives. This means `cleanId` is called twice (idempotent, not a bug). The `webpack-loader` does NOT call `cleanId` before `t.transform(this.resourcePath)` (line 45), which is the correct pattern since `transform` cleans internally. The three plugins are inconsistent with each other on this.
- Fix: Either remove the `cleanId` call from `vite-plugin` and `rollup-plugin` (matching webpack-loader's pattern of letting `transform` handle it), or remove the internal `cleanId` call from `bundler-utils/src/transform.ts:49` and document that callers must clean before calling. The first option is simpler -- the shared transform layer should handle cleaning internally, and callers should pass the raw id. Note: the plugins also call `cleanId` before `shouldTransform`, which also calls `cleanId` internally (`frontmatter.ts:31`). Removing the external `cleanId` calls entirely would simplify both plugins to:
  ```typescript
  async transform(_, id) {
    if (transformer === null) return null;
    const should = await transformer.shouldTransform(id);
    if (!should) return null;
    const result = await transformer.transform(id);
    // ...
  }
  ```
  The only remaining use of `cleanId` in vite-plugin would be in `handleHotUpdate` (line 66), which does not go through `shouldTransform`/`transform` and legitimately needs the direct call.

**Poisoned-promise recovery pattern uses different styles** - `packages/bundler-utils/src/transform.ts:36-39`, `packages/webpack-loader/src/index.ts:24-32`
**Confidence**: 80%
- Problem: Both files implement the same poisoned-promise reset pattern but with different syntax:
  - `bundler-utils/src/transform.ts` uses the two-argument `.then(onFulfill, onReject)` form:
    ```typescript
    initPromise = mds.init().then(
      () => { initialized = true; },
      (err: unknown) => { initPromise = null; throw err; },
    );
    ```
  - `webpack-loader/src/index.ts` uses a `.then(...).catch(...)` chain:
    ```typescript
    initPromise = import('@mds/mds')
      .then((mds) => { transformer = createMdsTransformer(mds, options); })
      .catch((err: unknown) => { initPromise = null; throw err; });
    ```
  While semantically equivalent here, using two different promise error-handling patterns for the same logical operation (init with retry on failure) creates inconsistency within the bundler plugin family.
- Fix: Pick one style and apply consistently. The two-argument `.then(onFulfill, onReject)` form is slightly more precise (it catches only the promise it's attached to, not errors from the onFulfill callback), making it the better choice:
  ```typescript
  // webpack-loader/src/index.ts
  initPromise = import('@mds/mds').then(
    (mds) => { transformer = createMdsTransformer(mds, options); },
    (err: unknown) => { initPromise = null; throw err; },
  );
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Stale `isMdsError` in test mock after interface removal** - `packages/bundler-utils/__test__/transform.spec.mjs:27-29`
**Confidence**: 85%
- Problem: The `MdsApi` interface in `types.ts` had `isMdsError(err: unknown): boolean` removed in this branch (visible in the diff). However, the `createMockMds` helper in `transform.spec.mjs` still implements `isMdsError` as a method. While this is not a runtime error (extra properties are harmless in JavaScript), it creates drift between the test mock and the actual interface it simulates. Future developers may think `isMdsError` is still part of the contract.
- Fix: Remove lines 27-29 from the mock:
  ```javascript
  // Remove this block:
  isMdsError(err) {
    return err instanceof Error && typeof err.code === 'string' && err.code.startsWith('mds::');
  },
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Webpack-loader `ensureTransformer` duplicates init pattern from bundler-utils** - `packages/webpack-loader/src/index.ts:15-38` (Confidence: 70%) -- The webpack-loader maintains its own `transformer`/`initPromise` singleton with retry logic, while the other two plugins delegate initialization entirely to `buildStart` + `createMdsTransformer`. This is architecturally justified (webpack loaders lack a `buildStart` hook), but the duplicated init/retry logic could drift from the shared layer over time. Consider whether `bundler-utils` could expose a higher-level `createSingletonTransformer` that encapsulates the module-level singleton pattern.

- **Vite error construction style differs from Rollup approach** - `packages/vite-plugin/src/index.ts:55-60` vs `packages/rollup-plugin/src/index.ts:48-51` (Confidence: 65%) -- Vite uses `Object.assign(new Error(...), {...})` while Rollup uses a conditional `pos` variable. Both are correct for their respective bundler APIs (Vite throws, Rollup calls `this.error`), but the error assembly logic could be unified into a shared helper in `bundler-utils` to keep the per-plugin code minimal.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | - | 2 | - |
| Should Fix | - | - | 1 | - |
| Pre-existing | - | - | - | - |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Significant improvement from Cycle 1 (score 6/10 -> 8/10). All 4 HIGH-severity issues from the prior cycle were fixed: dist artifacts removed from tracking, import ordering corrected in webpack-loader, package.json reformatted to expanded style, and JSDoc added to all interface members.

The remaining findings are MEDIUM severity:
1. **Redundant `cleanId` calls** -- vite-plugin and rollup-plugin double-clean while webpack-loader does not. Harmless but inconsistent.
2. **Promise error handling style** -- `.then(ok, err)` vs `.then().catch()` for the same poisoned-promise pattern.
3. **Stale mock method** -- `isMdsError` lingering in test mock after interface change.

None of these are blocking. The four packages are well-structured and internally consistent with each other, following the shared-utility-layer architecture cleanly.

# Testing Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T22:34

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`_setTransformerForTesting` in rollup-plugin and vite-plugin lacks NODE_ENV guard** (2 occurrences) -- Confidence: 85%
- `packages/rollup-plugin/src/index.ts:34`, `packages/vite-plugin/src/index.ts:40`
- Problem: The webpack-loader's `_setTransformerForTesting` and `_resetForTesting` both guard with `if (process.env['NODE_ENV'] !== 'test') throw ...`, which prevents accidental use in production. The identical `_setTransformerForTesting` exports in rollup-plugin and vite-plugin have no such guard -- any code can call them at runtime and inject an arbitrary transformer. While rollup/vite plugins create per-instance closures (limiting blast radius compared to webpack's singleton), the exported function still mutates module-level `_testTransformer` state.
- Fix: Add the same `NODE_ENV` guard to both:
```typescript
export function _setTransformerForTesting(t: ReturnType<typeof createMdsTransformer> | null): void {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  _testTransformer = t;
}
```
Note: This is a consistency issue that also has a testing dimension -- test-only seams should be safely guarded in all packages, not just some.

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No dedicated unit tests for `formatMdsError` error branches** -- `packages/bundler-utils/src/errors.ts` -- Confidence: 82%
- Problem: `formatMdsError` has three code paths (MDS error with help/span, generic Error, non-Error thrown value). The errors.spec.mjs exists but was not changed in this branch. The plugin-level tests only exercise the generic Error path through integration (passing a nonexistent file). The MDS-specific error formatting with `help` and `span` fields is not covered by any test visible in the changed files.
- Fix: Outside this PR's scope. Consider adding direct unit tests for the `help` text appending and `span.line`/`span.column` extraction branches.

## Suggestions (Lower Confidence)

- **Concurrent `_setTransformerForTesting` tests could theoretically interfere across test files** - `packages/rollup-plugin/__test__/plugin.spec.mjs:112`, `packages/vite-plugin/__test__/plugin.spec.mjs:157` (Confidence: 65%) -- The `_testTransformer` variable is module-level, and the warning tests use `try/finally` for cleanup. If node:test ever runs tests from the same file concurrently (it does not by default within a `describe`), the module-level state could leak. The current `try/finally` pattern is correct for serial execution, but an `afterEach` reset would be more defensive.

- **Mock transformer in plugin tests lacks `shouldTransform` async fidelity** - `packages/rollup-plugin/__test__/plugin.spec.mjs:103`, `packages/vite-plugin/__test__/plugin.spec.mjs:148` (Confidence: 62%) -- The mock's `shouldTransform` returns a synchronous `true`, while the real transformer can return `boolean | Promise<boolean>`. This is fine for current tests but could mask issues if the production code path changes to depend on the async variant. Not blocking since the sync `true` is a valid return per the type signature.

- **No test for `_setTransformerForTesting` cleanup (reset to null) re-enabling real init** - `packages/rollup-plugin/__test__/plugin.spec.mjs:124`, `packages/vite-plugin/__test__/plugin.spec.mjs:169` (Confidence: 60%) -- The tests call `_setTransformerForTesting(null)` in the `finally` block but never assert that a subsequent `buildStart` call reverts to the real `@mds/mds` import path. The current `try/finally` pattern is sufficient for isolation, but verifying the reset path would strengthen confidence in the test seam.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Testing Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Assessment

The test suite is well-structured and covers the important behaviors: init-once semantics, concurrent init safety, poisoned-promise retry, escaping (special chars, U+2028/U+2029, null bytes, `</script>`), warning emission, dependency passthrough, and error handling. The 80 tests across 9 suites provide strong behavioral coverage of the bundler plugin layer.

The test helpers (`createMockMds`, `createPluginContext`, `createLoaderContext`) follow best practices: they are fakes that record interactions rather than deep mocks, tests follow clear Arrange-Act-Assert structure, and test names describe expected behavior. The `try/finally` pattern for injected test transformers ensures cleanup.

The single blocking issue is the inconsistent `NODE_ENV` guard on the test seam export -- webpack-loader has it, rollup-plugin and vite-plugin do not. This is a low-effort fix for consistency and production safety.

### Prior Resolution Coverage

Cycle 2 resolutions (warning emission tests, concurrency test, U+2028/U+2029 test, rollup warning path test) are all confirmed present and passing. No regressions from prior resolution work.

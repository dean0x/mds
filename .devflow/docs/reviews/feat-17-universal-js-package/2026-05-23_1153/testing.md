# Testing Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T11:53

## Issues in Your Changes (BLOCKING)

### HIGH

**U-WB1 test relies on implicit init() success side-effect, not isolated circuit-breaker behavior** - `packages/mds/__test__/wasm-backend.spec.mjs:21-26`
**Confidence**: 85%
- Problem: U-WB1 pre-seeds `initFailures` to 2 via `_resetForTesting(MAX_INIT_RETRIES - 1)` then calls `init()`. The test asserts `doesNotReject`, but `init()` succeeds not because of the circuit-breaker logic but because `_init()` actually loads the WASM module. This test conflates "circuit breaker allows the attempt" with "WASM loading succeeds in this environment". If the WASM module were unavailable, this test would fail even though the circuit-breaker logic is correct. The test name says "init() succeeds when failures are below the limit" but what it actually validates is that the WASM module happens to be loadable in the test environment.
- Fix: Reframe the test to validate the circuit-breaker gate specifically. The current test is acceptable as a behavioral integration test (the circuit breaker allows the attempt and init succeeds), but the test name should more precisely describe what is validated, or a comment should acknowledge that this is an integration test requiring a functional WASM build:
```javascript
test('U-WB1: init() attempts loading when failures are below the limit (requires WASM build)', async () => {
  _resetForTesting(MAX_INIT_RETRIES - 1);
  // Circuit breaker allows the attempt; success depends on WASM being built.
  await assert.doesNotReject(init());
});
```

### MEDIUM

**Hardcoded magic number duplicates source constant** - `packages/mds/__test__/wasm-backend.spec.mjs:12`
**Confidence**: 85%
- Problem: `const MAX_INIT_RETRIES = 3;` is hardcoded in the test file, duplicating the constant from `wasm.ts:28`. If the source constant changes, the test will silently test the wrong threshold. This is a coupling to implementation detail that could cause silent test drift.
- Fix: Either import the constant from the source module (if exported) or add a comment acknowledging the duplication and asserting the expected value:
```javascript
// Mirror of MAX_INIT_RETRIES from wasm.ts — if this value drifts, U-WB2
// will fail to trigger the exhaustion path, surfacing the mismatch.
const MAX_INIT_RETRIES = 3;
```
Note: The current design is partially self-correcting (U-WB2 would fail if the source constant increased), but if the source constant decreased, U-WB2 would pass while U-WB1 could start failing with a confusing error message.

**U-WB afterEach resets global singleton state that other test files depend on** - `packages/mds/__test__/wasm-backend.spec.mjs:15-19`
**Confidence**: 82%
- Problem: The `afterEach` hook calls `_resetForTesting(0)` to reset the WASM module singleton. If `wasm-backend.spec.mjs` runs before other test files that import from `../dist/backend/wasm.js` (same module singleton in the ESM module graph), the reset could clear a successfully initialized WASM module, causing subsequent tests in other files to fail. The comment mentions "the main backend.spec tests" but the test runner execution order across files is not guaranteed. The node:test runner runs files in the order specified on the command line or as discovered by glob, and ESM singletons are shared within a process.
- Fix: This is mitigated if each test file runs in its own process (node:test `--experimental-test-isolation=process`). If running in a single process, the `_resetForTesting(0)` at the end could leave wasm uninitialized. Verify the test runner configuration ensures process-level isolation, or document the dependency:
```javascript
afterEach(() => {
  // Reset wasm singleton. Safe because node:test runs each file in a
  // separate child process by default, so this cannot affect other suites.
  _resetForTesting(0);
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing test for browser.ts init() permanent failure path after retry removal** - `packages/mds/src/browser.ts:41-48`
**Confidence**: 85%
- Problem: The browser.ts `init()` function was changed to remove the `catch` block that previously reset `initVoidPromise = null` on failure. Now a rejected promise is cached permanently, relying on wasm.ts's MAX_INIT_RETRIES. However, no browser test validates this behavior. U-BR6 tests concurrent init success, but there is no test for the scenario where init() fails and subsequent calls return the same rejected promise. The wasm-backend.spec.mjs tests the circuit breaker at the wasm.ts layer, but the browser.ts caching layer is untested for the failure path.
- Fix: Add a test that validates the permanent-rejection semantics of browser.ts init(). This may require a `_resetForTesting()` export from browser.ts similar to what wasm.ts provides, or a subprocess-based test approach like U-B5 uses.

**No test coverage for tryLoadCandidate error path** - `packages/mds/src/backend/wasm.ts:81-96`
**Confidence**: 80%
- Problem: The newly extracted `tryLoadCandidate` function catches all errors and returns `null`. There is no test that validates this fallback behavior in isolation. The function is tested indirectly through `init()` (the first candidate fails, the second succeeds), but there is no explicit test that verifies a require() failure returns `null` rather than throwing.
- Fix: This is acceptable as-is since `tryLoadCandidate` is a private function and its behavior is covered by integration tests. However, consider adding a test that exercises the candidate fallback path explicitly (e.g., calling init() when the first candidate path does not exist, which is already the case in test environments where only one WASM build exists).

**No test coverage for statAndValidateModule extraction** - `packages/mds/src/util/module-scanner.ts:131-169`
**Confidence**: 80%
- Problem: The `statAndValidateModule` function was extracted from `scan()` as a refactoring. It is tested indirectly through `buildModulesMap` (U-SM1 through U-SM5), but the security behaviors it encapsulates (symlink rejection, TOCTOU detection, project root containment) are not directly tested. The existing tests do not create symlinks or simulate path swaps.
- Fix: This is a pre-existing gap that was not introduced by this PR (the logic existed inline before the extraction). The extraction itself is a clean refactoring. Consider adding symlink/TOCTOU tests in a follow-up PR when test infrastructure for filesystem manipulation is available.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Error tests U-E1 through U-E3 use try/catch instead of assert.throws** - `packages/mds/__test__/error.spec.mjs:10-35`
**Confidence**: 85%
- Problem: Tests U-E1, U-E2, and U-E3 use manual try/catch with `assert.fail()` instead of `assert.throws()` which is used elsewhere in the suite (U-C5, U-K3, U-BR9). The try/catch pattern has a subtle risk: if the `assert.fail()` call itself throws an `AssertionError` (which it does), the catch block receives that error and the assertions inside catch may pass or fail for the wrong reason. In these specific tests the assertions check `err instanceof Error` and `typeof err.code === 'string'` which would not match `assert.fail`'s `AssertionError` (it has no `.code`), so the risk is mitigated, but the pattern is inconsistent with the rest of the suite.
- Fix: Migrate to `assert.throws()` for consistency, matching the pattern used in U-C5, U-K3, etc.

### LOW

**U-SM4 duplicates U-SM1 without adding distinct behavioral coverage** - `packages/mds/__test__/scanner.spec.mjs:125-131`
**Confidence**: 80%
- Problem: U-SM4 ("shallow import chain succeeds within depth limit") runs the same code as U-SM1 ("builds modules map for entry with imports") with the same fixture path and the same assertion (`Object.keys(result.modules).length >= 3`). The test name implies it validates the depth limit, but the assertion is identical to U-SM1. This duplication does not add behavioral coverage.
- Fix: Either differentiate U-SM4 by using a deeper fixture chain or remove it in favor of U-SM1. A comment acknowledging the structural verification of MAX_IMPORT_DEPTH=64 (as done in U-SM5's comment) would improve clarity.

## Suggestions (Lower Confidence)

- **compileOpts() not tested with vars=null edge case** - `packages/mds/src/backend/wasm.ts:144-147` (Confidence: 70%) -- The `compileOpts` helper delegates to `varsOpt` which handles null/undefined, but there is no WASM-specific test that validates the frozen DEFAULT_COMPILE_OPTS is returned when no vars are provided. U-C7 covers this at the node.ts integration level but not at the wasm backend level.

- **Missing test for aggregate size limit in buildModulesMap** - `packages/mds/src/util/module-scanner.ts:201-205` (Confidence: 65%) -- The `maxAggregateSize` resource limit is present in the code but no test exercises it. U-SM5 tests `maxModules` but there is no equivalent test with `maxAggregateSize: 1`.

- **Browser test relies on top-level describe ordering assumption** - `packages/mds/__test__/browser.spec.mjs:6-8` (Confidence: 65%) -- The comment states "Node.js test runner executes top-level describe blocks sequentially" which is true for node:test, but this is an implementation detail of the runner. If tests are ever migrated to a different runner or the node:test semantics change, the pre-init/post-init ordering could break.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 3 | 0 |
| Pre-existing | 0 | 0 | 1 | 1 |

**Testing Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The test suite is well-structured with clear naming conventions (U-prefixed IDs), good use of the Arrange-Act-Assert pattern, and appropriate behavioral focus. The new wasm-backend.spec.mjs tests are a valuable addition for circuit-breaker coverage. The main conditions for approval are: (1) clarifying the U-WB1 test name to reflect its integration nature, and (2) acknowledging the hardcoded MAX_INIT_RETRIES duplication with a comment. The browser.ts permanent-failure path is the most significant coverage gap introduced by this PR's changes, but it is mitigated by the wasm.ts-level circuit-breaker tests.

Prior cycle resolutions (Cycle 3) are reflected: `_resetForTesting()` was added, `compileOpts()` was extracted, `statAndValidateModule()` was extracted, and `DEFAULT_COMPILE_OPTS` was deep-frozen. All of these have at least indirect test coverage through integration tests.

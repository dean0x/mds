# Testing Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25
**Diff**: `git diff db99f70...HEAD`
**Prior Resolutions**: Cycle 1 resolved 6 issues, 4 false positives. This is Cycle 2.

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**U-WCF6 error test does not assert error message content** - `packages/mds/__test__/wasm-compileFile.spec.mjs:115-127`
**Confidence**: 82%
- Problem: U-WCF6 only asserts `result.threw === true` but does not validate the error message contains any expected substring (e.g., "ENOENT", "no such file", or the path). A function that throws for the wrong reason (e.g., a JSON parse error, or a different internal invariant violation) would silently pass this test. The same pattern exists in U-WCF11 (line 201-213). Both error path tests are weak.
- Fix: Assert the error message contains an expected indicator:
```javascript
// U-WCF6 (line 126)
assert.ok(result.threw, 'compileFile on nonexistent path must throw');
assert.ok(result.message.length > 0, 'error message must not be empty');

// U-WCF11 (line 212)
assert.ok(result.threw, 'checkFile on nonexistent path must throw');
assert.ok(result.message.length > 0, 'error message must not be empty');
```

**Parity tests only compare output, not full shape** - `packages/mds/__test__/wasm-compileFile.spec.mjs:129-199`
**Confidence**: 80%
- Problem: U-WCF7 and U-WCF9 compare `output` between WASM and native but do not compare `warnings` or `dependencies`. A regression where WASM produces correct output but wrong dependency lists would not be caught. U-WCF8 and U-WCF10 compare only `warnings`. There is no parity test that validates `dependencies` arrays match between backends.
- Fix: Add `dependencies` comparison to the compileFile parity tests (U-WCF7 line 140, U-WCF9 line 176):
```javascript
assert.deepEqual(
  wasmResult.dependencies,
  nativeResult.dependencies,
  'WASM and native compileFile dependencies must match',
);
```

## Pre-existing Issues (Not Blocking)

No pre-existing issues found at CRITICAL severity.

## Suggestions (Lower Confidence)

- **Missing test for `prepareFileArgs` invariant violation branch** - `packages/mds/src/node.ts:74-77` (Confidence: 72%) -- The new `prepareFileArgs` function has an explicit invariant check (`if (source === undefined)`) that is unreachable through normal test execution because `buildModulesMap` always populates the entry key. This is defensive code; consider whether a unit-level test using a mock `buildModulesMap` would be worthwhile to verify the error message.

- **No test for WASM compileFile with deep import chain parity** - `packages/mds/__test__/wasm-compileFile.spec.mjs` (Confidence: 68%) -- U-WCF9/U-WCF10 test import parity with `IMPORT_CONSUMER_MDS` (single-level import), but there is no parity test using `ENTRY_MDS` (deep import chain). U-WCF3 tests that the deep chain succeeds on WASM, but does not compare output against native.

- **Subprocess test suite total duration is 722ms** - `packages/mds/__test__/wasm-compileFile.spec.mjs` (Confidence: 60%) -- 11 subprocess-based tests running sequentially take ~722ms. This is acceptable for now but as the suite grows, consider batching independent subprocess calls or using a shared init pattern within a describe block to reduce overhead.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 2 | - |
| Pre-existing | - | - | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The test suite is well-structured and comprehensive. The refactoring from `runWasm`/`runNative` to unified `runScript` with explicit env-factory functions is a clean improvement that reduces duplication. The addition of U-WCF9 through U-WCF11 closes meaningful coverage gaps (import parity, checkFile error path). The empty-stdout guard in `runScript` (line 35) is a good defensive addition from Cycle 1 resolution.

The two MEDIUM should-fix items are:
1. Error tests (U-WCF6, U-WCF11) assert that an error was thrown but not what the error is about -- a test that passes for the wrong reason is brittle.
2. Parity tests compare `output` and `warnings` but never `dependencies`, leaving a gap in cross-backend validation.

Neither is blocking; the core bug fix (filename collision) is well-tested through the existing assertions.

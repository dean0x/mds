# Testing Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Scanner test uses hand-rolled regex instead of actual scanImports implementation** - `packages/mds/__test__/scanner.spec.mjs:22-39`
**Confidence**: 90%
- Problem: The `buildModulesMap` tests (U-SM1 through U-SM3) use a hand-rolled regex-based `scanImports` function instead of the actual NAPI or WASM `scanImports` implementation. This regex approximation may not match the actual parser's behavior, meaning these tests validate the module scanner's graph-walking logic against a different import-detection implementation than production uses. If the regex diverges from the real parser (which it will for edge cases like comments, string literals containing `@import`, or multi-line imports), bugs could go undetected.
- Fix: Import the actual napi addon's `scanImports` (available via `napiAddon` from `helpers.mjs`) or the WASM module's `scanImports` and use it as the scanner function passed to `buildModulesMap`. The `normalizeVirtualKey` unit tests (U-S1 through U-S10) are fine as-is since they test a pure function directly.
```javascript
// In scanner.spec.mjs — use the real implementation
import { napiAddon } from './helpers.mjs';
function scanImports(source) {
  return napiAddon.scanImports(source);
}
```

**Missing test for WASM backend fallback path** - `packages/mds/__test__/backend.spec.mjs`
**Confidence**: 85%
- Problem: The backend test suite only validates the "native is default" path (U-B2). The `MDS_BACKEND=wasm` environment variable fallback path in `node.ts:14-16` and the automatic WASM fallback when native is unavailable (`node.ts:29-33`) are untested. These are critical code paths in the universal package's value proposition -- backend flexibility. The `console.warn` fallback path in particular could silently break.
- Fix: Add tests that validate backend selection under `MDS_BACKEND=wasm`. This can be done by spawning a child process with the env var set, or by directly testing `createWasmBackend` if WASM is available:
```javascript
import { execFileSync } from 'node:child_process';

test('U-B5: MDS_BACKEND=wasm selects wasm backend', () => {
  const result = execFileSync('node', [
    '-e', 'import("../dist/node.js").then(m => console.log(m.getBackend()))'
  ], { env: { ...process.env, MDS_BACKEND: 'wasm' }, encoding: 'utf-8' });
  assert.equal(result.trim(), 'wasm');
});
```

### MEDIUM

**Parity tests only cover native backend, not cross-backend comparison** - `packages/mds/__test__/parity.spec.mjs`
**Confidence**: 88%
- Problem: The file is named "parity" and its header says "Verifies that native and WASM backends produce identical results", but U-P1 through U-P6 only test the native backend directly via `createNativeBackend`. No test actually compares native output against WASM output. The WASM skip logic mentioned in the header comment is absent entirely -- there are no WASM tests in this file at all. The describe block is even named `'native backend parity'`, which is a contradiction in terms (parity requires two things to compare).
- Fix: Either rename the file/describe to `native-backend.spec.mjs` / `'native backend'` to accurately reflect what it tests, or add actual parity comparison tests that run both backends and `assert.deepEqual` their results when WASM is available:
```javascript
describe('cross-backend parity', { skip: !wasmAvailable }, () => {
  test('compile output matches between native and wasm', () => {
    const nResult = nativeBackend.compile('Hello {name}!\n', { vars: { name: 'Test' } });
    const wResult = wasmBackend.compile('Hello {name}!\n', { vars: { name: 'Test' } });
    assert.deepEqual(nResult, wResult);
  });
});
```

**Weak assertion in compileFile import test** - `packages/mds/__test__/compileFile.spec.mjs:23`
**Confidence**: 85%
- Problem: Test U-CF2 asserts `result.dependencies.length >= 0` which is always true for any array. The comment says "dependencies may include imported files" but the assertion is vacuous. This test compiles `import_consumer.mds` which imports `import_provider.mds`, so dependencies should be >= 1. If the dependencies tracking is broken and returns an empty array, this test would still pass.
- Fix: Assert a meaningful minimum:
```javascript
assert.ok(result.dependencies.length >= 1,
  `expected at least 1 dependency for file with imports, got: ${result.dependencies.length}`);
```

**No test for compile/check with invalid option types** - `packages/mds/__test__/compile.spec.mjs`, `packages/mds/__test__/check.spec.mjs`
**Confidence**: 80%
- Problem: All tests pass well-formed options (`{ vars: { name: 'World' } }`). There are no tests for boundary conditions on the options object itself: passing `null` as vars, non-object vars, vars with `undefined` values, or non-string var values. Since the package is a public API, these are real-world edge cases users will hit. The `isMdsError` type guard is tested for non-error inputs but the compile/check functions themselves are never called with adversarial options.
- Fix: Add boundary tests:
```javascript
test('compile with null vars does not crash', () => {
  const result = compile('Hello!\n', { vars: null });
  assert.ok(typeof result.output === 'string');
});

test('compile with non-string var values', () => {
  const result = compile('Value: {x}\n', { vars: { x: 42 } });
  assert.ok(typeof result.output === 'string');
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Error tests use try/catch instead of assert.throws (inconsistent pattern)** - `packages/mds/__test__/error.spec.mjs:11-78`
**Confidence**: 82%
- Problem: Tests U-E1, U-E2, U-E3, U-E6, U-E7, U-E8 all use explicit `try/catch` with `assert.fail('expected error to be thrown')` in the try block. Meanwhile, tests in `compile.spec.mjs` (U-C5) and `check.spec.mjs` (U-K3) use `assert.throws()` for the same purpose. The try/catch pattern is more verbose and harder to read; it also masks the error if the function succeeds but for an unexpected reason (e.g., returns undefined instead of throwing). Six repetitions of the same try/catch boilerplate is a test design red flag.
- Fix: Refactor error shape tests to use `assert.throws()` consistently with the rest of the suite:
```javascript
test('U-E1: compile syntax error is an Error instance', () => {
  assert.throws(
    () => compile('Hello {name\n'),
    (err) => {
      assert.ok(err instanceof Error);
      return true;
    },
  );
});
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **No tests for browser.ts entry point** - `packages/mds/src/browser.ts` (Confidence: 70%) -- The browser entry point's `compileFile`/`checkFile` rejection behavior and `init()` race condition guard are untested. These are new code paths that could only be tested via a browser-like environment or targeted unit tests of the assertion helpers.

- **Test for module-scanner resource limits untested** - `packages/mds/src/util/module-scanner.ts:126-135` (Confidence: 65%) -- The `maxModules` and `maxAggregateSize` limits in `buildModulesMap` have no dedicated tests. These are security-relevant bounds that could regress silently.

- **Performance tests use wall-clock time without warmup** - `packages/mds/__test__/perf.spec.mjs:19-26` (Confidence: 60%) -- U-PF1/U-PF2 use `Date.now()` without a warmup call. The first compile may include JIT compilation overhead. The 2000ms threshold is generous enough that this is unlikely to cause flakiness, but a single warmup call before the timed loop would make the benchmark more meaningful.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 2 | 2 | - |
| Should Fix | - | - | 1 | - |
| Pre-existing | - | - | - | - |

**Testing Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The test suite has good breadth (60 tests across 8 files covering compile, check, error shapes, scanner, and performance) with clean Arrange-Act-Assert structure and helpful test IDs. However, the scanner tests use a fake regex parser instead of the real implementation, the "parity" tests do not actually test cross-backend parity, and the compileFile import test has a vacuous assertion. The backend fallback paths -- a core selling point of the universal package -- are untested.

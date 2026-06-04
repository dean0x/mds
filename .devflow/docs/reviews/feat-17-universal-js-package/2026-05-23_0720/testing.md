# Testing Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20

## Cross-Cycle Awareness

Cycle 2 resolved 18 of 21 issues. Test-related fixes included: stale npm script references renamed, dead variable removed, unused imports cleaned, misleading test names corrected, isMdsError boundary test added, and 10 new browser tests added. The deferred item (WASM init retry circuit breaker untested) is re-evaluated below.

## Issues in Your Changes (BLOCKING)

### HIGH

**U-SM5 does not exercise the depth guard it claims to test** - `packages/mds/__test__/scanner.spec.mjs:133-148`
**Confidence**: 85%
- Problem: The test is named "rejects import chain exceeding depth limit" but the extensive comment (lines 134-143) acknowledges it actually triggers the `maxModules: 1` resource limit, not the `MAX_IMPORT_DEPTH=64` depth guard. The test name and the exercised code path are misaligned. The test validates a resource-limit path (module count), while the depth guard (`depth > MAX_IMPORT_DEPTH` at `module-scanner.ts:137`) remains untested. The comment attempts to justify this as "structurally verified" but structural verification is not behavioral verification.
- Fix: Rename the test to accurately describe what it verifies, and add a targeted depth-guard test using a synthetic `scanImports` that returns unique import paths on each call to force recursion without the visited-set short-circuit:
```javascript
test('U-SM5: rejects when module count exceeds maxModules', async () => {
  const entryPath = path.join(FIXTURES, 'imports', 'entry.mds');
  await assert.rejects(
    () => buildModulesMap(entryPath, scanImports, { maxModules: 1 }),
    /resource limit/,
  );
});

test('U-SM6: depth guard rejects deeply chained imports', async () => {
  // Synthetic scanImports that returns a unique child on every call,
  // forcing recursion past MAX_IMPORT_DEPTH without visited-set dedup.
  let callCount = 0;
  const deepScanner = (_source) => {
    callCount++;
    return callCount <= 65 ? [`./child_${callCount}.mds`] : [];
  };
  // Use a real entry file; the scanner overrides import resolution.
  // This will fail at validateImportPath since child files don't exist,
  // which is acceptable — the depth check fires before filesystem access
  // only if depth > 64, so we'd need a stub for readFile too.
  // A pragmatic approach: accept the current maxModules test as proxy coverage
  // and rename U-SM5 to match what it actually exercises.
});
```
  At minimum, rename U-SM5 to `"rejects when module count exceeds maxModules"` so the test name matches the behavior it verifies.

### MEDIUM

**Browser pre-init test ordering relies on undocumented node:test describe sequencing** - `packages/mds/__test__/browser.spec.mjs:26-96`
**Confidence**: 82%
- Problem: The test file header (lines 6-8) states "pre-init tests run inside a describe whose before() hook establishes ordering" but there is no `before()` hook in the pre-init describe block. The ordering works correctly today because Node's test runner (v22+) runs top-level `describe` blocks sequentially and `U-BR6` (which calls `init()`) is the last test in the pre-init suite. However, this relies on two implicit guarantees: (1) describe blocks run sequentially, and (2) tests within a describe complete before the next describe starts. The header comment is misleading about the mechanism.
- Fix: Either add an explicit comment correcting the doc header, or restructure to be more resilient:
```javascript
/**
 * ...
 * Node.js test runner executes top-level describe blocks sequentially,
 * so pre-init tests complete before the post-init suite starts.
 * U-BR6 (concurrent init) is intentionally placed last in the pre-init
 * suite so that init() is called only after all pre-init assertions run.
 */
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**WASM init retry circuit breaker (MAX_INIT_RETRIES=3) has no test coverage** - `packages/mds/src/backend/wasm.ts:28-49`
**Confidence**: 85%
- Problem: The `wasm.ts` init function has a retry circuit breaker (lines 28-29, 45-49) that tracks `initFailures` and permanently rejects after 3 attempts. This behavior is referenced in the browser.ts comments (line 27, line 46) but has zero test coverage. This was deferred in cycle 2 as "architectural" but it is testable: the circuit breaker is a behavioral contract of the `init()` function. An incorrect `initFailures` counter or a missing reset could silently allow infinite retries or block recovery.
- Fix: This requires injecting a failure into the WASM loading path. One approach is a dedicated test file that uses a subprocess with a manipulated module resolution to force WASM load failures:
```javascript
test('U-W1: init permanently rejects after MAX_INIT_RETRIES failures', async () => {
  // Spawn a subprocess where the WASM module path is invalid,
  // forcing repeated init() failures. After 3 attempts, the 4th
  // should throw the permanent failure message.
  const script = `
    import { init } from '../dist/backend/wasm.js';
    for (let i = 0; i < 4; i++) {
      try { await init(); } catch (e) {
        if (e.message.includes('failed to initialize after')) {
          console.log('CIRCUIT_OPEN');
          process.exit(0);
        }
      }
    }
    console.log('NO_CIRCUIT');
  `;
  // ... execFileSync with --input-type=module, env without WASM available
});
```

**No test coverage for varsOpt behavioral change (null coalescing)** - `packages/mds/src/util/options.ts:11`
**Confidence**: 80%
- Problem: The `varsOpt` function was changed from `!== undefined` to `!= null`, which changes behavior: `{ vars: null }` previously returned `{ vars: null }` (passing null to the backend) and now correctly returns `undefined` (treating null as "no vars"). Test U-C7 (`compile with null vars does not throw`) verifies the end-to-end behavior but does not verify that null vars are actually omitted from the options passed to the backend. The fix is correct and the test passes, but the specific behavioral change lacks a unit-level assertion.
- Fix: Add a targeted test for `varsOpt` directly, or strengthen U-C7:
```javascript
test('U-C7: compile with null vars produces same output as no vars', () => {
  const withNull = compile('Hello World!\n', { vars: null });
  const withoutVars = compile('Hello World!\n');
  assert.equal(withNull.output, withoutVars.output);
});
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Error tests U-E1 through U-E3 use try/catch instead of assert.throws** - `packages/mds/__test__/error.spec.mjs:10-35`
**Confidence**: 85%
- Problem: Tests U-E1, U-E2, and U-E3 use manual try/catch with `assert.fail()` instead of the cleaner `assert.throws()` pattern already used in U-C5, U-BR1, U-BR9, and other tests in this PR. This is a style inconsistency. The try/catch pattern risks silently passing if the code under test returns normally and assert.fail itself throws an error matching the catch predicate (unlikely here, but the pattern is fragile by nature).
- Fix: Refactor to use `assert.throws()`:
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

### LOW

**scanner.spec.mjs duplicates FIXTURES/scanImports instead of importing from helpers** - `packages/mds/__test__/scanner.spec.mjs:13,19,22-39`
**Confidence**: 80%
- Problem: The scanner test file defines its own `__dirname`, `FIXTURES`, and `scanImports` helper locally rather than importing `FIXTURES` from `helpers.mjs` (which already exports it). The `scanImports` regex-based function is only used in scanner tests so local definition is reasonable, but `FIXTURES` is duplicated unnecessarily.
- Fix: Import `FIXTURES` from helpers:
```javascript
import { FIXTURES } from './helpers.mjs';
```

## Suggestions (Lower Confidence)

- **No test for browser init() failure/retry path** - `packages/mds/__test__/browser.spec.mjs` (Confidence: 70%) -- The browser.ts `init()` function has a `.catch()` handler (line 44-48) that resets `initVoidPromise` on failure to allow retries. No test verifies that a failed init can be retried successfully, or that the error propagates correctly to the caller.

- **Aggregate size limit in buildModulesMap is untested** - `packages/mds/src/util/module-scanner.ts:188-193` (Confidence: 65%) -- The `maxAggregateSize` resource limit path has no test coverage. A test could use `{ maxAggregateSize: 1 }` to trigger it with the existing fixtures.

- **U-SM4 is functionally identical to U-SM1** - `packages/mds/__test__/scanner.spec.mjs:125-131` (Confidence: 75%) -- Both tests call `buildModulesMap` on the same `entry.mds` fixture and assert `>= 3` modules. U-SM4 adds "within depth limit" framing but does not verify anything beyond what U-SM1 already asserts. Consider making U-SM4 verify the actual depth somehow, or removing it to reduce test suite noise.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 1 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The test suite is well-structured with good behavioral coverage of the public API surface (76 tests passing, 10 new browser tests). Test naming conventions are consistent (U-prefix IDs), AAA structure is followed, and the new browser tests correctly handle the async init ordering problem. The main gaps are: (1) the U-SM5 test name misrepresents what it actually verifies (depth guard vs module count limit), (2) the WASM circuit breaker retry logic remains untested from cycle 2, and (3) the varsOpt null-handling behavioral change lacks direct unit verification. None of these are critical blockers, but the misleading test name (U-SM5) should be corrected before merge to avoid confusion for future maintainers.

# Testing Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**PR**: #34

## Issues in Your Changes (BLOCKING)

### MEDIUM

**U-PR3 `findProjectRoot` fallback test may be environment-dependent** - `packages/mds/__test__/scanner.spec.mjs:231-248`
**Confidence**: 82%
- Problem: The test creates a temp directory with no `.git` or `.mdsroot` marker and asserts `findProjectRoot(sub) === sub` (strict equality). If the test runs on a machine where `os.tmpdir()` resides inside a git repository (some CI containers mount `/tmp` inside a worktree), `findProjectRoot` would walk up and find that `.git`, returning a different path. The comment on line 238-242 acknowledges this risk but the assertion remains strict. Compare with `U-PR4` (line 251-267), which uses a deliberately weaker assertion (`typeof result === 'string' && result.length > 0`) for exactly this reason.
- Fix: Weaken the assertion to accept either the fallback or any ancestor that contains a `.git`/`.mdsroot` marker:
```javascript
// Assert: fallback equals the start argument, OR an ancestor was found
// (we cannot control whether os.tmpdir() is inside a git repo).
assert.ok(
  result === sub || sub.startsWith(result + '/'),
  `result must be sub or an ancestor of sub, got: ${result}`
);
```

**`capturedCallback` variable assigned but never asserted** - `packages/webpack-loader/__test__/cjs-compat.spec.mjs:31`
**Confidence**: 85%
- Problem: `let capturedCallback = null;` is set inside the mock `async()` callback but is never asserted upon. The comment says "We only check the return type -- we do not assert on side effects", which is fine for the test's stated purpose. However, the variable itself is dead code that implies an incomplete test. It should either be removed (if truly unused) or asserted (if the intent was to verify the error path).
- Fix: Remove the dead variable since the test explicitly states it only checks the return type:
```javascript
const mockContext = {
  resourcePath: '/dev/null/nonexistent.mds',
  async() { return () => {}; },
  addDependency() {},
  emitWarning() {},
  getOptions() { return {}; },
};
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**CJS compat tests depend on build artifacts without documented build prerequisite** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs:19`, `packages/webpack-loader/__test__/cjs-compat.spec.mjs:19`
**Confidence**: 80%
- Problem: Both CJS compatibility test files require `../dist-cjs/index.js`, which is a build artifact produced by the CJS build step. If a developer runs `npm test` without first building the CJS output, the tests fail with an unhelpful `MODULE_NOT_FOUND` error. The test files have good doc comments explaining what they verify, but no guard for missing build output that would give a clear diagnostic.
- Fix: Add an existence guard at the top of the describe block or in a `before()` hook:
```javascript
import { existsSync } from 'node:fs';

const cjsPath = resolve(__dirname, '../dist-cjs/index.js');
const hasCjsBuild = existsSync(cjsPath);

describe('bundler-utils CJS build', { skip: !hasCjsBuild && 'dist-cjs/ not built (run build:cjs first)' }, () => {
  // ... existing tests
});
```

**Repeated `require(resolve(__dirname, '../dist-cjs/index.js'))` in every test** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs` (7 occurrences), `packages/webpack-loader/__test__/cjs-compat.spec.mjs` (5 occurrences)
**Confidence**: 81%
- Problem: Every test individually calls `require(resolve(__dirname, '../dist-cjs/index.js'))`. Because `require()` caches modules by resolved path, the repeated calls are functionally harmless, but the repetition obscures the test intent and adds unnecessary boilerplate. The first test (`loads without error via require()`) already verifies the load succeeds, so subsequent tests could reference a shared binding.
- Fix: Hoist the require to describe-level scope:
```javascript
const cjsBuild = require(resolve(__dirname, '../dist-cjs/index.js'));

describe('bundler-utils CJS build', () => {
  test('loads without error via require()', () => {
    assert.ok(cjsBuild, 'CJS build should load successfully');
  });
  test('exports createMdsTransformer', () => {
    assert.equal(typeof cjsBuild.createMdsTransformer, 'function');
  });
  // ...
});
```

## Pre-existing Issues (Not Blocking)

_No pre-existing CRITICAL issues identified._

## Suggestions (Lower Confidence)

- **Missing negative test for `@if !premium:` with `premium` undefined** - `crates/mds-cli/tests/language.rs` (Confidence: 65%) -- The negation tests cover true, false, 0, empty string, null, and dot-paths, but do not test the error path when the negated variable is undefined. The error-path test `if_negation_undefined_variable_is_error` exists in `errors.rs`, but a mirrored assertion in `language.rs` that undefined negation does NOT produce output would strengthen the behavioral contract.

- **`findProjectRoot` cache is never cleared between tests** - `packages/mds/__test__/scanner.spec.mjs` (Confidence: 70%) -- The `projectRootCache` module-level Map persists across all tests in the same process. The tests use `mkdtemp` to generate unique paths (avoiding cache collisions), which is the correct mitigation. However, if a future test reuses a path string, stale cache entries could cause subtle failures. A `_resetCacheForTesting()` export would make this robust.

- **No test for `@elseif` with equality + inequality mixed in the same chain** - `crates/mds-cli/tests/language.rs` (Confidence: 62%) -- Tests cover `@elseif` with truthiness, negation, and equality individually, but no single test exercises a chain mixing `==`, `!=`, and truthiness conditions in the same `@if`/`@elseif` block.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This PR introduces substantial and well-structured test coverage for all new language features (negation, equality/inequality operators, `@elseif`), CJS compatibility, and the `findProjectRoot` utility. Key strengths:

- **Comprehensive behavioral coverage**: 50+ new integration tests in `language.rs` and `errors.rs` cover happy paths, edge cases (NaN, cross-type comparisons, empty strings), error diagnostics, and boundary conditions (MAX_ELSEIF_BRANCHES).
- **Correct test architecture**: Tests use `compile_str()` for inline templates (no external fixture files needed), follow clear Arrange-Act-Assert structure, and assert on observable output rather than implementation details (applies ADR-002 -- tests verify the behavioral contract matches the stated PR goals).
- **CJS compat tests verify real behavioral contracts**: The webpack-loader test invokes the actual loader function with a mock context and validates the Promise return type, going beyond simple export-existence checks.
- **Prior cycle improvements incorporated**: findProjectRoot unit tests (U-PR1 through U-PR5), NaN semantics test, and behavioral CJS test all reflect cycle 2 resolutions.

The two blocking MEDIUM issues are minor quality improvements (dead variable, environment-dependent assertion) that do not affect correctness on most systems. The should-fix items are about test ergonomics (build prerequisite guard, DRY require calls).

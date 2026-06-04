# Testing Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing unit test for MAX_ELSEIF_BRANCHES resource limit** - `crates/mds-core/src/parser.rs:273`
**Confidence**: 90%
- Problem: The parser enforces a `MAX_ELSEIF_BRANCHES = 256` limit at line 273, but there is no test verifying this limit is enforced. Every other resource limit in the parser has a corresponding test (MAX_NESTING_DEPTH, MAX_DOT_SEGMENTS, MAX_CALL_DEPTH, MAX_OUTPUT_SIZE, MAX_IMPORT_DEPTH, maxModules, maxAggregateSize). This is the only resource limit without a boundary test, and it is a newly added code path that could silently regress.
- Fix: Add a parser unit test that constructs an `@if` block with `MAX_ELSEIF_BRANCHES + 1` `@elseif` branches and asserts the parse error mentions the branch limit. Pattern follows existing `parse_nesting_depth_limit_rejected` test:
```rust
#[test]
fn parse_elseif_branch_limit_rejected() {
    let mut src = String::from("@if x:\nbody\n");
    for _ in 0..=MAX_ELSEIF_BRANCHES {
        src.push_str("@elseif x:\nbranch\n");
    }
    src.push_str("@end\n");
    let tokens = tokenize(&src, "test.mds").unwrap();
    let result = parse_with_ctx(&tokens, "", "");
    assert!(result.is_err(), "exceeding MAX_ELSEIF_BRANCHES must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("@elseif branches"), "error must mention branch limit, got: {msg}");
}
```

### MEDIUM

**Missing unit tests for `findProjectRoot` function** - `packages/mds/src/util/module-scanner.ts:25-40`
**Confidence**: 85%
- Problem: `findProjectRoot` is a newly added exported function with non-trivial logic (marker-based traversal with depth limit and root-of-filesystem fallback). It is only tested indirectly through the `U-SM8` integration test, which relies on the `.git` directory happening to exist at the right level. There are no isolated unit tests for its edge cases: no marker found (fallback to start), reaching filesystem root, the depth limit guard.
- Fix: Add direct unit tests for `findProjectRoot` in `scanner.spec.mjs`:
```javascript
describe('findProjectRoot', () => {
  test('returns directory containing .git marker', () => {
    // Use the known fixtures dir which is under the repo .git
    const root = findProjectRoot(path.join(FIXTURES, 'imports'));
    assert.ok(existsSync(path.join(root, '.git')), 'should find .git marker');
  });

  test('falls back to start dir when no marker found', async () => {
    const tmpDir = await mkdtemp(path.join(os.tmpdir(), 'mds-root-test-'));
    try {
      const result = findProjectRoot(tmpDir);
      assert.equal(result, tmpDir, 'should fall back to start when no marker');
    } finally {
      await rm(tmpDir, { recursive: true, force: true });
    }
  });
});
```

**Webpack loader async detection uses fragile heuristic** - `packages/webpack-loader/__test__/cjs-compat.spec.mjs:28-32`
**Confidence**: 82%
- Problem: The test checks whether the default export is an async function via `mdsLoader.constructor.name === 'AsyncFunction' || mdsLoader.toString().includes('async')`. The `toString()` fallback is fragile -- minified/transpiled CJS output may not contain the literal string "async" even if the function is async. Under CJS transpilation, TypeScript may wrap the async function in `__awaiter`, producing output where neither check is reliable. If the CJS build changes its transpilation target, this test could silently pass or fail incorrectly.
- Fix: Test the behavioral contract instead -- call the function with a mock context and verify it returns a Promise:
```javascript
test('default export returns a Promise when invoked', () => {
  const { default: mdsLoader } = require(resolve(__dirname, '../dist-cjs/index.js'));
  // Create minimal mock context
  const ctx = {
    resourcePath: '/fake.mds',
    async: () => () => {},
    getOptions: () => ({}),
    addDependency: () => {},
    emitWarning: () => {},
  };
  const result = mdsLoader.call(ctx);
  assert.ok(result instanceof Promise || (result && typeof result.then === 'function'),
    'loader must return a thenable');
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**No unit test for `values_equal` NaN semantics** - `crates/mds-core/src/evaluator.rs:336-344`
**Confidence**: 80%
- Problem: The `values_equal` function documents IEEE 754 NaN semantics (`NaN == NaN` is `false`). While the parser now rejects NaN literals at parse time (tested in `condition_value_nan_rejected`), if `NaN` were ever to reach `values_equal` through a code path that bypasses the parser (e.g., runtime-computed values in a future version), the documented behavior should be verified. The doc comment makes a specific claim that has no corresponding test.
- Fix: Add a unit test in the evaluator's `#[cfg(test)]` module that directly exercises `values_equal` with `f64::NAN`:
```rust
#[test]
fn values_equal_nan_is_not_equal_to_nan() {
    let nan_val = Value::Number(f64::NAN);
    let nan_cond = CondValue::Number(f64::NAN);
    assert!(!values_equal(&nan_val, &nan_cond), "NaN must not equal NaN (IEEE 754)");
}
```

## Pre-existing Issues (Not Blocking)

(none found at CRITICAL severity in unchanged code)

## Suggestions (Lower Confidence)

- **Repeated `require()` calls across test cases** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs` (Confidence: 65%) -- Each test calls `require(resolve(__dirname, '../dist-cjs/index.js'))` independently. While Node caches `require()` results, extracting it to a `describe`-level constant would clarify intent and reduce repetition.

- **No test for @elseif branch limit at exactly the boundary** - `crates/mds-core/src/parser.rs:273` (Confidence: 70%) -- If the exceeding-limit test from the HIGH finding above is added, consider also adding an at-limit acceptance test (exactly `MAX_ELSEIF_BRANCHES` branches must succeed), following the pattern of `parse_nesting_depth_at_limit_accepted`.

- **Scanner test U-SM8 relies on `.git` marker being present** - `packages/mds/__test__/scanner.spec.mjs:159` (Confidence: 62%) -- The cross-directory import test depends on `.git` existing in the repo root. If the test suite is ever run from a tarball or shallow clone without `.git`, the project root discovery would fall back to the entry directory and the cross-dir import would fail. This is unlikely but could be guarded by checking for `.git` availability in a `before()` hook.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The test suite for the new language features (negation, equality, @elseif) is thorough and well-structured, with excellent coverage of happy paths, edge cases (type coercion, cross-type comparisons, empty strings, null, floats), error paths (parse errors with actionable messages), and interaction tests (nested @if inside @elseif, short-circuit semantics). The CJS compatibility tests validate the critical webpack interop path. The one blocking gap is the missing resource limit test for MAX_ELSEIF_BRANCHES -- every other resource limit in the codebase has boundary tests, and this omission creates a regression risk for a security-relevant guard.

Applies ADR-001 -- the pre-merge gate should verify the MAX_ELSEIF_BRANCHES limit test is added before squash merge.

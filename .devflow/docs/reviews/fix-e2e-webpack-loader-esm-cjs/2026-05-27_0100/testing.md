# Testing Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27T01:00:00Z

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Duplicate test: `if_double_negation_error` and `if_double_negation_is_parse_error` test identical behavior** - `crates/mds-cli/tests/errors.rs:262`, `crates/mds-cli/tests/errors.rs:386`
**Confidence**: 90%
- Problem: Both tests verify that `@if !!var:` produces a "double negation" error. `if_double_negation_error` (line 262) uses `!!premium`, `if_double_negation_is_parse_error` (line 386) uses `!!var`. The only difference is the variable name; the behavior tested is identical. This adds noise and maintenance burden without covering any additional edge case.
- Fix: Remove one of the two tests. Keep `if_double_negation_is_parse_error` in the "condition parse error tests" section (line 386) since it sits with the other parse-error tests and has the more descriptive name.

**Dead `cjsBuild` variable shared across tests via describe scope (2 occurrences)** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs:18`, `packages/webpack-loader/__test__/cjs-compat.spec.mjs:18`
**Confidence**: 92%
- Problem: Both CJS compat specs declare `let cjsBuild;` at the describe scope (line 18) and assign it in the first test (`cjsBuild = require(...)`, line 22). However, no subsequent test reads `cjsBuild` — every other test does its own `require()` call. The shared mutable state is dead code and creates the appearance of test-order coupling (a test anti-pattern) even though no actual coupling exists.
- Fix: Replace the describe-scoped `let cjsBuild;` with `const cjsBuild` local to the first test:
```javascript
test('loads without error via require()', () => {
    const cjsPath = resolve(__dirname, '../dist-cjs/index.js');
    const cjsBuild = require(cjsPath);
    assert.ok(cjsBuild, 'CJS build should load successfully');
});
```
Note: Uncommitted working tree changes already apply this fix.

**Webpack CJS test 5 is a tautology** - `packages/webpack-loader/__test__/cjs-compat.spec.mjs:47`
**Confidence**: 85%
- Problem: Test "CJS build uses require() for @mds/bundler-utils (not import)" (line 47-56, committed version) calls `require()` on the same path as test 1 and asserts `typeof cjsBuild.default === 'function'`. This duplicates the assertion from test 2 ("exports default as an async function") and does not actually verify that bundler-utils is resolved via CJS rather than ESM — it only verifies the default export exists. The test name claims something the assertions cannot prove.
- Fix: Remove this test entirely. The CJS resolution of bundler-utils is already implicitly verified by all the other `require()` calls succeeding. Note: Uncommitted working tree changes already remove this test.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing error-path test: `@elseif` after `@else:` should produce a parse error** - `crates/mds-cli/tests/errors.rs`
**Confidence**: 82%
- Problem: The spec (`spec.md:153`) documents: "@elseif must appear before @else:; @else: cannot be followed by @elseif". The parser handles this correctly (the `@else:` body parser uses `parse_body(&["@end"], &[])` with no elseif prefix terminators, so any `@elseif` after `@else:` hits the "unknown directive" path). However, there is no test verifying this constraint. The error message would also be suboptimal — "unknown directive" rather than "@elseif cannot appear after @else:". Given that this is a spec-documented invariant for the new `@elseif` feature, a test is warranted.
- Fix: Add a test in `errors.rs`:
```rust
#[test]
fn elseif_after_else_is_parse_error() {
    let source = "---\nx: true\n---\n@if x:\nyes\n@else:\nno\n@elseif x:\nbad\n@end\n";
    let result = mds::compile_str(source);
    assert!(result.is_err(), "@elseif after @else: must be rejected");
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`find_unquoted_operator` escape-then-close-quote ordering could be clarified with an early continue** - `crates/mds-core/src/parser.rs:493` (Confidence: 65%) — The escape-character check (line 498) runs after the close-quote check (line 494) rather than before it. While functionally correct (backslash is never `"` or `'`), swapping the order to check escape first would make the intent clearer and eliminate a potential future maintenance trap if the quote chars ever change.

- **Async function detection in webpack-loader CJS test is fragile** - `packages/webpack-loader/__test__/cjs-compat.spec.mjs:29` (Confidence: 62%) — The `mdsLoader.constructor.name === 'AsyncFunction' || mdsLoader.toString().includes('async')` check depends on runtime characteristics that can change with TS target settings or minification. Consider checking `typeof mdsLoader(...) === 'object' && typeof mdsLoader(...).then === 'function'` instead, which tests the actual contract (returns a thenable).

- **No unit test for `parse_condition` function directly** - `crates/mds-core/src/parser.rs:538` (Confidence: 60%) — The new `parse_condition` function is well-tested via integration tests in `language.rs` and `errors.rs`, but there are no unit tests in the `parser.rs` `mod tests` block that directly exercise `parse_condition` with edge cases. The integration coverage is likely sufficient, but direct unit tests would provide faster feedback on regressions.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The test coverage for both features (Webpack CJS compat and @if equality/negation/@elseif) is strong. The PR adds 45 behavior tests and 8 error tests for the language features, covering happy paths, edge cases (type coercion, NaN, null, negative numbers, floats, empty strings, dot paths), error paths (unterminated strings, double negation, missing RHS, undefined variables, bare `=`), and structural cases (@elseif chaining, short-circuit, nesting, five-branch chains). The CJS compat tests verify export surface and basic behavioral correctness.

The blocking issues are all low-severity (duplicate test, dead code, tautological test) and two of the three are already fixed in uncommitted working tree changes. The one should-fix issue (missing `@elseif` after `@else:` error test) addresses a gap in spec-constraint coverage for the new feature.

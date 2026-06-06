# Testing Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04T16:54

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing NotEq (!=) operator tests with expression-based conditions**
**Confidence**: 90%
- `crates/mds-core/src/parser_tests.rs` (new test section, lines 1077-1580)
- Problem: The PR adds `Condition::NotEq(Expr, Expr)` which now accepts full expressions on both sides (previously `NotEq(Vec<String>, CondValue)`). There are zero tests -- neither parser-level nor evaluator-level -- for `!=` with expression-based operands (e.g., `@if lower(name) != "bob":`). The `==` path has thorough coverage (`parse_if_call_eq_literal`, `parse_if_call_eq_call`, `evaluate_if_call_eq_call_match`, `evaluate_if_call_eq_call_no_match`) but the `!=` path has none. The underlying `find_unquoted_operator` function checks `!=` before `==`, and `values_equal_runtime` is inverted for `NotEq`, but neither code path is exercised with the new expression types.
- Fix: Add at minimum:
  1. Parser test: `@if lower(name) != "alice":` produces `Condition::NotEq(Expr::Call, Expr::StringLiteral)`
  2. Evaluator test: `@if lower(name) != "bob":` with `name: Alice` evaluates to truthy branch

### MEDIUM

**Missing OR (||) operator test with expression-based operands** - `crates/mds-core/src/parser_tests.rs`
**Confidence**: 85%
- Problem: `parse_if_and_with_calls` (line 1150) tests `&&` with `Expr::Call` operands, and `evaluate_if_and_with_calls` (line 1465) tests the evaluator path. But there is no equivalent test for `||` with expression-based operands. The `split_on_unquoted_op` function was modified to respect parentheses (line 470-479), and the `||` splitting path is only tested with simple variable operands (pre-existing `parse_condition_or_two_vars`). An `@if func(a) || func(b):` test would exercise the new paren-aware `||` splitting.
- Fix: Add `parse_if_or_with_calls` and `evaluate_if_or_with_calls` tests, e.g.:
  ```rust
  #[test]
  fn parse_if_or_with_calls() {
      let src = "@if contains(t, \"x\") || contains(t, \"y\"):\nyes\n@end\n";
      // Assert Condition::Or with two Truthy(Expr::Call) operands
  }
  ```

**Node-API error test catches but does not assert error content** - `examples/node-api-test.mjs:503-510`
**Confidence**: 82%
- Problem: The error test `'expression @if: error cases (undefined function)'` uses a bare `catch {}` that swallows all errors without asserting the error message content. This means any error -- including unexpected ones like a WASM memory corruption or wrong error type -- would pass the test. All other error tests in the same file follow similar patterns, so this is consistent with the existing style, but for new feature validation it would be more robust to assert the error message.
- Fix: Assert the error message contains relevant context:
  ```javascript
  } catch (e) {
    assert(e.message.includes('notabuiltin') || e.message.includes('undefined'),
      'error should reference the undefined function');
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**No @for with qualified call test despite validator coverage** - `crates/mds-core/src/validator.rs:146-172`
**Confidence**: 80%
- Problem: The validator has a dedicated `Expr::QualifiedCall` branch in `validate_for_node` (lines 146-172) that validates namespace existence, function existence, and arity. This code path has zero test coverage -- no parser test for `@for x in ns.func(args):` and no evaluator test. While the qualified call path in `@if` conditions is tested (`parse_if_qualified_call_truthy`, line 1172), the `@for` equivalent is not.
- Fix: Add a test exercising `@for x in ns.func(args):` through the full compile pipeline (requires an imported module with a function returning an array).

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No NotEq operator tests at all across the entire test suite**
**Confidence**: 88%
- Problem: `Condition::NotEq` has zero dedicated tests across `parser_tests.rs` and `evaluator.rs` -- neither for the old `Vec<String>, CondValue` signature nor for the new `Expr, Expr` signature. The only indirect coverage is through the `NaN` semantics test (`values_equal_nan_is_not_equal_to_itself`) which tests `values_equal_runtime` directly. This pre-dates the PR.

## Suggestions (Lower Confidence)

- **Missing negative test: expression evaluating to non-iterable in @for** - `crates/mds-core/src/parser_tests.rs` (Confidence: 72%) -- `evaluate_for_non_array_result_is_error` tests `upper(name)` returning a string, but there is no test for a function returning a number, boolean, null, or object. These would exercise different type-error paths.

- **Missing backward-compat test for @if with dot-path equality** (Confidence: 70%) -- `parse_backward_compat_if_var_eq_string` tests `@if role == "admin":` but there is no backward-compat test for `@if config.debug == true:` (dot-path LHS with equality operator). The new code routes this through `parse_expr_inner` which produces `MemberAccess`, so a regression test would be valuable.

- **Security test allocates large memory** - `crates/mds-core/src/parser_tests.rs:1535-1580` (Confidence: 65%) -- `split_resource_limit_too_many_elements` creates a string with 100K+ commas and `join_resource_limit_output_too_large` allocates 50K+ elements of 1KB each. While these test important limits, they could strain CI memory in constrained environments. Consider gating with `#[cfg(not(miri))]` or documenting expected memory use.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR adds 31 Rust tests and 9 Node-API integration tests covering the new expression directive feature well across parser assertions, evaluator integration, security limits, backward compatibility, and error cases. Test quality is high -- tests follow AAA structure, assert behavioral outcomes, and cover both happy and error paths. The main gap is asymmetric operator coverage: `==` has thorough expression-based tests but `!=` has none, and `&&` with calls is tested but `||` with calls is not. Fixing the HIGH issue (NotEq coverage) and the top MEDIUM issue (OR with calls) would bring this to strong coverage. Applies ADR-008 (bundled language features in single PR -- tests appropriately bundled too).

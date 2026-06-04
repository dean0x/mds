# Testing Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

This PR bundles three language features (#53 built-in functions, #54 default arguments, #55 logical operators) into a single PR (applies ADR-008). All 690 tests pass. The PR adds approximately 100 new tests across builtins.rs (52), parser_tests.rs (35), evaluator.rs (11), and error_tests.rs (2).

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing test for `length()` byte-vs-character semantics on multi-byte strings** - `crates/mds-core/src/builtins.rs:341`
**Confidence**: 85%
- Problem: `builtin_length` uses `s.len()` which returns byte count, not character count. `length("café")` returns `5` (bytes) not `4` (characters). No test documents which behavior is intended. For a template language, users will expect character-count semantics. Whether the current byte-count behavior is intentional or a bug, it needs a test that pins the contract.
- Fix: Add a test that explicitly documents the intended behavior:
  ```rust
  #[test]
  fn length_string_multibyte_returns_byte_count() {
      // "café" is 5 bytes but 4 Unicode scalar values.
      // length() returns byte length (consistent with slice() indices).
      let result = call_builtin("length", &[s("café")]).unwrap();
      assert_eq!(result, Value::Number(5.0));
  }
  ```

**No validator-level test for built-in function arity checking** - `crates/mds-core/src/validator.rs:195-207`
**Confidence**: 82%
- Problem: The validator gained new logic to recognize built-in functions and check their arity at validation time (lines 195-210, 300-312). This is distinct from the evaluator arity check -- it catches arity errors early, before evaluation. There are zero tests that exercise this validator path. The evaluator integration tests (`compile_str`) do exercise the validator indirectly, but a targeted test would catch regressions if the validator's built-in lookup were accidentally removed.
- Fix: Add a validator-level test in the `validator::tests` module:
  ```rust
  #[test]
  fn builtin_wrong_arity_rejected_at_validate_time() {
      // upper() takes 1 arg; calling with 0 or 2 should fail validation.
      let result = crate::check_str("{upper()}\n");
      assert!(result.is_err(), "zero args to upper() should fail validation");
      let result2 = crate::check_str("---\na: x\nb: y\n---\n{upper(a, b)}\n");
      assert!(result2.is_err(), "two args to upper() should fail validation");
  }
  ```

### MEDIUM

**No test for `sort()` stability with NaN values** - `crates/mds-core/src/builtins.rs:413-415`
**Confidence**: 80%
- Problem: `sort()` uses `partial_cmp` with `unwrap_or(Equal)` for number arrays. If a NaN value enters the array (theoretically possible through `number()` or arithmetic), the sort produces non-deterministic ordering silently. While the parser rejects NaN literals, there is no test documenting what happens if NaN reaches `sort()` at runtime.
- Fix: Add a test that either (a) shows NaN is rejected by `sort()` or (b) documents the silent Equal fallback:
  ```rust
  #[test]
  fn sort_numbers_with_nan_does_not_panic() {
      let with_nan = Value::Array(vec![Value::Number(2.0), Value::Number(f64::NAN), Value::Number(1.0)]);
      let result = call_builtin("sort", &[with_nan]);
      assert!(result.is_ok(), "sort with NaN should not panic");
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Misleading test name `parse_condition_or_higher_precedence_than_and`** - `crates/mds-core/src/parser_tests.rs:957`
**Confidence**: 85%
- Problem: The test name states "or higher precedence than and" but the actual behavior being tested is that `&&` binds tighter than `||` (i.e., AND has higher precedence than OR). The test body is correct, but the name inverts the semantics and will confuse future readers.
- Fix: Rename to `parse_condition_and_has_higher_precedence_than_or` or `parse_condition_or_has_lower_precedence_than_and`.

**No test for `@elseif` with logical operators** - `crates/mds-core/src/parser_helpers.rs:283-318`
**Confidence**: 80%
- Problem: The `parse_condition` function is used for both `@if` and `@elseif` conditions. All logical operator tests use `@if` only. While `@elseif` shares the same code path, an integration test would confirm end-to-end correctness and serve as a regression guard.
- Fix: Add one integration test:
  ```rust
  #[test]
  fn evaluate_elseif_with_logical_operator() {
      let result = crate::compile_str(
          "---\na: false\nb: true\nc: true\n---\n@if a:\nA\n@elseif b && c:\nBC\n@else:\nNO\n@end\n"
      ).unwrap();
      assert!(result.contains("BC"), "elseif with && should work, got: {result}");
  }
  ```

## Pre-existing Issues (Not Blocking)

None identified.

## Suggestions (Lower Confidence)

- **No test for nested built-in returning non-string Value to outer built-in** - `crates/mds-core/src/evaluator.rs:220` (Confidence: 70%) -- The `call_function` return type changed from `String` to `Value`, enabling typed value passing between nested calls. The `builtin_compose_join_split` test covers string-to-string composition, but there is no test for a chain like `string(length(word))` where `length` returns `Number` and `string` converts it. This would exercise the `Value`-typed nested call path with cross-type composition.

- **No test for `reverse()` with combining characters** - `crates/mds-core/src/builtins.rs:363` (Confidence: 65%) -- `reverse` uses `.chars().rev()` which reverses Unicode scalar values, not grapheme clusters. For strings with combining marks (e.g., "é" where the accent is a combining character), reversal produces "́e" (accent before the base character). This is a known limitation of `.chars().rev()` but is not tested or documented.

- **`split_on_unquoted_commas` drops empty trailing token** - `crates/mds-core/src/parser_helpers.rs:880-884` (Confidence: 60%) -- `split_on_unquoted_commas` trims and skips the final token if empty, meaning a trailing comma in a param list is silently ignored rather than producing an error. No test covers the trailing comma case (`"a, b, "`) to document this behavior.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The test suite is solid overall -- 100 new tests for 2045 lines of new/changed code is good coverage. Built-in functions have thorough unit tests with happy-path, type-error, and edge-case coverage. The UTF-8 char-boundary fix in `slice` has targeted tests. The logical operator parser tests cover precedence, error cases, and limits. Default parameter parsing tests cover all CondValue types plus ordering/duplicate constraints.

The main gaps are (1) missing documentation of the `length()` byte-vs-char contract, (2) no validator-specific tests for the new built-in arity checking path, and (3) a misleading test name. None of these gaps represent likely runtime failures, but they leave behavioral ambiguity and reduce regression safety for future changes. Fix the HIGH items before merge.

# Testing Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Missing CLI-level / end-to-end integration tests for all three new features** - `crates/mds-cli/tests/`
**Confidence**: 85%
- Problem: The PR adds ~2800 lines across 15 files implementing three major language features (built-in functions, default arguments, logical operators) with 113 new tests. However, all tests are unit-level (in-crate `mod tests`) or use `compile_str()` integration. The CLI test suite (`crates/mds-cli/tests/`) has 10 categorized test files (`language.rs`, `errors.rs`, etc.) that exercise the full pipeline through the binary, and none were updated. This means the features are not tested through the actual CLI entry point — `run_build`/`run_check` with file I/O, `--vars`, `--set`, error exit codes, and diagnostic output formatting. The existing test infrastructure in `language.rs` and `errors.rs` is the natural place for these.
- Fix: Add tests to `crates/mds-cli/tests/language.rs` exercising:
  1. Built-in function calls in `.mds` fixture files compiled via `mds build`
  2. Default arguments with `mds check` validation (valid and invalid arity)
  3. Logical operators `&&`/`||` in `@if`/`@elseif` conditions
  4. Error diagnostics for arity mismatches on builtins (verify exit code 1 and error message format)
  5. Interaction with `--vars`/`--set` overrides (e.g., `mds build template.mds --set flag=true` where flag is used in `@if flag && other:`)

**Missing negative test: `contains` on non-string/non-array first argument** - `crates/mds-core/src/builtins.rs`
**Confidence**: 82%
- Problem: The `contains()` function has a type-error branch for when the first argument is neither a string nor an array (line 272-277), but no test covers this error path. Every other dual-type builtin (`length`, `slice`, `reverse`) has a type-error test.
- Fix: Add a test:
  ```rust
  #[test]
  fn contains_requires_string_or_array() {
      let err = call_builtin("contains", &[Value::Number(1.0), s("x")]).unwrap_err();
      assert!(err.to_string().contains("string or array"));
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing test for `contains` with array element not found** - `crates/mds-core/src/builtins.rs`
**Confidence**: 85%
- Problem: Tests cover `contains_string_found`, `contains_string_not_found`, and `contains_array_found`, but there is no `contains_array_not_found` test. This is an asymmetric gap — the `false` return path for arrays is untested.
- Fix: Add:
  ```rust
  #[test]
  fn contains_array_not_found() {
      let result = call_builtin("contains", &[arr(&["a", "b"]), s("z")]).unwrap();
      assert_eq!(result, Value::Boolean(false));
  }
  ```

**Missing test for `reverse` type error path** - `crates/mds-core/src/builtins.rs`
**Confidence**: 82%
- Problem: `reverse()` has a type-error arm (lines 403-408) for non-string/non-array input, but no test covers it. Similar dual-type functions like `length` and `slice` have type-error tests.
- Fix: Add:
  ```rust
  #[test]
  fn reverse_requires_string_or_array() {
      let err = call_builtin("reverse", &[Value::Number(1.0)]).unwrap_err();
      assert!(err.to_string().contains("string or array"));
  }
  ```

**Missing test for `slice` type error path** - `crates/mds-core/src/builtins.rs`
**Confidence**: 82%
- Problem: `slice()` has a type-error arm (lines 315-320) for non-string/non-array input, but no test covers it. While `slice_infinity_index_rejected` and `slice_nan_index_rejected` test the index type-error path, the first-argument type error is untested.
- Fix: Add:
  ```rust
  #[test]
  fn slice_requires_string_or_array() {
      let err = call_builtin("slice", &[Value::Number(1.0), Value::Number(0.0)]).unwrap_err();
      assert!(err.to_string().contains("string or array"));
  }
  ```

**Missing test for `number()` with array/object input** - `crates/mds-core/src/builtins.rs`
**Confidence**: 80%
- Problem: `number()` has a catch-all error arm (lines 531-534) for array/object types, but only `number_rejects_non_numeric_string` tests the error path (for string input). The array/object rejection path is untested.
- Fix: Add:
  ```rust
  #[test]
  fn number_rejects_array() {
      let err = call_builtin("number", &[Value::Array(vec![])]).unwrap_err();
      assert!(err.to_string().contains("cannot convert"));
  }
  ```

**No test for `condvalue_to_value` conversion correctness** - `crates/mds-core/src/evaluator.rs`
**Confidence**: 80%
- Problem: `condvalue_to_value` is a new public(crate) function used in the default-argument filling path. While it is implicitly tested via `evaluate_default_param_used_when_not_provided`, there is no direct unit test verifying the conversion for all four `CondValue` variants (String, Number, Boolean, Null). Integration tests only exercise String defaults.
- Fix: Add direct tests for each variant, particularly Number, Boolean, and Null defaults:
  ```rust
  #[test]
  fn evaluate_default_param_number() {
      let result = crate::compile_str(
          "@define repeat(n = 3):\n{n}\n@end\n{repeat()}\n"
      ).unwrap();
      assert_eq!(result.trim(), "3");
  }
  
  #[test]
  fn evaluate_default_param_boolean() {
      let result = crate::compile_str(
          "@define check(flag = true):\n@if flag:\nyes\n@end\n@end\n{check()}\n"
      ).unwrap();
      assert!(result.contains("yes"));
  }
  ```

### LOW

**Duplicated arity range display tests** - `crates/mds-core/src/parser_tests.rs:731-758` and `crates/mds-core/src/error_tests.rs:282-290`
**Confidence**: 85%
- Problem: Three tests in `parser_tests.rs` (`arity_range_exact_one_argument`, `arity_range_exact_plural_arguments`, `arity_range_min_max`) test identical behavior to tests in `error_tests.rs` (`arity_display_singular_argument`, `arity_display_plural_arguments`, `arity_display_range`). Both sets construct `MdsError::arity(...)` and assert on the display string.
- Fix: Remove the duplicates from `parser_tests.rs` — arity display formatting belongs in `error_tests.rs`. Or consolidate into one location with a comment explaining the test belongs to the error module.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing test for `sort()` with boolean arrays** - `crates/mds-core/src/builtins.rs` (Confidence: 70%) — `sort()` rejects non-string/non-number arrays at lines 465-468. Sorting an array of booleans would hit this path, but no test covers it. Low risk since the path is straightforward, but it would complete the type-rejection coverage.

- **Missing test for `And`/`Or` with empty operand vector** - `crates/mds-core/src/evaluator.rs` (Confidence: 65%) — The evaluator handles `Condition::And(operands)` and `Condition::Or(operands)` by iterating the operands vec. An empty operands vec would make `And` return `true` and `Or` return `false` (vacuous truth / falsity). The parser prevents this, but a direct unit test would document the edge-case behavior.

- **No test for `slice` negative index clamping** - `crates/mds-core/src/builtins.rs` (Confidence: 65%) — The spec says negative indices clamp to 0, and the code does `n.max(0.0)`. While `slice_clamps_to_bounds` tests a large positive index, there is no test specifically for a negative index like `slice("hello", -5)` to verify the clamping.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | 0 |
| Should Fix | 0 | 0 | 5 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR adds an impressive 113 new tests (up from ~590 to 703 total), covering all 18 built-in functions, the new Arg variants, default parameter parsing, logical operator precedence, and integration through `compile_str`. The test quality is high: tests follow AAA structure, use behavior-focused assertions, and cover important edge cases (Unicode, NaN, infinity, empty arrays, type errors, resource limits). The `unique_large_array_completes_in_linear_time` test is a good performance regression guard.

The primary gap is the absence of CLI-level end-to-end tests (HIGH), which is significant because the existing test suite has a dedicated CLI integration test layer that exercises file I/O, exit codes, and diagnostic formatting. The secondary gaps are a handful of missing error-path unit tests for type-error branches in dual-type builtins (`contains`, `reverse`, `slice`, `number`). These are individually low-risk but collectively represent incomplete branch coverage on the new builtins module. Applies ADR-008 (bundled feature delivery).

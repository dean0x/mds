# Testing Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19
**Scope**: Incremental review (420e2259...HEAD, 7 commits)

## Issues in Your Changes (BLOCKING)

### HIGH

**No test for `check_source_size` / resource limit at the WASM boundary** - `crates/mds-wasm/src/lib.rs:309-321`
**Confidence**: 95%
- Problem: The `check_source_size()` guard was introduced in this diff and is called at the top of both `compile()` and `check()`. However, there are zero `wasm_bindgen_test` tests exercising this path. The `load_vars_str` size guard in `mds-core` has corresponding tests (`load_vars_str_rejects_oversized_input`, `load_vars_str_accepts_valid_json_within_limit`), but the WASM-boundary equivalent does not. If `check_source_size` were accidentally removed or its error code changed, no test would catch it.
- Fix: Add at least one test that passes a source exceeding `MAX_SOURCE_SIZE` and asserts `mds::resource_limit` code. A practical approach for WASM tests (since allocating 10 MiB in wasm-pack is slow) is a unit-test-style assertion that `MAX_SOURCE_SIZE` equals `mds::MAX_FILE_SIZE as usize`, plus a boundary test with a reasonable mock:
```rust
#[wasm_bindgen_test]
fn compile_oversized_source_returns_resource_limit() {
    // Use a source just over the limit. In practice this test is expensive
    // for 10 MiB, so if infeasible, at least verify the constant derivation.
    // For a lighter alternative, test with a source of known size and assert
    // the error code format.
    let big = "x".repeat(mds_wasm::MAX_SOURCE_SIZE_FOR_TEST + 1); // needs a test-visible const
    let err = mds_wasm::compile(&big, JsValue::NULL).unwrap_err();
    let code = get_str(&err, "code");
    assert_eq!(code, "mds::resource_limit");
}
```
If exposing the constant is undesirable, a simpler approach is an integration-level assertion that `compile("x".repeat(11_000_000), NULL)` returns the expected error code.

**No test for `catch_panic` / `mds::internal` error path** - `crates/mds-wasm/src/lib.rs:135-156`
**Confidence**: 85%
- Problem: The `catch_panic` wrapper converts Rust panics into JS `Error` objects with `code = "mds::internal"` and a `detail` property. This is a critical safety boundary for the WASM module. There is no test verifying that panics produce the expected error shape. If the panic-handling logic regressed (e.g., the `detail` property attachment broke), callers would get unhelpful errors.
- Fix: Testing panics in `wasm_bindgen_test` is non-trivial since you cannot easily trigger a panic through the public API. Options include:
  1. A `#[cfg(test)]` public function that deliberately panics, exercised by a WASM test.
  2. Documenting this as a known gap with a tracking issue.
  At minimum, a comment in the test file acknowledging the gap ensures future reviewers know it is intentional.

### MEDIUM

**Span assertions rely on duplicated magic offsets** - `crates/mds-wasm/tests/web.rs:164-194`
**Confidence**: 82%
- Problem: `compile_error_has_span_with_offset_and_length` tests that `offset >= 0` and `length > 0`, but the comment on line 166 says the span starts at byte 6 without asserting it. The test only checks `>= 0`, which would pass even for an incorrect offset. This is a weak assertion for a test whose comment indicates the expected value is known.
- Fix: Assert the exact expected values:
```rust
assert_eq!(
    offset.as_f64().unwrap() as usize,
    6,
    "span.offset should point at the opening brace of {undefined_var}"
);
assert_eq!(
    length.as_f64().unwrap() as usize,
    15, // "{undefined_var}".len()
    "span.length should cover the variable reference"
);
```
This tightens the test to catch offset/length regressions that the current `>= 0` / `> 0` checks would miss.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Repeated error-triggering input across 6 tests without extraction** - `crates/mds-wasm/tests/web.rs:139-240`
**Confidence**: 80%
- Problem: The string `"Hello {undefined_var}!\n"` is used as the error-triggering input in 6 separate tests: `compile_undefined_variable_returns_error`, `compile_error_has_code_property`, `compile_error_has_span_with_offset_and_length`, `compile_error_span_has_line_and_column`, `compile_error_has_help_for_undefined_variable`, and `compile_error_is_js_error` (which uses `"{undefined}\n"` instead, a slightly different input). Each test independently calls `mds_wasm::compile` with nearly identical setup. If the error-triggering input needs to change, all 6 locations need updating.
- Fix: Extract a constant or helper:
```rust
const UNDEFINED_VAR_SOURCE: &str = "Hello {undefined_var}!\n";

fn compile_error_for_undefined_var() -> JsValue {
    mds_wasm::compile(UNDEFINED_VAR_SOURCE, JsValue::NULL).unwrap_err()
}
```
This reduces duplication and makes the offset/length assertions self-documenting (the constant is the single source of truth for what byte 6 means).

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`compile_error_is_js_error` uses a different input from other error tests** - `crates/mds-wasm/tests/web.rs:153-161`
**Confidence**: 80%
- Problem: This test uses `"{undefined}\n"` while all other error property tests use `"Hello {undefined_var}!\n"`. The different input is not harmful, but it obscures the intent: it is unclear whether the difference is deliberate (testing a different error variant) or accidental. The test name suggests it should verify error shape, not a specific error type.
- Fix: Align to use the same input as the other error tests, or add a comment explaining why a different input was chosen.

## Suggestions (Lower Confidence)

- **Missing `check()` error property parity tests** - `crates/mds-wasm/tests/web.rs` (Confidence: 70%) -- The `compile()` error path has dedicated tests for `span`, `help`, and `code` properties, but `check()` only tests `code`. Since `check()` uses the same `mds_error_to_js` converter, this is low risk, but parity tests would guard against future divergence if the paths split.

- **No negative test for non-object options** - `crates/mds-wasm/tests/web.rs` (Confidence: 65%) -- `parse_options` rejects non-object inputs (e.g., passing a number or string as options), but no test covers this path. The `compile_invalid_vars_type_returns_error` test covers a type error within the options object, but not the top-level type check.

- **`load_vars_str_rejects_oversized_input` allocates 10 MiB in every test run** - `crates/mds-core/src/lib.rs:883-894` (Confidence: 65%) -- The `"x".repeat((MAX_FILE_SIZE as usize) + 1)` allocates over 10 MiB. This is acceptable for a unit test but may slow down the test suite. Consider using `#[ignore]` if it becomes an issue, or testing with a smaller custom limit for the unit test.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The new tests are well-structured, follow clear Arrange-Act-Assert patterns, and significantly improve coverage of the WASM boundary (span properties, help text, check() parity, dependency content verification). Test names describe expected behavior and the helper functions (`get_prop`, `get_str`, `vars_opts`, `modules_opts`, `filename_opts`) keep setup concise. The two HIGH findings (no test for the resource-limit guard and no test for the panic-catch boundary) represent untested safety-critical paths introduced in this diff. The span assertion weakness (MEDIUM) is a matter of tightening an existing test to match its documented intent. None of these issues indicate architectural problems -- they are straightforward coverage gaps.

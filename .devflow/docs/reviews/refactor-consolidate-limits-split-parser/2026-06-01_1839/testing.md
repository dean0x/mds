# Testing Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Extracted helper functions lack direct unit tests** - `parser_helpers.rs` (Confidence: 65%) -- Several extracted `pub(super)` functions (`parse_dot_path`, `parse_negation_condition`, `parse_quoted_path`, `parse_for_vars`, `validate_dot_path_parts`, `is_directive_token`, `strip_leading_newline`, `strip_trailing_newline`, `unescape_string`) are only tested indirectly through integration-level parser tests. This is acceptable given the PR description states "no behavioral changes" and all 48 tests transferred intact, but adding targeted unit tests for these helpers in a follow-up would improve fault isolation.

- **Pinning test in limits.rs duplicates literal values** - `crates/mds-core/src/limits.rs:41-46` (Confidence: 60%) -- The `limits_have_expected_values` test asserts each constant equals a hardcoded literal (e.g., `assert_eq!(MAX_DOT_SEGMENTS, 32)`). This guards against accidental value changes but is purely a snapshot test -- it will fail any time a limit is intentionally adjusted, requiring a mechanical update. This is a minor style observation; pinning tests are a valid pattern for safety-critical constants.

- **`parse_single_arg` test helper under `#[cfg(test)]` in non-test file** - `crates/mds-core/src/parser_helpers.rs:609-612` (Confidence: 62%) -- The `parse_single_arg` convenience wrapper is gated with `#[cfg(test)]` inside the helpers module rather than living in the test module itself. This is functional (the gate compiles it out in release), but placing test-only helpers in the test file is the more conventional Rust idiom.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Testing Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This is a clean structural refactor that splits `parser.rs` (1820 lines) into three focused files (`parser.rs` ~423 lines, `parser_helpers.rs` 733 lines, `parser_tests.rs` 668 lines) and consolidates 5 cross-module constants into `limits.rs`. From a testing perspective:

1. **Test count preserved**: All 48 parser tests transferred 1:1 from the old monolithic `parser.rs` to the new `parser_tests.rs` with identical function names and bodies. No tests were dropped, renamed, or weakened.

2. **New pinning test added**: `limits.rs` adds a `limits_have_expected_values` test that pins all 5 consolidated constants, providing regression protection against accidental value drift.

3. **Total test count increased**: 591 tests pass (vs. 590+ baseline from CLAUDE.md), confirming the new limits pinning test adds coverage.

4. **Module wiring is correct**: `parser_tests.rs` uses `#[path = "parser_tests.rs"] mod tests` under `#[cfg(test)]`, and imports `super::helpers::*` and `super::*`, giving tests access to both the parser struct methods and the extracted helper functions.

5. **Test helper properly gated**: `parse_single_arg` (a convenience wrapper used only by tests) is correctly `#[cfg(test)]`-gated in `parser_helpers.rs`, preventing dead code in release builds.

6. **No behavioral changes**: The PR description states no behavioral changes, and the 1:1 test preservation plus 591/591 pass rate confirms this. The only API signature change (`parse_export_directive` dropping unused `_offset` parameter) is internal (`pub(super)`) and the call site in `parser.rs:197` was updated accordingly.

Applies ADR-001 (squash merge with pre-merge gate): all tests pass, confirming quality gate readiness. Applies ADR-002 (verify PR content addresses linked issues): the test evidence confirms the refactor is purely structural with no behavioral regression, consistent with #35 (consolidate constants) and #36 (split parser).

# Rust Review Report

**Branch**: refactor/consolidate-limits-split-parser -> main
**Date**: 2026-06-01

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Missing doc comments on 4 `pub(super)`/`pub(crate)` functions** -- `parser_helpers.rs:556`, `parser_helpers.rs:631`, `parser_helpers.rs:635`, `parser_helpers.rs:714`
**Confidence**: 82%
- Problem: `parse_args_inner`, `parse_single_arg`, `parse_single_arg_inner`, and `is_valid_identifier` lack `///` doc comments, while every other public function in the same file has them. This creates an inconsistency within the newly extracted module and makes the API harder to navigate.
- Fix: Add brief `///` doc comments to each. For example:
  ```rust
  /// Recursively parse function call arguments with depth tracking.
  pub(super) fn parse_args_inner(...) { ... }

  /// Parse a single argument string (test-only convenience wrapper).
  #[cfg(test)]
  pub(super) fn parse_single_arg(...) { ... }

  /// Parse a single argument string with depth tracking for nested calls.
  pub(super) fn parse_single_arg_inner(...) { ... }

  /// Return true if `s` is a valid ASCII identifier (leading alpha/underscore, then alphanumeric/underscore).
  pub(crate) fn is_valid_identifier(...) { ... }
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`.expect()` in `collect_elseif_branches`** - `parser.rs:296` (Confidence: 65%) -- The `.expect("loop guard guarantees @elseif prefix")` is logically safe since the while-loop guard already checked `starts_with("@elseif ")`, but using `.unwrap_or_default()` or propagating with `?` after mapping to `MdsError` would avoid a panic path in library code entirely. This was classified as a false positive in the prior review cycle and the guard is solid, so this is purely a stylistic preference.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This is a well-executed refactoring PR. The key observations (applies ADR-001 -- clean, reviewable changes suitable for squash merge):

1. **Constant consolidation is correct.** All 5 constants (`MAX_NESTING_DEPTH`, `MAX_ELSEIF_BRANCHES`, `MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`, `MAX_DOT_SEGMENTS`) now live in a single `limits.rs` module with `pub(crate)` visibility. The re-exports in `lib.rs` maintain backward compatibility for downstream consumers. Import paths in `evaluator.rs`, `fs.rs`, `parser.rs`, and `validator.rs` are all updated consistently.

2. **Module split is clean.** `parser.rs` went from ~1820 lines to ~423 lines. The `#[path = "parser_helpers.rs"]` and `#[path = "parser_tests.rs"]` module declarations follow the established `error.rs` precedent in this codebase. Visibility is well-chosen: `pub(super)` for helpers consumed only by the parser module, `pub(crate)` for `is_valid_identifier` which has callers across crates.

3. **No behavioral changes.** The refactoring is purely structural -- all 591 tests pass, clippy is clean with zero warnings, and no logic was altered. The `parse_export_directive` signature cleanup (removing unused `_offset` parameter) is a minor, correct simplification.

4. **Ownership and borrowing patterns are correct.** Functions consistently borrow `&str` instead of taking `String` ownership. No unnecessary `.clone()` calls were introduced. `Vec::with_capacity(4)` pre-allocation in hot paths like `parse_dot_path` and `collect_elseif_branches` is appropriate.

5. **Error handling is consistent.** All fallible functions return `Result<T, MdsError>` with the `?` operator for propagation. No new `.unwrap()` calls in non-test code. The single `.expect()` at line 296 has a comment documenting the invariant.

6. **Pinning tests in `limits.rs`** provide a safety net against accidental constant value changes -- good practice for security-relevant limits.

7. **Test code quality is solid.** Tests use `parse_with_ctx` consistently, test both boundary conditions (at limit, above limit), and have clear assertion messages. The `#[cfg(test)]` guard on `parse_single_arg` correctly scopes a test-only helper.

The only blocking condition is the missing doc comments on 4 functions -- a minor consistency fix. Everything else about this refactoring is clean and production-ready.

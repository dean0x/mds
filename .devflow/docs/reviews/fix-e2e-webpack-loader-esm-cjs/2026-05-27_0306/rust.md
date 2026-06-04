# Rust Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**Files reviewed**: 7 Rust files (+990 / -85 lines)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Stale comment: MAX_ELSEIF_BRANCHES says "matches MAX_NESTING_DEPTH" but values diverge (256 vs 64)** - `crates/mds-core/src/ast.rs:9`
**Confidence**: 95%
- Problem: The comment on `MAX_ELSEIF_BRANCHES` reads "Matches MAX_NESTING_DEPTH to prevent pathological chains" but `MAX_ELSEIF_BRANCHES` is 256 while `MAX_NESTING_DEPTH` was lowered to 64 in this PR. The comment is factually wrong and will mislead future maintainers into believing these values are kept in sync.
- Fix: Either update the comment to explain why the values differ, or change `MAX_ELSEIF_BRANCHES` to 64 (or some intentional value). Suggested comment:
  ```rust
  /// Maximum number of @elseif branches on a single @if block.
  /// 256 is generous for real templates while bounding pathological chains.
  pub const MAX_ELSEIF_BRANCHES: usize = 256;
  ```

### MEDIUM

**`#[must_use]` missing on `Condition::root()` which returns `Result`** - `crates/mds-core/src/ast.rs:53`
**Confidence**: 82%
- Problem: Per Rust API Guidelines [C-MUST-USE], public functions returning `Result` should be annotated with `#[must_use]` to warn callers who forget to propagate the error. All current call sites use `?`, but future callers could silently discard the `Result`.
- Fix:
  ```rust
  #[must_use]
  pub fn root(&self) -> Result<&str, crate::error::MdsError> {
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`elseif_branches` limit check runs after parsing the branch body** - `crates/mds-core/src/parser.rs:273`
**Confidence**: 80%
- Problem: The limit check `if elseif_branches.len() >= MAX_ELSEIF_BRANCHES` runs after `parse_body` for the exceeding branch (line 271), meaning the parser does full body parsing on one branch that will be rejected. For adversarial input with 257 massive `@elseif` bodies, this wastes parse time unnecessarily.
- Fix: Move the length check before parsing the body:
  ```rust
  // Check limit before parsing body
  if elseif_branches.len() >= MAX_ELSEIF_BRANCHES {
      return Err(MdsError::syntax(format!(
          "@if block has more than {MAX_ELSEIF_BRANCHES} @elseif branches"
      )));
  }
  let elseif_body = self.parse_body(&["@else:", "@end"], &["@elseif "])?;
  elseif_branches.push((elseif_cond, elseif_body));
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing `PartialEq` derive on `Condition` enum** - `crates/mds-core/src/ast.rs:29` (Confidence: 65%) -- `CondValue` derives `PartialEq` but `Condition` does not. While no code currently compares `Condition` values, the inconsistency may block future test assertions or structural comparisons. All inner types support `PartialEq`.

- **`parse_condition` bare `=` check uses naive `s.find('=')` not quote-aware** - `crates/mds-core/src/parser.rs:609` (Confidence: 62%) -- The bare `=` check on line 609 uses `s.find('=')` which is not quote-aware (unlike `find_unquoted_operator`). Currently safe because this branch only runs for truthy-path conditions where no quotes are expected, but fragile if the condition grammar expands. Consider using a quote-aware scan or adding a comment documenting the invariant.

- **`MAX_ELSEIF_BRANCHES` at 256 is generous given `MAX_NESTING_DEPTH` is 64** - `crates/mds-core/src/ast.rs:10` (Confidence: 70%) -- 256 `@elseif` branches on a single `@if` allows far more complexity than 64 nesting levels. While not a bug, the asymmetry may warrant intentional alignment or a documented rationale for the difference.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Positive Observations

- **Type-driven design**: The `Condition` enum with `Truthy`, `Not`, `Eq`, `NotEq` variants makes illegal states unrepresentable -- invalid condition combinations are rejected at parse time, not runtime.
- **Centralized invariant**: `Condition::root()` eliminates the duplicated empty-path error message from evaluator.rs and validator.rs into a single canonical source.
- **Correct error handling**: All fallible operations return `Result` with `?` propagation. No `.unwrap()` in production code.
- **Safe byte-level scanning**: `find_unquoted_operator` correctly operates on ASCII bytes within UTF-8 strings, with proper backslash escape handling.
- **NaN/Infinity rejection**: `parse_cond_value` uses `n.is_finite()` to reject NaN and Infinity at parse time, preventing downstream IEEE 754 comparison surprises.
- **Strict equality semantics**: `values_equal` implements type-safe comparison with no coercion, and the doc comment explicitly documents NaN behavior.
- **Comprehensive test coverage**: New features have thorough test coverage including edge cases (empty strings, null equality, cross-type non-coercion, operator-in-string).

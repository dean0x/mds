# Complexity Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicated quote/paren scanning state machines (5 occurrences)** -- Confidence: 88%
- `parser_helpers.rs:30-85` (`strip_trailing_directive_colon`)
- `parser_helpers.rs:93-115` (`has_unterminated_string`)
- `parser_helpers.rs:328-390` (`find_unquoted_operator`)
- `parser_helpers.rs:439-489` (`split_on_unquoted_op`)
- `parser_helpers.rs:607-645` (inline bare-`=` scanner in `parse_simple_condition`)
- Problem: Five separate functions implement nearly identical byte-level state machines that track `in_string`, `string_char`, `paren_depth` with identical escape handling (`\\` skips next byte, matching close-quote toggles `in_string`). While three of these existed before this PR, this PR **added** `strip_trailing_directive_colon`, `has_unterminated_string`, and the inline `paren_depth` tracking to all scanners. The total `in_string` scanner pattern now appears 8 times in the file. Each instance is ~20-40 lines of near-identical boilerplate.
- Impact: Any future change to scanning semantics (e.g., supporting backtick strings, nested quotes, or bracket expressions) must be replicated in all 5+ locations. A bug fix in one scanner that is missed in another creates subtle parsing inconsistencies. This is the primary maintainability risk in this PR. `applies ADR-008` -- bundling features is fine but the resulting code should consolidate shared patterns.
- Fix: Extract a reusable `ScanState` struct or a `scan_outside_quotes_and_parens` iterator/callback that encapsulates the `in_string`/`string_char`/`paren_depth`/escape logic. Each consumer provides only its specific matching logic:
  ```rust
  struct ScanState { in_string: bool, string_char: u8, paren_depth: usize }
  impl ScanState {
      fn advance(&mut self, ch: u8, next: Option<u8>) -> bool { /* returns skip_next */ }
      fn is_bare(&self) -> bool { !self.in_string && self.paren_depth == 0 }
  }
  ```
  Then `find_unquoted_operator`, `split_on_unquoted_op`, `strip_trailing_directive_colon`, and the inline `=` scanner all use `ScanState::advance` + `ScanState::is_bare()`, reducing each to 5-10 lines of unique logic.

### MEDIUM

**`parse_simple_condition` function complexity** -- `parser_helpers.rs:569-662` -- Confidence: 85%
- Problem: This function is 94 lines with a cyclomatic complexity of approximately 12 (negation check, operator split, empty LHS, empty RHS, operator match, inline byte-level `=` scanner with its own `while` loop containing `match` with 5 arms including nested conditions, then literal rejection `match` with 4 arms). The inline bare-`=` scanner block (lines 607-645) alone is 38 lines of nested `while`/`match`/`if` logic, reaching nesting depth 4.
- Impact: New contributors will struggle to understand the function's flow. The inline scanner is particularly hard to follow because it duplicates the same pattern used in `find_unquoted_operator` but with a different check target.
- Fix: Extract the bare-`=` detection into a named helper function `has_bare_assignment_operator(s: &str) -> bool` that reuses the proposed `ScanState`. This would reduce `parse_simple_condition` to ~55 lines and bring nesting depth to 2.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`validate_for_node` duplicates `validate_expr` logic for `QualifiedCall`** -- `validator.rs:89-193` -- Confidence: 82%
- Problem: The `Expr::QualifiedCall` arm in `validate_for_node` (lines 146-172) is nearly identical to the `Expr::QualifiedCall` arm in `validate_expr` (lines 327-354): both look up the namespace, resolve the function, check arity, and validate args. The `Expr::Call` arm (lines 134-144) is simpler but still duplicates `validate_expr`'s Call arm. The function is 105 lines overall.
- Impact: If validation logic changes (e.g., new error messages, new checks), it must be updated in both places. This is a maintenance trap.
- Fix: For the non-`Var` cases, delegate to `validate_expr` directly:
  ```rust
  match &block.iterable {
      Expr::Var(root) => { /* keep the Var-specific static type check logic */ }
      other => {
          let len = match other {
              Expr::Call { name, .. } => name.len(),
              Expr::QualifiedCall { name, .. } => name.len(),
              Expr::MemberAccess { object, .. } => object.len(),
              _ => 0,
          };
          validate_expr(other, scope, file, source, block.offset, len)?;
      }
  }
  ```
  This would cut `validate_for_node` from 105 lines to ~50 and eliminate the duplication.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parser_helpers.rs` file length at 1373 lines** -- Confidence: 80%
- Problem: The file exceeds the 500-line critical threshold significantly. It contains 8 distinct scanner state machines, condition parsing, expression parsing, interpolation parsing, import/export parsing, and string utilities. This PR added ~300 lines of new code, pushing an already-large file further.
- Impact: Orientation cost for new contributors. Finding the right function to modify requires scanning through 1300+ lines of mixed concerns.
- Note: This is informational. The file was already large before this PR. A future refactoring could split it into `scan.rs` (scanner state machine), `expr_parse.rs` (expression parsing), and `directive_parse.rs` (condition/import/export parsing).

## Suggestions (Lower Confidence)

- **`parse_expr_inner` partially duplicates `parse_interpolation_expr`** -- `parser_helpers.rs:128-246` (Confidence: 70%) -- Both functions parse expressions with similar dispatch (dot/paren positions), but `parse_expr_inner` adds literal recognition. A shared core with an `allow_literals: bool` flag could unify them, though the two functions serve different entry points and have different return types (`Expr` vs `Interpolation`).

- **`evaluate_condition_value` trivial wrapper may be dead code** -- `evaluator.rs` diff (Confidence: 65%) -- The diff showed addition of an `evaluate_condition_value` wrapper that just delegates to `evaluate_expr`, but the final code calls `evaluate_expr` directly from `evaluate_condition`. If the wrapper exists, it adds a needless indirection layer. If it was removed during development, this is a non-issue.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The core complexity concern is the proliferated quote/paren scanning state machine pattern (5 near-identical implementations in `parser_helpers.rs`). While each individual function is well-commented and bounded, the maintenance cost of keeping them synchronized is HIGH. Extracting a shared `ScanState` abstraction would materially improve maintainability without changing any behavior. The `validate_for_node` duplication is a secondary concern that could be addressed in the same pass. The feature logic itself (expression support in directives) is well-structured -- the complexity issues are in the supporting infrastructure, not the feature design.

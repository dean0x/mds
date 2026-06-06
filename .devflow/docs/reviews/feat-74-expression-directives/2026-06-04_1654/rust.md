# Rust Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Stale doc comment references CondValue in Condition's PartialEq rationale** - `crates/mds-core/src/ast.rs:37`
**Confidence**: 85%
- Problem: The doc comment on `Condition` says "Condition intentionally does not derive PartialEq even though CondValue does." This was accurate before the PR, when `Condition` variants held `Vec<String>` and `CondValue`. After this PR, `Condition` holds `Expr` (not `CondValue`). `Expr` does NOT derive `PartialEq`, so the "even though CondValue does" clause is misleading -- `CondValue` is no longer a constituent type of `Condition`. The NaN rationale on line 38 (`Expr::NumberLiteral(f64)`) is correct, but the framing implies `CondValue` is still relevant to `Condition`.
- Fix: Update the doc comment to remove the `CondValue` reference since `Condition` no longer contains it:
```rust
/// `Condition` intentionally does **not** derive `PartialEq`.
/// `Expr::NumberLiteral(f64)` uses IEEE 754 semantics where `NaN != NaN`, so
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Trivial indirection in evaluate_condition_value** - (Confidence: 65%) — The diff initially showed an `evaluate_condition_value` wrapper around `evaluate_expr`. In the final code, the `Truthy`/`Not` arms call `evaluate_expr` directly (lines 420-421), while `Eq`/`NotEq` also call `evaluate_expr` directly (lines 422-430). This is clean — no unnecessary indirection remains. No action needed; this is just confirming the final state is correct.

- **Repeated quote/paren scanning logic across 4 functions** - `parser_helpers.rs` (Confidence: 70%) — The byte-level scanning pattern (track `in_string`, `string_char`, `paren_depth`, handle escape sequences) is duplicated across `strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op`, and the inline bare-`=` check in `parse_simple_condition`. Each instance handles the same state machine. A shared `ScanState` helper could consolidate the scanning logic, reducing the risk of divergent behavior if one copy gets a bug fix that the others miss. However, each scanner has slightly different actions at each byte, so consolidation is not trivial and the current code is correct.

- **MAX_ARRAY_ELEMENTS not enforced for user-defined functions returning arrays** - `crates/mds-core/src/builtins.rs:263` (Confidence: 60%) — The `split()` builtin guards against producing arrays exceeding `MAX_ARRAY_ELEMENTS`, but user-defined functions or other builtins (e.g., future array-producing builtins) do not have this guard. This is acceptable for now since `split()` is the only builtin that produces arrays, but the pattern should be documented as a convention for future builtin authors.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### What's Done Well

- **Type-driven design** (applies ADR-008): The `Condition` enum now holds `Expr` instead of `Vec<String>`, unifying the expression model across interpolation, conditions, and iterables. This eliminates the impedance mismatch between the parser's literal types (`CondValue`) and the evaluator's runtime types (`Value`). The `Expr` literal variants (`StringLiteral`, `NumberLiteral`, `BooleanLiteral`, `NullLiteral`) cleanly extend the AST.

- **Exhaustive match enforcement**: Every `match` on `Expr` handles all variants, including the new literal variants. The validator's `validate_for_node` and `validate_condition_expr` both cover all `Expr` variants with appropriate behavior (literals need no validation, complex expressions delegate to scope lookups).

- **Result types throughout**: The signature change from `evaluate_expr -> Result<String>` to `evaluate_expr -> Result<Value>` with a separate `render_expr` wrapper is architecturally clean. It separates "evaluate to a value" from "render a value to a string for interpolation", which is exactly the right separation for supporting expressions in directives.

- **Defense-in-depth**: The `MAX_ARRAY_ELEMENTS` guard on `split()` and the `MAX_OUTPUT_SIZE` guard on `join()` prevent amplification attacks through the new expression-in-directive surface area. The `strip_trailing_directive_colon` function correctly handles colons inside string arguments, preventing directive-colon confusion.

- **Backward compatibility**: Existing `@if var:` and `@for item in items:` syntax continues to work unchanged -- simple variables are parsed as `Expr::Var`, dot-paths as `Expr::MemberAccess`. The test suite includes explicit backward-compatibility tests.

- **f64 in enum**: The PR description correctly notes that `Expr` cannot derive `PartialEq` due to `f64`. This is handled correctly -- `Expr` derives only `Debug, Clone`, and the evaluator uses `values_equal_runtime` with explicit IEEE 754 NaN handling.

### Condition

The single MEDIUM finding (stale doc comment) should be fixed before merge -- it's a one-line change that prevents future confusion about the `Condition`/`CondValue`/`Expr` relationship.

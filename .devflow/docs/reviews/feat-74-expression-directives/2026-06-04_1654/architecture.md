# Architecture Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicated type representation: `CondValue` and `Expr` literal variants** - `crates/mds-core/src/ast.rs:12-31`, `crates/mds-core/src/ast.rs:116-123`
**Confidence**: 85%
- Problem: The PR adds `StringLiteral`, `NumberLiteral`, `BooleanLiteral`, and `NullLiteral` variants to `Expr` — which are semantically identical to the existing `CondValue::String`, `CondValue::Number`, `CondValue::Boolean`, and `CondValue::Null` variants. `CondValue` is still used for `Param.default` and has its own `parse_cond_value` parser. There is also a `condvalue_to_value` conversion function in the evaluator (line 255). This creates two parallel type hierarchies for the same concept (literal values), with separate parsing paths (`parse_cond_value` vs literal branches in `parse_expr_inner`), separate conversion functions (`condvalue_to_value` vs `evaluate_expr` literal arms), and no compile-time guarantee they stay in sync. This violates SRP: two types sharing one reason to change (literal representation).
- Fix: Unify by removing `CondValue` and using `Expr` literal variants throughout. Change `Param.default` from `Option<CondValue>` to `Option<Expr>` (restricting to literal variants at parse time). Remove `condvalue_to_value` and `parse_cond_value`, reusing the `evaluate_expr` and `parse_expr_inner` literal paths. This eliminates the parallel hierarchy entirely. Applies ADR-008 (bundle related features touching the same compiler layers).

### MEDIUM

**Duplicated byte-scanning state machine across 5+ call sites** - `crates/mds-core/src/parser_helpers.rs:37-75`, `95-114`, `332-393`, `442-492`, `609-647`
**Confidence**: 85%
- Problem: The `in_string` / `string_char` / `paren_depth` byte-scanning pattern is duplicated at least 5 times in `parser_helpers.rs` (`strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op`, and the bare `=` detection in `parse_simple_condition`). Each copy manually tracks whether it is inside a quoted string and at what paren nesting depth. If the quoting rules ever change (e.g. adding backtick strings or bracket notation), every copy must be updated in lockstep — a classic SRP violation. The new code adds or extends 3 of these 5 sites.
- Fix: Extract a reusable `QuotedScanner` or `TokenIterator` struct that yields `(index, byte, is_quoted, paren_depth)` tuples. Each call site would consume this iterator and apply its specific logic (find operator, find colon, split on operator, etc.). This consolidates the quoting/paren rules in one place while keeping each consumer focused on its domain concern.

**`parse_expr_inner` duplicates logic from `parse_interpolation_expr` / `parse_dot_expr`** - `crates/mds-core/src/parser_helpers.rs:128-246`, `833-901`, `910-960`
**Confidence**: 82%
- Problem: The new `parse_expr_inner` function (128 lines) reimplements the `Var`, `Call`, `QualifiedCall`, and `MemberAccess` parsing logic that already exists in `parse_interpolation_expr` and `parse_dot_expr`. The dot-vs-paren dispatch order, identifier validation, `strip_suffix(')')` pattern, and `validate_dot_path_parts` calls are structurally identical between the two code paths. The only difference is that `parse_expr_inner` additionally handles literal values and returns `Expr` directly rather than wrapping in `Interpolation`. Two parallel parsing implementations for the same expression grammar increases the risk of divergence if the grammar evolves (e.g. adding method chaining or index access).
- Fix: Refactor `parse_interpolation_expr` to call `parse_expr_inner` for the core expression parsing, then wrap the result in `Interpolation { expr, offset, len }`. This makes `parse_expr_inner` the single canonical expression parser with `parse_interpolation_expr` as a thin adapter that adds span information. The literal variants would naturally be rejected by the interpolation path (objects cannot be interpolated), maintaining the existing error behavior.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`evaluate_condition` signature change broadens mutability requirement** - `crates/mds-core/src/evaluator.rs:414-418`
**Confidence**: 80%
- Problem: The old `evaluate_condition` took `scope: &Scope` (immutable borrow). The new version takes `scope: &mut Scope` and `ctx: &mut EvalContext` because expressions in conditions can now invoke functions (which push/pop call frames and scope layers). While this is functionally necessary, it means conditions are no longer side-effect-free from the evaluator's perspective. A condition expression like `@if func_that_sets_var(x):` could modify scope as a side effect. The architecture does not distinguish between pure expressions (suitable for conditions) and effectful ones.
- Fix: This is an accepted architectural trade-off for this feature — the existing function call infrastructure already manages scope push/pop correctly, and Condition::And/Or short-circuit evaluation means some side effects may not execute (consistent with most languages). Document this explicitly in the `evaluate_condition` doc comment: "Note: condition expressions may invoke functions that modify scope (push/pop call frames). Short-circuit evaluation means not all operands may be evaluated."

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`CondValue` doc comment drift** - `crates/mds-core/src/ast.rs:37-38` (Confidence: 65%) — The comment on `Condition`'s `PartialEq` rationale still references `CondValue` even though conditions now hold `Expr` instead. The comment was partially updated but still opens with "does not derive PartialEq even though CondValue does" which is misleading since `CondValue` is no longer part of `Condition`.

- **`strip_suffix(')')` approach in `parse_expr_inner` does not handle strings containing `)` at the end** - `crates/mds-core/src/parser_helpers.rs:189` (Confidence: 65%) — For expressions like `func(")")`, the `.strip_suffix(')')` would strip the closing paren of the string literal rather than the function call's closing paren. This is the same pattern used in `parse_interpolation_expr` (pre-existing), but the new code expands its surface area to directive contexts where colons and quotes interact with the tokenizer differently.

- **`MAX_ARRAY_ELEMENTS` constant placement** - `crates/mds-core/src/limits.rs:53` (Confidence: 70%) — The new `MAX_ARRAY_ELEMENTS = 100_000` constant is only used by `split()` in `builtins.rs`. If other builtins or user-defined functions can produce arrays (e.g. via future `range()` or `repeat()` builtins), the guard should be in the array construction path rather than individual builtins. Consider whether this belongs as a check in `Value::Array` construction or in the `@for` evaluator's iteration loop.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The core architectural decision — unifying `Condition` variants to hold `Expr` rather than `Vec<String>` — is sound and follows the Open-Closed Principle well: the expression grammar is now extensible without modifying condition handling. The evaluator refactoring (splitting `evaluate_expr` into `Value`-returning core and `render_expr` string adapter) is a clean separation of concerns.

The primary concern is the dual type hierarchy (`CondValue` vs `Expr` literal variants) which should be unified to avoid parallel maintenance burden. The duplicated byte-scanning state machines and the parallel expression parsing paths (`parse_expr_inner` vs `parse_interpolation_expr`) are structural debt that this PR expands — addressing them would reduce the maintenance surface and make future grammar extensions safer.

# Architecture Review Report

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04
**PR**: #76

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`Arg` and `Expr` literal variants are structurally identical** - `ast.rs:103-147`
**Confidence**: 82%
- Problem: `Expr` now has `StringLiteral`, `NumberLiteral`, `BooleanLiteral`, `NullLiteral` variants (added in this PR), and `Arg` has identical variants (`StringLiteral`, `NumberLiteral`, `BooleanLiteral`, `NullLiteral`). Both enums represent the same concept (a parsed expression node) at different levels but with duplicated shape. The `Arg` enum is essentially a subset of `Expr` with the addition of call nesting but without `QualifiedCall` or `MemberAccess`+`object`/`fields`. This means `parse_single_arg_inner` and `parse_expr_inner` contain parallel classification logic for the same literal forms.
- Impact: Adding a new literal type requires changes in both enums and both parsers. The `resolve_args` function in the evaluator must also mirror the `evaluate_expr` paths for each shared variant. This is a maintenance multiplier that will grow as the expression grammar evolves.
- Fix: Consider unifying `Arg` into `Expr` so that function arguments are simply expressions. The evaluator's `resolve_args` would call `evaluate_expr` instead of pattern-matching `Arg` variants independently. This aligns with the PR's direction of making `Expr` the universal expression node. Note: this is a refactoring opportunity, not a blocking issue.

## Pre-existing Issues (Not Blocking)

(none -- prior cycle deferred items not re-raised per cross-cycle awareness)

## Suggestions (Lower Confidence)

- **Scanner state machine duplication across `strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op`, `parse_simple_condition` bare-`=` check, `parse_args_inner`, `split_on_unquoted_commas`, and `find_unquoted_equals`** - `parser_helpers.rs` (Confidence: 70%) -- Eight separate functions each implement their own quote-tracking + escape-handling + paren-depth byte scanner. The PR adds `strip_trailing_directive_colon` (new) and `has_unterminated_string` (new), bringing the total scanner count higher. A shared `QuoteParenScanner` iterator could encapsulate the common state machine and reduce each function to consuming tokens from the scanner. Deferred from prior cycle; the new functions make the case slightly stronger but the risk profile has not changed.

- **`validate_condition` accepts `&Scope` but `evaluate_condition` now requires `&mut Scope`** - `validator.rs:180` vs `evaluator.rs:421` (Confidence: 65%) -- The validator validates conditions with an immutable scope reference, meaning it cannot invoke functions during validation (function dispatch requires `&mut Scope`). This means a condition like `@if func(x):` has its function existence validated but not its return type. This is documented as an accepted limitation, but as expression complexity grows (chained calls, nested expressions in conditions), the gap between what the validator checks and what the evaluator executes widens. Worth monitoring as the expression grammar matures.

- **`parse_expr_inner` does not handle nested parenthesized expressions** - `parser_helpers.rs:186-235` (Confidence: 62%) -- The function finds `first_paren` via `s.find('(')` to detect calls, but a qualified call like `ns.func(inner("x"))` would have the closing `)` matched by `strip_suffix(')')` which takes the last `)` in the string. This works today because the grammar does not allow expression-level grouping `(a == b)` or chained calls `f(g(x))` in directive positions. If the grammar ever adds those, this simple positional parsing will break. The current behavior is correct for the current grammar.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | - | - | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED

### Architectural Assessment

This PR executes a clean, well-structured refactoring that unifies the expression model across the compiler pipeline. The key architectural changes are sound:

1. **`Condition` now holds `Expr` instead of `Vec<String>`** -- This eliminates the artificial restriction that conditions could only reference bare variables and dot-paths. The `Condition::path()` and `Condition::root()` helper methods are removed, and the evaluator delegates directly to `evaluate_expr`, which is the correct layering. Both sides of `Eq`/`NotEq` are now symmetric `Expr` values rather than the asymmetric `Vec<String>` + `CondValue` pattern.

2. **`ForBlock.iterable` changes from `Vec<String>` to `Expr`** -- Same unification. The evaluator's `evaluate_for` now calls `evaluate_expr` instead of manually resolving a dot-path. The validator correctly handles each `Expr` variant with appropriate static checks (full type check for `Var`, root-exists check for `MemberAccess`, arity validation for `Call`/`QualifiedCall`, and a defensive guard for literal variants).

3. **`evaluate_expr` returns `Result<Value>` instead of `Result<String>`** -- The split into `evaluate_expr` (returns `Value`) and `render_expr` (returns `String` with object-interpolation guard) cleanly separates the concerns of expression evaluation from string rendering. This allows `evaluate_condition` and `evaluate_for` to work with typed `Value` results without redundant string conversion. (applies ADR-008 -- this is part of a bundled language feature PR)

4. **`strip_trailing_directive_colon` adds quote+paren-aware scanning** -- Necessary for correctness now that directives can contain function calls with colons in string arguments (e.g., `@if func("a:b"):`). The implementation is thorough with unclosed-paren detection.

5. **Resource limit hardening on `split()`/`join()`** -- The incremental element counting in `builtin_split` and output size guard in `builtin_join` are well-placed defense-in-depth measures.

The `CondValue`/`Expr` duplication (deferred from cycle 1) is now partially resolved: `CondValue` is decoupled from conditions and only used for `@define` default parameters, which is a reasonable scoping. The remaining `Arg`/`Expr` duplication is flagged as a should-fix opportunity but does not block this PR.

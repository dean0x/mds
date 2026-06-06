# Regression Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04
**PR**: #76

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

- **Missing CHANGELOG entry** - `CHANGELOG.md` (Confidence: 65%) -- The PR introduces a new user-visible feature (expression support in `@if`/`@for` directives) and changes public AST types (`Condition`, `ForBlock.iterable`), but no CHANGELOG entry was added. This is a minor documentation gap, not a code regression.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### AST Breaking Changes (Fully Migrated)

The following AST types were changed in this PR:

1. **`Condition` enum variants**: `Truthy(Vec<String>)` -> `Truthy(Expr)`, `Not(Vec<String>)` -> `Not(Expr)`, `Eq(Vec<String>, CondValue)` -> `Eq(Expr, Expr)`, `NotEq(Vec<String>, CondValue)` -> `NotEq(Expr, Expr)`. All consumers (evaluator, validator, tests) updated.

2. **`Condition::path()` and `Condition::root()` methods removed**: These helper methods returned dot-path segments. No remaining callers in the codebase -- the evaluator now uses `evaluate_expr` directly, and the validator uses pattern matching on expression types.

3. **`ForBlock.iterable` type changed**: `Vec<String>` -> `Expr`. Both consumers (evaluator at line 617, validator at line 96) updated to match on `Expr` variants. `applies ADR-008` (bundled language feature changes).

4. **`values_equal` renamed to `values_equal_runtime`**: Signature changed from `(Value, CondValue)` to `(Value, Value)` for expression-vs-expression comparison. No stale references remain. Semantics preserved (strict equality, NaN != NaN).

5. **`evaluate_condition` signature changed**: `(&Condition, &Scope)` -> `(&Condition, &mut Scope, &mut EvalContext)`. Required because conditions can now contain function calls that need mutable scope access (push/pop frames) and evaluation context (call depth tracking). All 4 call sites updated.

6. **`evaluate_expr` return type changed**: `Result<String, MdsError>` -> `Result<Value, MdsError>`. New `render_expr` wrapper added for interpolation (the only path that needs string output). Object-interpolation error preserved with equivalent messages.

### Resource Limits Preserved

- `MAX_DOT_SEGMENTS` (32): Still enforced via `validate_dot_path_parts` called from `parse_expr_inner` for `MemberAccess` expressions.
- `MAX_NESTING_DEPTH`: Unchanged.
- `MAX_ELSEIF_BRANCHES`: Unchanged.
- `MAX_LOGICAL_OPERANDS`: Unchanged.
- `MAX_CALL_DEPTH`: Unchanged, now also applies to function calls in conditions.
- `MAX_OUTPUT_SIZE`: Unchanged, plus new guard added inside `join()` builtin.
- **New**: `MAX_ARRAY_ELEMENTS` (100,000) added for `split()` builtin -- defensive limit against adversarial input.

### Backward Compatibility Verified

Explicit backward compatibility tests added:
- `parse_backward_compat_if_var_truthy`: `@if active:` still produces `Truthy(Expr::Var("active"))`
- `parse_backward_compat_if_var_eq_string`: `@if role == "admin":` still produces `Eq(Var, StringLiteral)`
- `parse_backward_compat_for_var_iterable`: `@for x in items:` still produces `ForBlock` with `Expr::Var("items")`

All 764 existing tests pass. The PR description's claim of expression directive support is accurately reflected in the implementation.

### Scope Mutation in Conditions

The `evaluate_condition` signature changed from `&Scope` to `&mut Scope`. This is necessary and safe: function calls within conditions use `scope.push()`/`scope.pop()` isolation, so the parent scope is not corrupted. The `EvalContext` parameter enables call-depth tracking for recursive function calls in conditions, which was not previously needed.

### `CondValue` Still Used

`CondValue` enum and `parse_cond_value` function are retained for default parameter values in `@define` blocks. This is correct -- default parameter parsing has a different grammar than directive condition expressions and does not need the full expression parser.

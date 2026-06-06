# Regression Review Report

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04

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

(none)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### 1. Lost Functionality

**Removed `Condition::path()` and `Condition::root()` methods** (ast.rs lines 60-93)

These two public methods on `Condition` were removed. However, the `ast` module is `pub(crate)` in `lib.rs`, so these are internal-only. A full search for `condition.path()`, `condition.root()`, and `resolve_condition_value` across all crates confirmed zero remaining callers. The migration is complete.

**`ForBlock.iterable` type change: `Vec<String>` to `Expr`**

All 5 sites that construct or match on `ForBlock.iterable` (parser.rs, evaluator.rs, validator.rs, parser_tests.rs, evaluator tests) have been updated to use `Expr` variants. No remaining `Vec<String>` references to the iterable field.

**`Condition` variant payloads changed: `Vec<String>` / `CondValue` to `Expr`**

`Truthy(Vec<String>)` became `Truthy(Expr)`, `Not(Vec<String>)` became `Not(Expr)`, `Eq(Vec<String>, CondValue)` became `Eq(Expr, Expr)`, `NotEq(Vec<String>, CondValue)` became `NotEq(Expr, Expr)`. All call sites verified updated. `And`/`Or` variants unchanged.

**`evaluate_expr` return type: `Result<String>` to `Result<Value>`**

The old `evaluate_expr` returned a string directly. The new version returns `Value`, with a new `render_expr` wrapper that converts to string for interpolation. The object-interpolation guard (preventing `{obj}` from silently rendering as `[Object]`) is preserved in `render_expr` with the same error messages.

**`evaluate_condition` signature: added `&mut EvalContext` parameter**

All 4 call sites (primary `@if`, `@elseif` loop, `And` operands, `Or` operands) pass the `ctx` parameter. No callers missed.

**`validate_call_arity` return type: `Result<bool>` to `Result<()>`**

The old return distinguished builtins from user functions, but both paths in the caller (`validate_expr`) performed the same `validate_var_args` call. The simplification is behavior-preserving.

**`parse_dot_path` removed, replaced by `parse_expr_inner`**

`parse_dot_path` parsed dot-separated identifiers for conditions. `parse_expr_inner` is a superset that accepts the same dot-paths (delegating to `validate_dot_path_parts` which enforces the same `MAX_DOT_SEGMENTS` limit and identifier validation) plus function calls and literals. One test function (`parse_dot_path_at_limit_accepted`) retains the old name but tests interpolation dot-paths, not the removed function.

**`values_equal` renamed to `values_equal_runtime`**

Now compares two `Value` instances instead of `Value` vs `CondValue`. The NaN semantics test was updated accordingly.

### 2. Broken Behavior

No broken behavior detected. Key behavioral guarantees preserved:

- Simple `@if var:`, `@if !var:`, `@if var == "val":` patterns parse and evaluate identically.
- Simple `@for item in items:` and `@for item in data.list:` patterns parse and evaluate identically.
- `@if` with `&&`/`||` logical operators works identically.
- `@elseif` branches parse identically.
- Object-interpolation guard preserved in `render_expr`.
- NaN inequality semantics preserved in `values_equal_runtime`.
- `strip_trailing_directive_colon` correctly handles the simple case (`var:`) identically to the old `strip_suffix(':')`, while adding safe handling for colons inside strings/parens.

### 3. Intent vs Reality

PR description claims align with implementation:
- "Condition leaf variants now hold Expr" -- confirmed in ast.rs
- "ForBlock.iterable changes from Vec<String> to Expr" -- confirmed
- "evaluate_expr returns Result<Value>" -- confirmed
- "strip_trailing_directive_colon uses quote+paren-aware scanning" -- confirmed
- "split()/join() get resource limit hardening" -- confirmed (MAX_ARRAY_ELEMENTS, MAX_OUTPUT_SIZE)
- "All types are pub(crate) -- no public API change" -- confirmed (ast module is pub(crate))
- "764 existing tests pass" -- 771 tests now pass (7 new test files added)

### 4. Incomplete Migrations

No incomplete migrations found. All consumers of changed types and functions have been updated.

### 5. Test Coverage

38 new parser tests added covering expression directives. Existing tests updated to match new AST types. All 771 tests pass. Clippy and fmt clean.

# Security Review Report

**Branch**: feat/74-expression-directives -> main
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

(none)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis

This PR extends the directive parsing grammar to accept full expressions (function calls, qualified calls, literals) in `@if`, `@elseif`, and `@for` directives. The security posture is strong:

### Resource Limit Hardening (Positive)

The PR *improves* security by adding two new resource limits that were not previously enforced:

1. **`split()` element cap** (`MAX_ARRAY_ELEMENTS = 100,000`) in `builtins.rs:264-273` -- prevents adversarial inputs from producing arrays with hundreds of thousands of elements. The incremental check fires before full allocation, which is important for WASM environments where a single-byte split on a large input could peak at ~240 MB.

2. **`join()` output size guard** (`MAX_OUTPUT_SIZE = 50 MB`) in `builtins.rs:396-401` -- prevents output amplification where large arrays with long elements could produce output exceeding the evaluator's per-node check.

Both limits align with existing defense-in-depth patterns (`MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE` in evaluator).

### Input Validation (Positive)

- **Bare literal rejection**: The parser correctly rejects bare literals in truthy/negation positions (`@if true:`, `@if "str":`, `@if !42:`) and as `@for` iterables (`@for x in "literal":`), preventing template authors from writing semantically meaningless directives.
- **NaN/Infinity rejection**: `parse_expr_inner` rejects non-finite numbers with an explicit check (`!n.is_finite()`), preventing IEEE 754 edge cases from entering the expression AST.
- **Empty LHS validation**: `parse_simple_condition` now validates that the LHS of a comparison operator is non-empty (`lhs.is_empty()` check at line 599-603), closing a gap where `== "val"` would have been accepted.

### Quote/Paren-Aware Parsing (Positive)

The new `strip_trailing_directive_colon` function is a significant parser security improvement. The previous `strip_suffix(':')` approach was fragile -- it would misparse directives containing colons inside string arguments (e.g., `@if contains(s, "a:b"):`). The new scanner correctly tracks:
- String literal boundaries (with escape handling for `\"` and `\'`)
- Parenthesis depth
- Unclosed parenthesis detection (returns `None`, causing a parse error)

The same paren-awareness was consistently added to `find_unquoted_operator` and `split_on_unquoted_op`, preventing operators inside function call arguments from being mistaken for top-level condition operators.

### Evaluated Expression Scope (Safe)

The change from `evaluate_condition(condition, scope)` to `evaluate_condition(condition, scope, ctx)` passes `EvalContext` through to enable function calls within conditions. This is safe because:
- `EvalContext` tracks call depth (`MAX_CALL_DEPTH`), preventing stack overflow from recursive calls
- Short-circuit evaluation is correctly maintained for `&&`/`||`
- `is_truthy()` handles all `Value` variants safely (including `Object` and `Array`)
- `values_equal_runtime` uses strict type matching (cross-type comparisons return `false`)

### Nesting Depth Protection (Inherited)

`parse_expr_inner` delegates to `parse_args` for function call arguments, which uses `parse_args_inner` with `MAX_NESTING_DEPTH` enforcement. `parse_expr_inner` itself is not recursive, so there is no unbounded recursion risk in the new code.

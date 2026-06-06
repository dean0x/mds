# Reliability Review Report

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

- **Duplicated scanner state machines across 4 functions** - `strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op` (Confidence: 65%) -- Each function independently implements quote+paren-aware byte scanning with its own `in_string`/`string_char`/`paren_depth` state machine. A single reusable scanner abstraction would eliminate the risk of one scanner drifting out of sync with another (e.g., one gains bracket-awareness while others do not). This was deferred in the prior review cycle; noting again as the count grew from 2 to 4 functions with this PR.

- **`parse_expr_inner` quoted-string detection uses `ends_with('"')` without checking for escaped trailing quote** - `parser_helpers.rs:146-148` (Confidence: 62%) -- A string like `"hello\"` (unterminated -- the final `"` is escaped) would match `starts_with('"') && ends_with('"')` and be accepted as the literal `hello\` after unescape. In practice this is unlikely because `strip_trailing_directive_colon` would have already consumed the outer directive structure, and the condition-split functions only feed well-bounded substrings. But the guard is incomplete in isolation. The same pattern exists in `parse_cond_value` (pre-existing).

- **`String::new()` without capacity hint in `join()` output accumulator** - `builtins.rs:385` (Confidence: 60%) -- For large arrays, `String::new()` will reallocate multiple times as it grows. A conservative capacity estimate (e.g., `min(arr.len() * avg_element_guess, MAX_OUTPUT_SIZE)`) could reduce allocation churn. The output size guard at line 396 bounds total memory, so this is a performance microoptimization, not a correctness issue.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This PR demonstrates strong reliability practices:

1. **Bounded iteration** -- All loops remain bounded. `evaluate_for_array` and `evaluate_for_key_value` enforce `MAX_LOOP_ITERATIONS` and `MAX_TOTAL_ITERATIONS`. The new `split()` enforces `MAX_ARRAY_ELEMENTS` (100K) incrementally, preventing peak allocation before the limit fires. `join()` checks `MAX_OUTPUT_SIZE` per element.

2. **Bounded recursion** -- `parse_args_inner` enforces `MAX_NESTING_DEPTH` on nested function call depth. `parse_condition` enforces `MAX_LOGICAL_OPERANDS` (16) on condition trees. The condition evaluation recursion is structurally limited to depth 2 (Or -> And -> leaf) with `debug_assert!` canaries.

3. **Assertion density** -- `debug_assert!` guards on And/Or nesting invariants in the evaluator, defensive `Expr::*Literal` match arm in `validate_for_node` that returns an error if literals bypass the parser check, and `saturating_sub` on parenthesis depth counters.

4. **Allocation discipline** -- `split()` uses incremental `Vec::new()` (intentionally avoiding pre-allocation to catch the limit before full allocation). `join()` checks output size after each element push. The `strip_trailing_directive_colon` scanner is zero-allocation (operates on `&str` slices).

5. **Resource limits preserved** -- `MAX_DOT_SEGMENTS` is still enforced via `validate_dot_path_parts` in `parse_expr_inner`. `MAX_NESTING_DEPTH`, `MAX_ELSEIF_BRANCHES`, `MAX_LOGICAL_OPERANDS`, `MAX_CALL_DEPTH`, `MAX_OUTPUT_SIZE`, `MAX_LOOP_ITERATIONS`, and `MAX_TOTAL_ITERATIONS` remain intact and correctly applied to the new expression paths.

6. **Cross-cycle note** -- The "duplicated scanner state machines" issue was deferred in the prior cycle. It grew slightly (from 2 to 4 functions) but remains a LOW-severity maintainability concern, not a reliability risk. The `parse_simple_condition` complexity item was also deferred previously; this PR does not materially increase its complexity.

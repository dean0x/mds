# Complexity Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### HIGH

**`builtin_sort` exceeds function length guideline (52 lines)** - `builtins.rs:378`
**Confidence**: 85%
- Problem: `builtin_sort` is 52 lines with two parallel match arms (String and Number) that each contain a validation loop followed by a sort-by closure. The two arms are structurally identical (validate homogeneity, then sort), differing only in the Value variant they match. Cyclomatic complexity is elevated by the nested match-inside-sort_by plus unreachable!() arms.
- Fix: Extract a generic `sort_homogeneous` helper that accepts a type predicate and a comparator, or at minimum extract the homogeneity check into a `require_homogeneous(arr, expected_type)` helper. This would reduce `builtin_sort` to ~20 lines.
```rust
fn require_homogeneous(arr: &[Value], expected: &str) -> Result<(), MdsError> {
    for item in arr {
        if item.type_name() != expected {
            return Err(MdsError::builtin_error(format!(
                "sort() requires a homogeneous array; found {} mixed with {expected}",
                item.type_name()
            )));
        }
    }
    Ok(())
}
```

**`validate_node` exceeds function length guideline (109 lines)** - `validator.rs:23`
**Confidence**: 82%
- Problem: `validate_node` is a single match with 9 arms totaling 109 lines. The `Node::For` arm alone is 40+ lines with nested if-conditions (nesting depth 4 at the type-check branch). The `Node::If` arm, while shorter per-arm, chains through multiple validate calls. This function carries the highest cyclomatic complexity in the PR.
- Fix: Extract the `Node::For` and `Node::If` arms into dedicated `validate_for_node` and `validate_if_node` functions, similar to how the evaluator already has `evaluate_if` and `evaluate_for` as separate functions. This would bring `validate_node` below 30 lines and each extracted function below 40.

### MEDIUM

**`parse_args_inner` has 4-level nesting and complex state machine (79 lines)** - `parser_helpers.rs:685`
**Confidence**: 82%
- Problem: `parse_args_inner` uses a hand-rolled state machine (5 mutable variables: `in_string`, `string_char`, `escaped`, `paren_depth`, `current`) with nested match arms inside an if-else chain inside a for loop. While each piece is individually simple, the interaction between the 5 state variables requires careful mental tracking.
- Fix: This is acceptable for a parser, but could benefit from a brief inline comment at the top summarizing the state transitions (e.g., "States: normal -> in_string -> escaped -> normal"). The current doc comment on the state variables (lines 696-701) is good; the complexity is managed. No code change required, but consider this if future modifications increase the state count.

**`validate_var_args` has near-duplicated arity check logic (79 lines)** - `validator.rs:249`
**Confidence**: 80%
- Problem: The arity-check logic for nested calls (lines 280-322) is structurally identical to the arity check in `validate_expr` for `Expr::Call` (lines 179-214). Both follow the pattern: check user-defined function first, then check builtins, then error. This duplication means a change to the arity-check logic must be made in two places.
- Fix: Extract a shared `check_call_arity(name, args_len, scope, file, source, offset, len)` helper that handles the user-defined-then-builtin lookup and returns an appropriate error. Both `validate_expr::Call` and `validate_var_args::Call` would delegate to it.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Four near-identical quote-aware scanning state machines in `parser_helpers.rs`** - `parser_helpers.rs:124,205,840,890`
**Confidence**: 85%
- Problem: `find_unquoted_operator`, `split_on_unquoted_op`, `split_on_unquoted_commas`, and `find_unquoted_equals` each implement a quote-tracking byte scanner with the same `in_string`/`string_char`/escaped pattern. There are 21 occurrences of the `in_string` variable across these 4 functions. While each has slightly different action-on-match behavior, the quote-tracking skeleton is identical.
- Fix: Consider extracting a generic `scan_unquoted` iterator or callback-based scanner:
```rust
fn for_each_unquoted_byte(s: &str, mut f: impl FnMut(usize, u8) -> ControlFlow<T>) -> Option<T>
```
This would eliminate the duplicated quote-tracking and reduce each caller to ~5 lines that only express the match logic. Applies ADR-008 rationale -- batching these related changes is the right time to consolidate the shared pattern.

## Pre-existing Issues (Not Blocking)

_None identified at CRITICAL severity._

## Suggestions (Lower Confidence)

- **`builtins.rs` file length (855 lines including tests)** - `builtins.rs:1` (Confidence: 65%) -- The file is at the upper end of the Warning threshold (300-500 production lines + ~380 test lines). The tests are comprehensive and well-structured; the file is manageable today but will cross into Critical territory when additional built-ins are added. Consider splitting tests to `builtins_tests.rs` (matching the `error_tests.rs` pattern) to keep the production code under 500 lines.

- **`call_builtin` dispatch match has 18 arms** - `builtins.rs:136` (Confidence: 60%) -- The `call_builtin` dispatch match is a flat, 1:1 name-to-function mapping with 18 arms. This is maintainable today because each arm is a single function call, but adding more built-ins will make this unwieldy. A `HashMap<&str, fn(&[Value]) -> Result<Value, MdsError>>` lookup table would be more scalable.

- **`parser_helpers.rs` file length (1074 lines)** - `parser_helpers.rs:1` (Confidence: 70%) -- At 1074 lines, this file exceeds the Critical threshold (>500 production lines). It grew organically from condition parsing (pre-existing) plus the new `parse_define_params`, `split_on_unquoted_commas`, and `find_unquoted_equals` additions. Consider splitting into `parser_conditions.rs` and `parser_args.rs` when convenient.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR bundles three features well (applies ADR-008). Individual functions are generally well-decomposed -- most stay under 30 lines. The two HIGH findings (`builtin_sort` at 52 lines and `validate_node` at 109 lines) are the primary concerns. The quadruplicated quote-aware scanner is the most significant maintainability debt introduced. None of these issues risk correctness or safety; they affect readability and future change cost. The code is well-documented with thorough inline comments explaining invariants.

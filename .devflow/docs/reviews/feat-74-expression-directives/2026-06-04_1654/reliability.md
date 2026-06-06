# Reliability Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04
**PR**: #76

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Silent absorption of unmatched close-parens in scanner functions (4 occurrences)** -- Confidence: 82%
- `crates/mds-core/src/parser_helpers.rs:67`, `crates/mds-core/src/parser_helpers.rs:368`, `crates/mds-core/src/parser_helpers.rs:475`, `crates/mds-core/src/parser_helpers.rs:632`
- Problem: All four new byte-level scanner functions (`strip_trailing_directive_colon`, `find_unquoted_operator`, `split_on_unquoted_op`, and the bare-`=` scanner inside `parse_simple_condition`) use `paren_depth = paren_depth.saturating_sub(1)` for `)` characters. While this prevents panics from underflow, it silently absorbs unmatched closing parentheses, meaning the scanner treats them as if they were at depth 0. For `strip_trailing_directive_colon`, this could cause the scanner to find a "bare colon" that is actually inside an unbalanced paren group. The `saturating_sub` pattern is appropriate as a defensive floor, but without any diagnostic for the `paren_depth == 0 && ch == b')'` case, malformed input is silently accepted rather than rejected. applies ADR-008 (batched features share the same parser layer -- consistent error handling matters).
- Fix: After the `while` loop in `strip_trailing_directive_colon` (and optionally the other scanners), consider checking if `paren_depth > 0` at loop exit and returning `None` (indicating malformed input). For the operator scanners that return `Option`, returning `None` on unbalanced parens is a conservative safe default. Example for `strip_trailing_directive_colon`:
```rust
// After the while loop, before the last_bare_colon check:
if paren_depth > 0 {
    return None; // Unclosed parenthesis -- treat as malformed directive
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`evaluate_condition_value` is a trivial pass-through to `evaluate_expr`** - `crates/mds-core/src/evaluator.rs:414-420` (Confidence: 65%) -- This function adds an abstraction layer with zero logic. It exists as a 1-line delegation to `evaluate_expr`. Consider calling `evaluate_expr` directly in `evaluate_condition` to reduce indirection (Reliability Category 4: Indirection Limits). However, the wrapper may serve as a future extension point, so this is a minor style note.

- **`parse_expr_inner` uses `strip_suffix(')')` to find the closing paren of function calls** - `crates/mds-core/src/parser_helpers.rs:187-190` (Confidence: 70%) -- This works correctly for current grammar (nested calls like `sort(unique(tags))`) because `strip_suffix` removes only the last character and inner parens remain balanced. However, if the grammar ever adds expressions after the closing paren (e.g., method chaining `func(x).field`), this would silently mismatch. The existing interpolation parser uses the same pattern, so this is consistent. Not a bug today, but a fragility note.

- **Duplicated byte-level scanning logic across 5 functions** - `crates/mds-core/src/parser_helpers.rs` (Confidence: 60%) -- The same string/paren-tracking state machine (in_string, string_char, paren_depth, escape handling) is duplicated in `strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op`, and the bare-`=` scanner. Each copy must be kept in sync for correctness. A single `ScanState` struct with shared advancement logic would reduce the risk of divergence. This is a maintainability note, not a current bug.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10

**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Reliability Observations

1. **Bounded iteration**: All new byte-level loops iterate over `bytes.len()`, which is upstream-bounded by `MAX_FILE_SIZE` (10 MB). No unbounded loops introduced.

2. **New resource limits for split/join**: The `MAX_ARRAY_ELEMENTS` (100K) limit on `split()` and `MAX_OUTPUT_SIZE` (50 MB) limit on `join()` are well-placed defensive bounds that prevent amplification attacks through the new expression-in-directive pathway (e.g., `@for x in split(adversarial_input, ","):`).

3. **Recursion depth is bounded**: `parse_args_inner` checks depth against `MAX_NESTING_DEPTH` (64), preventing stack overflow from deeply nested calls like `f(f(f(...)))`. The `evaluate_condition` recursion through `And`/`Or` is bounded by `MAX_LOGICAL_OPERANDS` (16) enforced at parse time.

4. **Defense-in-depth in validator**: The validator's `Expr::StringLiteral | ... | Expr::NullLiteral` guard in `validate_for_node` catches literal iterables that should have been rejected at parse time -- good defensive programming even though the parser already rejects this case.

5. **NaN handling preserved**: `values_equal_runtime` correctly preserves IEEE 754 NaN semantics (NaN != NaN), and `parse_expr_inner` rejects non-finite numeric literals at parse time.

6. **Test coverage**: 764 tests pass, including targeted tests for the new resource limits (`split_resource_limit_too_many_elements`, `join_resource_limit_output_too_large`) and expression directive parsing/evaluation.

### Condition for Approval

The unmatched-paren silent absorption (MEDIUM finding above) is a defense-in-depth concern, not a correctness bug in normal use. It should be addressed before the next release but does not block this merge.

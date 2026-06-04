# Complexity Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**`evaluate_for` has grown to 94 lines with near-duplicate loop bodies** - `src/evaluator.rs:338-431`
**Confidence**: 88%
- Problem: The function now handles two iteration modes (key-value over objects and standard array iteration). Both branches contain nearly identical loop bodies: increment `ctx.total_iterations`, check `MAX_TOTAL_ITERATIONS`, `scope.push()`, set variables, `evaluate_nodes`, `scope.pop()`, `prefer_first_error`. This duplication increases the function's line count to 94 (exceeds the 50-line warning threshold) and creates a maintenance risk where a bug fix in one loop body could miss the other.
- Fix: Extract the shared loop body into a helper function:
```rust
fn evaluate_loop_iteration(
    body: &[Node],
    scope: &mut Scope,
    ctx: &mut EvalContext,
) -> Result<String, MdsError> {
    ctx.total_iterations += 1;
    if ctx.total_iterations > MAX_TOTAL_ITERATIONS {
        return Err(MdsError::resource_limit(format!(
            "total loop iterations exceeded maximum of {} across all loops in this compilation",
            MAX_TOTAL_ITERATIONS
        )));
    }
    let rendered = evaluate_nodes(body, scope, ctx);
    let pop_result = scope.pop();
    prefer_first_error(rendered, pop_result)
}
```
Then both the key-value and array iteration paths call `scope.push()`, set their vars, and delegate to this helper.

**`parse_for_block` has grown to 72 lines** - `src/parser.rs:249-320`
**Confidence**: 82%
- Problem: The function grew from a simple `splitn(3, ' ')` pattern to a multi-phase parse: find `" in "`, split on comma for key-value pattern, validate each identifier, split iterable on dots, validate each segment. At 72 lines it crosses the 50-line warning threshold. The cyclomatic complexity is moderate (approximately 8 paths: comma present/absent, valid/invalid key, valid/invalid val, valid/invalid iterable parts) but manageable.
- Fix: Extract the variable-part parsing into a helper:
```rust
fn parse_for_vars(var_part: &str) -> Result<(Option<String>, String), MdsError> {
    if var_part.contains(',') {
        let comma_idx = var_part.find(',').expect("comma check already passed");
        let key = var_part[..comma_idx].trim().to_string();
        let val = var_part[comma_idx + 1..].trim().to_string();
        // ... validation ...
        Ok((Some(key), val))
    } else {
        // ... validation ...
        Ok((None, var_part.to_string()))
    }
}
```

### MEDIUM

**`parse_interpolation_expr` is 90 lines with 4-level nesting** - `src/parser.rs:497-586`
**Confidence**: 85%
- Problem: The function handles four expression forms (MemberAccess, QualifiedCall, Call, Var) in a single 90-line body. The `match (first_dot, first_paren)` arm has nested `if let Some(paren_pos)` inside a match guard, reaching 4 levels of nesting. The function was already long before this PR; the changes restructured it (replacing `dot_notation_error` with `MemberAccess` handling) but did not reduce its size.
- Fix: Extract the dot-path branch (lines 516-556) into a `parse_dot_expr` helper that returns `Ok(Interpolation)` for either QualifiedCall or MemberAccess. This would reduce `parse_interpolation_expr` to ~45 lines and eliminate the deepest nesting.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`validate_node` is 100 lines with growing `Node::For` arm** - `src/validator.rs:17-116`
**Confidence**: 83%
- Problem: The `validate_node` function is a single match expression spanning 100 lines. The `Node::For` arm alone is 44 lines (lines 48-92) and now contains a 3-level nested conditional: `if block.key_var.is_none() && ... { if matches!(iterable_val, Value::Object(_)) { ... } ... }`. While each arm is self-contained, the overall function is approaching the critical threshold. The changes to this function added the object-type check and the key-var scope injection, which were necessary but increased both line count and nesting.
- Fix: Extract the `Node::For` arm body into `validate_for_block(block, scope, file, source)`. Similarly, the `Node::If` arm could become `validate_if_block(...)`. This pattern is already used in the evaluator (`evaluate_for`, `evaluate_if`).

## Pre-existing Issues (Not Blocking)

No critical pre-existing complexity issues found in the changed files.

## Suggestions (Lower Confidence)

- **Dot-path construction is repeated in 3 places** - `src/evaluator.rs:164-166`, `src/evaluator.rs:207-209`, and similar patterns in the validator (Confidence: 72%) -- The pattern `std::iter::once(object.clone()).chain(fields.iter().cloned()).collect()` appears twice in evaluate_expr (for `Expr::MemberAccess` and `Arg::MemberAccess`). A small helper `fn build_dot_path(object: &str, fields: &[String]) -> Vec<String>` would reduce repetition.

- **Identifier validation loop is repeated for dot-paths** - `src/parser.rs:220-226`, `src/parser.rs:300-306`, `src/parser.rs:541-548`, `src/parser.rs:686-692` (Confidence: 68%) -- Four locations split a string on `.`, iterate parts, and call `is_valid_identifier`. A `fn validate_dot_path(s: &str) -> Result<Vec<String>, MdsError>` helper would consolidate this.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new code is logically sound and well-commented. The primary concerns are (1) the `evaluate_for` function's near-duplicate loop bodies creating a maintenance risk, and (2) several functions crossing the 50-line threshold due to the combined weight of three features landing in one PR. None of these are blocking -- the code is understandable, well-structured within each function, and all resource limits are properly enforced in both code paths. The duplication in `evaluate_for` is the strongest candidate for extraction before or shortly after merge.

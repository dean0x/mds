# Complexity Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**`evaluate_for` function exceeds 50 lines (96 lines)** - `src/evaluator.rs:333`
**Confidence**: 92%
- Problem: The function grew to 96 lines after adding key-value object iteration alongside the existing array iteration. It contains two distinct iteration paths (key-value and array) with duplicated resource-limit checks and scope management, making it harder to understand at a glance. Nesting reaches 4 levels inside the key-value iteration loop.
- Fix: Extract the key-value iteration path (lines 342-383) into a dedicated helper function:

```rust
fn evaluate_for_key_value(
    key_var: &str,
    value_var: &str,
    map: HashMap<String, Value>,
    body: &[Node],
    scope: &mut Scope,
    ctx: &mut EvalContext,
) -> Result<String, MdsError> {
    if map.len() > MAX_LOOP_ITERATIONS {
        return Err(MdsError::resource_limit(format!(
            "object has {} entries, exceeding maximum loop iteration limit of {}",
            map.len(), MAX_LOOP_ITERATIONS
        )));
    }
    let mut entries: Vec<(String, Value)> = map.into_iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));

    let mut output = String::new();
    for (key, val) in entries {
        ctx.total_iterations += 1;
        if ctx.total_iterations > MAX_TOTAL_ITERATIONS {
            return Err(MdsError::resource_limit(format!(
                "total loop iterations exceeded maximum of {} across all loops in this compilation",
                MAX_TOTAL_ITERATIONS
            )));
        }
        scope.push();
        scope.set_var(key_var, Value::String(key));
        scope.set_var(value_var, val);
        let rendered = evaluate_nodes(body, scope, ctx);
        let pop_result = scope.pop();
        output.push_str(&prefer_first_error(rendered, pop_result)?);
    }
    Ok(output)
}
```

This would reduce `evaluate_for` to ~50 lines and give each iteration mode a clear, focused home.

---

**`parse_interpolation_expr` function exceeds 50 lines (94 lines)** - `src/parser.rs:501`
**Confidence**: 88%
- Problem: This function handles 4 distinct expression types (QualifiedCall, MemberAccess, Call, Var) in a single 94-line body. The match arm for the dot-before-paren case has 3 levels of nesting (match + guard condition + inner if-let). While each individual branch is clear, the overall function requires reading all 94 lines to understand the full parsing logic.
- Fix: Extract the dot-path handling (lines 519-561, the `Some(dot_pos)` match arm) into a helper:

```rust
fn parse_dot_expr(
    content: &str,
    dot_pos: usize,
    offset: usize,
    len: usize,
    file: &str,
    source: &str,
) -> Result<Interpolation, MdsError> {
    let rest_after_dot = &content[dot_pos + 1..];
    if let Some(paren_pos) = rest_after_dot.find('(') {
        // QualifiedCall path
        ...
    }
    // MemberAccess path
    ...
}
```

This would bring `parse_interpolation_expr` under 50 lines and make the routing logic (dot vs paren vs plain) immediately visible.

---

### MEDIUM

**`validate_node` function exceeds 50 lines (115 lines)** - `src/validator.rs:17`
**Confidence**: 85%
- Problem: The `Node::For` arm within `validate_node` grew to ~56 lines with the addition of key-var handling, dot-path skipping, and the object-specific error message. The compound boolean condition on line 78 (`block.key_var.is_none() && block.iterable.len() == 1 && !matches!(...)`) packs 3 conditions into one `if`, requiring careful reading.
- Fix: Extract the `Node::For` validation arm into a `validate_for_block` helper function (as was done for other validators in similar projects), or at minimum extract a named predicate for the compound condition:

```rust
/// Whether the validator can statically check the iterable's type.
fn can_static_check_iterable(block: &ForBlock) -> bool {
    block.key_var.is_none() && block.iterable.len() == 1
}
```

## Issues in Code You Touched (Should Fix)

_No issues identified in this category._

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parse_body` at 50 lines with deep match nesting** - `src/parser.rs:108`
**Confidence**: 80%
- Problem: The `parse_body` function is exactly at the 50-line threshold with a while-loop containing a match with 7 arms. This is pre-existing structure but now sits at the complexity boundary. Not blocking since it was not modified in this PR.

## Suggestions (Lower Confidence)

- **Duplicated iteration-loop boilerplate** - `src/evaluator.rs:367-381` and `src/evaluator.rs:412-424` (Confidence: 72%) -- The scope push/evaluate/pop/prefer-first-error pattern is repeated verbatim in both iteration paths. A small helper like `eval_loop_body(scope, ctx, body)` could DRY this, though Rust ownership may make it non-trivial.

- **Verbose comment block in validator** - `src/validator.rs:66-77` (Confidence: 65%) -- The 12-line comment explaining why dot-path type checking is skipped is thorough but could be condensed to 3-4 lines, with the rationale moved to a design doc or ADR. Long inline comments can be a code smell indicating the logic should be self-documenting.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 6/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new features (dot-notation, key-value iteration, frontmatter preservation) are well-structured individually, but two functions crossed the 50-line complexity threshold significantly. Both `evaluate_for` (96 lines) and `parse_interpolation_expr` (94 lines) would benefit from extraction of their new code paths into focused helper functions. The logic within each is linear and comprehensible, but the sheer length makes them harder to review, test in isolation, and modify safely in the future. Extracting helpers would bring the codebase back to the "explainable in 5 minutes" standard while preserving the same behavior.

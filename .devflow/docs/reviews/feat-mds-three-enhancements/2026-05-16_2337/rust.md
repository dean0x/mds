# Rust Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Unnecessary clone in `run_loop_body` due to slice-of-owned-values pattern** - `src/evaluator.rs:353-357`
**Confidence**: 80%
- Problem: `run_loop_body` accepts `bindings: &[(&str, Value)]` — callers construct a temporary array of owned `Value`s, then the function borrows and re-clones each value via `val.clone()`. This means every loop iteration allocates/moves values into the temp array, then clones them again into the scope. For simple strings this is negligible, but for deeply nested `Value::Object` maps it allocates twice per iteration.
- Fix: Accept owned values directly to eliminate the redundant clone:
```rust
fn run_loop_body(
    scope: &mut Scope,
    ctx: &mut EvalContext,
    body: &[Node],
    bindings: Vec<(&str, Value)>,  // owned, no clone needed
) -> Result<String, MdsError> {
    scope.push();
    for (name, val) in bindings {
        scope.set_var(name, val);  // move, not clone
    }
    // ...
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parse_dot_expr` QualifiedCall path does not validate `namespace` or `name`** - `src/parser.rs:533-534`
**Confidence**: 82%
- Problem: When parsing `{ns.func(args)}`, the `namespace` and `name` extracted from byte slicing are never checked with `is_valid_identifier()`. This allows syntactically invalid identifiers (e.g. `{123.foo()}` or `{a b.func()}`) to reach the evaluator, where they would fail with a confusing "undefined variable" error rather than a clear parse-time diagnostic.
- Fix: Add identifier validation after extracting namespace and name:
```rust
let namespace = content[..dot_pos].trim().to_string();
if !is_valid_identifier(&namespace) {
    return Err(MdsError::syntax_at(
        format!("invalid namespace in qualified call: '{namespace}'"),
        file, source, offset, len,
    ));
}
let name = rest_after_dot[..paren_pos].trim().to_string();
if !is_valid_identifier(&name) {
    return Err(MdsError::syntax_at(
        format!("invalid function name in qualified call: '{name}'"),
        file, source, offset, len,
    ));
}
```

## Suggestions (Lower Confidence)

- **`path_so_far` allocation on every resolve** - `src/evaluator.rs:112` (Confidence: 65%) — `path_so_far` allocates a `String` and appends on every field traversal. For hot paths with frequent shallow lookups (1-2 fields), this is a minor overhead. Could use a lazy approach (only allocate on error path) via a helper that reconstructs the path when needed.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED

The changes are well-structured: panicking `assert!()` calls replaced with idiomatic `.first().ok_or_else()` error returns, extracted helper functions (`run_loop_body`, `evaluate_for_key_value`, `parse_dot_expr`) reduce cognitive load, `MAX_DOT_SEGMENTS` provides defense-in-depth at both parse and evaluation boundaries, and the `strip_type_mds` enhancement correctly handles all three YAML quoting styles. All resource limits are properly bounded, no `.unwrap()` in production code, clippy is clean, and 349 tests pass. The single blocking MEDIUM finding (extra clone in loop body) is a performance nit, not a correctness issue, and does not warrant blocking the merge.

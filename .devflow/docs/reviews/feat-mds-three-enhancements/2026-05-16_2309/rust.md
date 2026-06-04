# Rust Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**`assert!()` in production code will panic instead of returning an error** - `src/evaluator.rs:321`, `src/evaluator.rs:339`
**Confidence**: 92%
- Problem: Two `assert!()` macros enforce the parser invariant that `condition` and `iterable` Vecs are non-empty. Unlike `debug_assert!`, these fire in release builds, causing a panic (process abort) instead of a graceful `Result::Err`. This is inconsistent with how the same invariant is handled in `src/validator.rs:27-29` and `src/validator.rs:54-56`, which use `.first().ok_or_else(...)` to return a proper error.
- Fix: Replace the panicking assertions with fallible checks, consistent with the validator:
```rust
// evaluator.rs:321-322 — replace:
assert!(!block.condition.is_empty(), "IfBlock.condition must be non-empty");
let value = resolve_dot_path(&block.condition[0], &block.condition[1..], scope)?;

// with:
let root = block.condition.first().ok_or_else(|| {
    MdsError::syntax("internal error: @if block has empty condition path")
})?;
let value = resolve_dot_path(root, &block.condition[1..], scope)?;
```
Apply the same pattern at line 339 for `block.iterable`.

### MEDIUM

**Misleading error message for nested dot-path field-not-found** - `src/evaluator.rs:108-111`
**Confidence**: 85%
- Problem: When resolving `{a.b.c}` and field `c` is not found on the sub-object `a.b`, the error reports "field 'c' not found on object 'a'" — citing the top-level root variable rather than the intermediate path. This is confusing for users with deeply nested objects.
- Fix: Build the path traversed so far and include it in the error message:
```rust
fn resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError> {
    let mut current = scope
        .get_var(root)
        .cloned()
        .ok_or_else(|| MdsError::undefined_var(root))?;
    let mut path_so_far = root.to_string();
    for field in fields {
        match current {
            Value::Object(ref map) => {
                current = map.get(field).cloned().ok_or_else(|| {
                    MdsError::syntax(format!(
                        "field '{field}' not found on object '{path_so_far}'"
                    ))
                })?;
                path_so_far.push('.');
                path_so_far.push_str(field);
            }
            _ => {
                return Err(MdsError::syntax(format!(
                    "cannot access field '{field}' on {typ} at '{path_so_far}'",
                    typ = current.type_name()
                )));
            }
        }
    }
    Ok(current)
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**resolve_dot_path clones all intermediate Values during traversal** - `src/evaluator.rs:100-123`
**Confidence**: 80%
- Problem: The function clones the root value from scope (line 103) and every intermediate value during field traversal (line 108). For deeply nested objects or large object values, this allocates unnecessarily. The function could return a `Cow<'_, Value>` or borrow the chain by returning a reference, since callers only need to call `.is_truthy()` (evaluate_if) or `.to_string()` (evaluate_expr) on the result — both of which work on `&Value`.
- Fix: This is an optimization opportunity, not a correctness bug. The current approach is sound and consistent with how `get_var().cloned()` is used elsewhere. If objects grow large, consider refactoring to return `&Value` with appropriate lifetime annotations. Acceptable for v0.1 given the template compiler workload.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Consider `BTreeMap` for `Value::Object`** - `src/value.rs:30` (Confidence: 65%) — The code sorts keys for display and iteration determinism. Using `BTreeMap<String, Value>` would provide sorted order intrinsically, avoiding the explicit sort in `evaluate_for` (line 363-364) and `Display::fmt` (line 236-237). Trade-off: slightly slower random access vs. guaranteed order.

- **`strip_type_mds` could use `lines().filter().collect()` pattern** - `src/lib.rs:342-361` (Confidence: 62%) — The manual push_str loop is clear but a functional approach would be more concise. Minor style suggestion only.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The implementation demonstrates solid Rust patterns overall: proper `Result` propagation with `?`, bounded loops with iteration limits, depth-limited recursion, and clean enum-based AST design. The `assert!()` in production code is the primary concern — it contradicts the established pattern in the same PR (validator.rs uses `.first().ok_or_else()`) and violates the principle of never panicking in library code. The error message issue is a usability concern that would improve the developer experience for users of nested objects.

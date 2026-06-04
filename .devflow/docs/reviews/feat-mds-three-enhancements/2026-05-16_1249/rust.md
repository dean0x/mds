# Rust Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`resolve_dot_path` uses `debug_assert` followed by unchecked indexing** - `src/evaluator.rs:100-101`
**Confidence**: 90%
- Problem: `resolve_dot_path` guards the `path.is_empty()` precondition with `debug_assert!`, which is stripped in release builds. The immediately following `let root = &path[0];` will panic with an index-out-of-bounds in release mode if an empty path ever reaches this function. While callers currently guarantee non-empty paths (the parser always produces at least one element), this is a defense-in-depth gap. Per the project's own conventions, the prior commit (b9587ce) specifically called out adding `debug_assert` preconditions for "defense-in-depth against empty-path invariant violations" -- but a `debug_assert` does NOT provide defense in depth since it is absent in release builds. The evaluator's `call_stack` LIFO check uses a full `assert!` for the same class of safety concern, establishing the project's precedent that safety-critical invariants use `assert!`, not `debug_assert!`.
- Fix: Either use `assert!` for true defense-in-depth (matching the `call_stack` precedent), or use `path.first()` with an error return:
```rust
fn resolve_dot_path(path: &[String], scope: &Scope) -> Result<Value, MdsError> {
    let root = path.first().ok_or_else(|| {
        MdsError::syntax("internal error: empty dot path".to_string())
    })?;
    // ...
}
```
This also applies to `src/validator.rs:25` and `src/validator.rs:49` which have the same `debug_assert` + `[0]` pattern.

**Redundant `Vec` allocation on every `MemberAccess` evaluation** - `src/evaluator.rs:164-166`
**Confidence**: 85%
- Problem: Every `Expr::MemberAccess` evaluation allocates a new `Vec<String>` by cloning the object name and all field names, only to pass it to `resolve_dot_path`. The same pattern repeats in `resolve_args` at line 207-209 and in `evaluate_if`/`evaluate_for`. Since `resolve_dot_path` only borrows the slice (`&[String]`), this allocation is unnecessary -- the path is already stored on the AST node and could be passed as a slice directly.
- Fix: Refactor `resolve_dot_path` to accept the root and fields separately, or change `MemberAccess` to store the full path as a single `Vec<String>` (like `IfBlock.condition` and `ForBlock.iterable` already do), eliminating the rebuild:
```rust
// Option A: Accept root + fields directly
fn resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError> {
    let mut current = scope.get_var(root).cloned()
        .ok_or_else(|| MdsError::undefined_var(root))?;
    for field in fields {
        // ...
    }
    Ok(current)
}

// Then in evaluate_expr:
Expr::MemberAccess { object, fields } => {
    let value = resolve_dot_path(object, fields, scope)?;
    // ...
}
```

**YAML non-string keys silently dropped** - `src/value.rs:66-70`
**Confidence**: 82%
- Problem: When converting YAML mappings to `Value::Object`, non-string keys are silently skipped (`if let serde_yml::Value::String(key) = k`). YAML allows integer, boolean, and null keys. A user who writes `42: answer` or `true: yes` in frontmatter will see those entries silently vanish with no warning or error. This contradicts the project's principle of failing honestly rather than silently discarding data.
- Fix: Return an error for non-string keys so users get a clear diagnostic:
```rust
for (k, v) in mapping {
    let key = match k {
        serde_yml::Value::String(s) => s,
        other => {
            return Err(MdsError::yaml_error(format!(
                "MDS only supports string keys in objects, found: {other:?}"
            )));
        }
    };
    let value = Self::from_yaml_inner(v, depth + 1)?;
    map.insert(key, value);
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`_file` and `_source` parameters unused in `parse_interpolation_expr`** - `src/parser.rs:500-501`
**Confidence**: 92%
- Problem: The `file` and `source` parameters were renamed to `_file` and `_source` (prefixed with underscore to suppress unused warnings) after removing the `dot_notation_error` function that was the sole consumer. These parameters were previously used to produce source-span-aware errors for dot-notation. Now all errors from this function use `MdsError::syntax(msg)` (bare variant without source location) instead of `MdsError::syntax_at(msg, file, source, offset, len)`. This is a regression in error quality -- invalid dot-path errors like `"invalid dot-path in interpolation: '{content}'"` at lines 544-546 could carry source location but don't.
- Fix: Restore the parameter names and use `syntax_at` for errors that have offset context:
```rust
fn parse_interpolation_expr(
    content: &str,
    offset: usize,
    file: &str,
    source: &str,
) -> Result<Interpolation, MdsError> {
    // ...
    // At line 544:
    return Err(MdsError::syntax_at(
        format!("invalid dot-path in interpolation: '{content}' -- each segment must be a valid identifier"),
        file, source, offset, content.len(),
    ));
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`HashMap<String, Value>` for object fields loses insertion order** - `src/value.rs:17`
**Confidence**: 85%
- Problem: `Value::Object` uses `std::collections::HashMap` which does not preserve insertion order. While the code sorts keys for display and iteration (in `Display` impl and `evaluate_for`), this means object field order from YAML frontmatter is not preserved. YAML mappings have a defined order, and users may expect fields to appear in their authored order. The `indexmap::IndexMap` crate is already a dependency (used for `IndexSet` in the resolver), so using `IndexMap<String, Value>` would preserve insertion order at zero added dependency cost while making the sort-for-display step optional.
- Note: This is a design choice, not a bug. The current code explicitly sorts where determinism matters. Flagging as informational since changing the underlying map type would be a broader refactor.

## Suggestions (Lower Confidence)

- **`strip_type_mds` matches `type: mds` with string manipulation instead of YAML parsing** - `src/lib.rs:342-358` (Confidence: 65%) -- The function filters lines matching `type:` prefix with `mds` value. This could false-positive on a YAML value like `my_type: mds_extended` if a key happened to end with `type:`. However, since this operates on raw frontmatter that was already parsed as valid YAML, the risk is minimal in practice.

- **`MemberAccess` in `Expr` and `Arg` duplicates the `(object, fields)` structure** - `src/ast.rs:63-67,80-84` (Confidence: 70%) -- Both `Expr::MemberAccess` and `Arg::MemberAccess` carry identical `{ object: String, fields: Vec<String> }` fields. A shared struct (e.g., `DotPath { object: String, fields: Vec<String> }`) would reduce duplication and ensure consistency. However, this is a minor structural preference.

- **Inconsistent path representation across AST nodes** - `src/ast.rs` (Confidence: 62%) -- `IfBlock.condition` and `ForBlock.iterable` store paths as `Vec<String>` (full path including root), while `Expr::MemberAccess` and `Arg::MemberAccess` split into `object: String` + `fields: Vec<String>`. This inconsistency forces the `evaluate_expr` and `resolve_args` functions to reconstruct the full path via `iter::once(object.clone()).chain(...)`. A unified representation would eliminate these rebuilds and reduce cognitive overhead.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Rust Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The Rust code is well-structured with thorough error handling, proper use of `Result` types throughout, and good test coverage (325 tests passing, zero clippy warnings). The object/map support follows existing patterns correctly -- `Value::Object` is integrated into all match sites (`is_truthy`, `type_name`, `Display`, `from_yaml`, `from_json`, `as_array`), `Arg::MemberAccess` is handled in all three required sites (parser, evaluator, validator), and resource limits are enforced for key-value iteration.

Conditions for approval:
1. Address the `debug_assert` + unchecked indexing in `resolve_dot_path` and `validate_node` -- either upgrade to `assert!` or use fallible indexing. This is the most important finding: the commit message explicitly describes these as "defense-in-depth" but `debug_assert` provides no defense in release builds.
2. Consider restoring source-span context to parser errors in `parse_interpolation_expr` (currently lost due to `_file`/`_source` parameter suppression).

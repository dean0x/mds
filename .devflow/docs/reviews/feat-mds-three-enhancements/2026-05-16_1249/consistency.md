# Consistency Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent path reconstruction pattern: `std::iter::once().chain().collect()` duplicated instead of passing `(object, fields)` directly to `resolve_dot_path`** - `src/evaluator.rs:164`, `src/evaluator.rs:207`
**Confidence**: 85%
- Problem: The `resolve_dot_path` function takes `&[String]` as a path, but the AST types `Expr::MemberAccess` and `Arg::MemberAccess` store `(object, fields)` as separate fields. Every call site must reconstruct the path via `std::iter::once(object.clone()).chain(fields.iter().cloned()).collect()`. This is done twice in `evaluate_expr` (line 164) and `resolve_args` (line 207). Meanwhile, `IfBlock.condition` and `ForBlock.iterable` already store `Vec<String>` and pass directly. This inconsistency between how the AST represents dot paths (two different shapes for the same concept) will get worse as more call sites are added.
- Fix: Either change `Expr::MemberAccess` and `Arg::MemberAccess` to store a single `path: Vec<String>` (matching `IfBlock.condition` and `ForBlock.iterable`), or add a helper function to eliminate the repeated reconstruction:
```rust
fn build_dot_path(object: &str, fields: &[String]) -> Vec<String> {
    std::iter::once(object.to_string())
        .chain(fields.iter().cloned())
        .collect()
}
```

### MEDIUM

**`from_yaml` silently skips non-string keys; `from_json` cannot encounter them -- missing documentation of the silent skip** - `src/value.rs:67`
**Confidence**: 82%
- Problem: The `from_yaml` YAML mapping handler silently skips non-string keys with a comment ("Skip non-string keys (YAML allows non-string keys, MDS does not)") but emits no warning. Every other error path in the converters returns an explicit `Err`. Silent data loss during frontmatter parsing could confuse users whose YAML uses integer or boolean keys. This is a consistency gap with the project's existing pattern of failing explicitly or at least warning on unsupported constructs (e.g., empty `@include` produces a warning).
- Fix: Either emit a warning (requires threading `warnings` through `from_yaml`, which may not be worthwhile) or return an error with a clear message:
```rust
if let serde_yml::Value::String(key) = k {
    let value = Self::from_yaml_inner(v, depth + 1)?;
    map.insert(key, value);
} else {
    return Err(MdsError::yaml_error(
        "YAML mapping keys must be strings in MDS"
    ));
}
```

**Dot-path representation is inconsistent across AST types** - `src/ast.rs:64-66`, `src/ast.rs:81-83`, `src/ast.rs:91`, `src/ast.rs:103`
**Confidence**: 84%
- Problem: The same concept ("dot-separated path") is represented two different ways in the AST:
  - `Expr::MemberAccess` and `Arg::MemberAccess`: `{ object: String, fields: Vec<String> }` (root + rest)
  - `IfBlock.condition` and `ForBlock.iterable`: `Vec<String>` (flat list including root)

  This means downstream code (validator, evaluator) must handle both representations. The evaluator reconstructs the flat form from the split form in two places. The parser already produces both forms -- `parse_if_block` and `parse_for_block` use `split('.')` into `Vec<String>`, while `parse_interpolation_expr` splits into `(object, fields)`.
- Fix: Standardize on one representation. `Vec<String>` (flat path) is simpler and matches `IfBlock`/`ForBlock`. Rename `object` and `fields` to a single `path: Vec<String>` in `MemberAccess` variants. This eliminates the reconstruction overhead and makes the AST uniform.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`_file` and `_source` parameters now unused in `parse_interpolation_expr`** - `src/parser.rs:500-501`
**Confidence**: 85%
- Problem: The old `dot_notation_error` function was the only consumer of `file` and `source` in `parse_interpolation_expr`. Its removal leaves these parameters as dead code, prefixed with `_` to suppress warnings. The caller still passes `self.file` and `self.source` at line 129. While this is functionally harmless, it creates a trap: a future developer may wonder whether these params should be used, and the `_` prefix obscures intent. Every other parser function that does NOT need source context simply does not take these parameters.
- Fix: Remove the `_file` and `_source` parameters from `parse_interpolation_expr` and stop passing them from `parse_body`:
```rust
fn parse_interpolation_expr(
    content: &str,
    offset: usize,
) -> Result<Interpolation, MdsError> {
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`HashMap` for `Object` requires sorting on every display/iteration** - `src/value.rs:17` (Confidence: 65%) -- The `Value::Object` uses `HashMap<String, Value>`, but the spec guarantees deterministic sorted output. Every iteration (`Display`, `@for key, value`) must sort. A `BTreeMap` would provide sorted order natively and eliminate the sort step. However, `HashMap` is the established pattern throughout the codebase (scope, functions, etc.), so this is a design tradeoff rather than a clear bug.

- **`ForBlock` key-value resource-limit check pattern differs from array path** - `src/evaluator.rs:357-363` (Confidence: 62%) -- The object iteration checks `map.len() > MAX_LOOP_ITERATIONS` before the loop but does not check `ctx.total_iterations` prior to starting (unlike the array path at line 399-406 which checks array length before cloning). This is minor since both paths check `ctx.total_iterations` inside the loop body.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR introduces object support, frontmatter preservation, and escape doc fixes consistently across all pipeline layers (parser, validator, evaluator, value, tests, spec). The new `Value::Object` variant is properly handled in all required sites (`from_yaml`, `from_json`, `Display`, `is_truthy`, `type_name`, `as_array`). Error handling follows existing patterns (bare constructors in evaluator, `_at` constructors in validator). The `Arg::MemberAccess` variant correctly updates all three match sites (parser, evaluator, validator) as required by the codebase contract. Test coverage is thorough with 20+ new integration tests.

The main consistency concern is the dual representation of dot-paths in the AST (`object + fields` vs `Vec<String>`), which forces reconstruction at every use site and will become a maintenance burden. The unused `_file`/`_source` parameters in `parse_interpolation_expr` should be cleaned up while this code is being touched.

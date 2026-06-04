# Architecture Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Validator skips static type checks on dot-path iterables, deferring validation to runtime** - `src/validator.rs:63`
**Confidence**: 85%
- Problem: The validator previously performed a static type check ensuring `@for` iterables are arrays. The new code at line 63 conditionally skips this check when `block.iterable.len() > 1` (i.e., for dot paths like `config.items`). This means dot-path iterables bypass static validation entirely -- type errors that were caught at validate time now surface only at evaluation time, with potentially less precise diagnostics. The pipeline's architectural invariant is that the validator catches errors before the evaluator runs so that users get span-aware diagnostics; this change weakens that guarantee for dot-path iterables.
- Fix: Consider resolving dot paths at validation time (similar to how `resolve_dot_path` works in the evaluator) to retain static type checking. If full resolution is too expensive at validation time, document the architectural decision that dot-path iterables are validated lazily.

```rust
// Current: skips check entirely for dot paths
if block.key_var.is_none() && block.iterable.len() == 1 && !matches!(iterable_val, ...) {
```

```rust
// Suggested: attempt dot-path resolution at validation time for deeper checks
// If the intermediate fields can be statically resolved (all root objects in scope
// are known), the type check should still apply.
```

### MEDIUM

**`HashMap<String, Value>` chosen for `Value::Object` -- nondeterministic iteration** - `src/value.rs:17`
**Confidence**: 82%
- Problem: `Value::Object` uses `HashMap<String, Value>` as its backing store. While the code compensates by sorting keys in `Display::fmt` and `evaluate_for` (key-value iteration), every consumer of `Value::Object` that iterates must independently remember to sort. This is a shallow module design -- the abstraction leaks its non-determinism to callers. If a future code path iterates without sorting, output becomes platform-dependent and non-reproducible.
- Fix: Consider using `indexmap::IndexMap<String, Value>` (already a dependency via `IndexSet`) or `BTreeMap<String, Value>` (stdlib). Both provide deterministic iteration order. `BTreeMap` gives alphabetical ordering for free (matching the sort in `evaluate_for`), eliminating the need for manual sorting in `Display::fmt` and the evaluator. This is a contained change since `Value::Object` construction only happens in `from_yaml`, `from_json`, and the `From<HashMap>` impl.

**`raw_frontmatter` stored on every `ResolvedModule` including imports** - `src/resolver.rs:43`
**Confidence**: 80%
- Problem: The new `raw_frontmatter: Option<String>` field is added to `ResolvedModule`, which is cached as `Arc<ResolvedModule>` for every resolved module -- including imported libraries. Only the root module's frontmatter is ever used (prepended by `compile_collecting_warnings` / `compile_str_collecting_warnings` in `lib.rs`). Imported modules carry frontmatter data that is captured but never consumed, adding memory overhead proportional to the number of imported modules. While the memory cost is small for typical usage, this violates the "deep modules" principle: the field's purpose is unclear from `ResolvedModule`'s interface since it has no method to access it and only the top-level public API uses it.
- Fix: Two options: (1) Capture `raw_frontmatter` only in a wrapper at the `lib.rs` level rather than storing it in every `ResolvedModule`, or (2) add a `pub(crate) fn raw_frontmatter(&self) -> Option<&str>` accessor and document that only the root module's value is used for output prepending. Option 1 is cleaner architecturally.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`parse_interpolation_expr` silences `_file` and `_source` parameters** - `src/parser.rs:500-501`
**Confidence**: 85%
- Problem: The parameters were renamed to `_file` and `_source` because the old `dot_notation_error` function (which used them for span-aware errors) was removed. However, the `MemberAccess` validation errors in this function now use bare `MdsError::syntax(...)` without source spans. When a user writes `{a.123.b}` (invalid dot segment), the error will lack file/line context. The prior version's `dot_notation_error` function attached source spans; this is a regression in diagnostic quality.
- Fix: Use `MdsError::syntax_at(message, file, source, offset, len)` for the invalid dot-path error at line 544, restoring the source-span information. This requires un-prefixing the `_file` and `_source` parameters.

```rust
// Before (suppressed): _file: &str, _source: &str,
// After (restored): file: &str, source: &str,
// Then use: MdsError::syntax_at(msg, file, source, offset, len)
```

**`resolve_dot_path` uses `debug_assert` for the empty-path precondition** - `src/evaluator.rs:100`
**Confidence**: 82%
- Problem: The function uses `debug_assert!(!path.is_empty(), ...)` followed by an unconditional `path[0]` access at line 101. In release builds the `debug_assert` is stripped, and if the precondition is violated, the code will panic with an index-out-of-bounds error instead of a clear compiler-bug diagnostic. The KNOWLEDGE.md documents that the project uses `assert!` (not `debug_assert!`) for safety-critical invariants (e.g., the LIFO check in evaluator line 268). The prior commit `1edbc92` specifically added `debug_assert` preconditions on Vec index access in `resolve_dot_path` and `validate_node`, but the codebase convention for evaluator invariants is `assert!`.
- Fix: Either (a) upgrade to `assert!` for consistency with the LIFO invariant precedent, or (b) use `.first().unwrap_or()` / `.get(0)` with a proper error path, matching the pattern used in `auto_detect_mds_file` (commit `1edbc92`). Option (b) is preferred since it returns a user-facing error rather than panicking.

```rust
fn resolve_dot_path(path: &[String], scope: &Scope) -> Result<Value, MdsError> {
    let root = path.first().ok_or_else(|| {
        MdsError::syntax("internal error: empty dot path (this is a compiler bug, please report it)")
    })?;
    // ...
}
```

**Same `debug_assert` concern in validator** - `src/validator.rs:25,49`
**Confidence**: 82%
- Problem: Both `validate_node` arms for `Node::If` (line 25) and `Node::For` (line 49) use `debug_assert!(!block.condition.is_empty(), ...)` and `debug_assert!(!block.iterable.is_empty(), ...)` followed by `&block.condition[0]` / `&block.iterable[0]`. Same risk as the evaluator: release builds will panic with an opaque index error instead of a diagnostic.
- Fix: Use `.first()` with a proper error return, consistent with the evaluator fix above.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No `as_object()` accessor on `Value` -- asymmetry with `as_array()`** - `src/value.rs:114-121`
**Confidence**: 85%
- Problem: `Value` provides `as_array() -> Option<&[Value]>` but has no corresponding `as_object() -> Option<&HashMap<String, Value>>`. This forces the evaluator to use pattern matching everywhere for object access (`match current { Value::Object(ref map) => ... }`), while array access has a clean method API. This is an asymmetry in the public interface that will grow as more code consumes objects.
- Fix: Add `pub fn as_object(&self) -> Option<&HashMap<String, Value>>` to `Value`, parallel to `as_array`.

## Suggestions (Lower Confidence)

- **YAML non-string keys silently dropped** - `src/value.rs:66-70` (Confidence: 70%) -- Non-string YAML keys are silently skipped with no warning. Users with integer or boolean YAML keys will get incomplete objects with no diagnostic. Consider emitting a warning via the warnings pipeline.

- **`strip_type_mds` is fragile for edge cases** - `src/lib.rs:342-358` (Confidence: 65%) -- The function filters lines matching `type: mds` using string inspection. Edge cases like `type:  mds` (double space), `type: MDS`, or `type: mds  # comment` may not be handled consistently with how YAML is parsed upstream.

- **Parser does not validate multi-level qualified calls** - `src/parser.rs:514-537` (Confidence: 62%) -- The parser only handles one dot level for `QualifiedCall` (e.g. `ns.func()`). An expression like `a.b.func()` will match the `rest_after_dot.find('(')` branch and treat `b.func` as the function name. While this may be intentional for v0.1 scope, it could produce confusing errors for users expecting nested namespace access.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 3 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The three enhancements (object support, frontmatter preservation, escape docs) are well-structured and follow the existing pipeline architecture. The `Value::Object` variant is added consistently across all required methods (`is_truthy`, `from_yaml`, `from_json`, `Display`, `type_name`, `as_array`). The parser correctly distinguishes `MemberAccess` from `QualifiedCall`. The frontmatter preservation is cleanly layered in `lib.rs` without leaking into the evaluator or resolver.

Conditions for approval:
1. Address the `debug_assert` + direct index access pattern in `resolve_dot_path` and `validate_node` -- these should either use `assert!` (codebase convention for safety-critical invariants) or use `.first()` with a proper error path to avoid opaque panics in release builds.
2. Restore source-span diagnostics for invalid dot-path errors in `parse_interpolation_expr` (the `_file`/`_source` suppression is a diagnostic regression).

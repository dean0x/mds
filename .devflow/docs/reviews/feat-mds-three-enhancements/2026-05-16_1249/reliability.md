# Reliability Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`debug_assert!` used for parser invariant instead of production assertion** (2 occurrences) -- `src/validator.rs:25`, `src/validator.rs:49`
**Confidence**: 90%
- Problem: `debug_assert!(!block.condition.is_empty(), ...)` and `debug_assert!(!block.iterable.is_empty(), ...)` guard against empty `Vec<String>` paths before indexing with `[0]`. These assertions are stripped in release builds. If the parser ever produces an empty `condition` or `iterable` Vec (e.g., from a future refactor or a new code path), `&block.condition[0]` / `&block.iterable[0]` will panic with an out-of-bounds index in production. The codebase already uses `assert!` (not `debug_assert!`) for similar LIFO-critical invariants in the evaluator's call stack (per KNOWLEDGE.md: "this assertion is safety-critical and runs in release mode").
- Fix: Replace `debug_assert!` with a runtime check that returns a proper `MdsError`:
```rust
// In place of: debug_assert!(!block.condition.is_empty(), "...");
if block.condition.is_empty() {
    return Err(MdsError::syntax(
        "internal error: @if condition path is empty (parser invariant violated)"
    ));
}
```
Alternatively, use `assert!` to match the existing pattern for safety-critical invariants in this codebase.

**No depth bound on `resolve_dot_path` traversal** -- `src/evaluator.rs:99-124`
**Confidence**: 82%
- Problem: `resolve_dot_path` iterates over `path[1..]` without any depth limit. While the parser constructs dot paths from user input that is bounded by interpolation token size, the function itself has no explicit guard. The existing `MAX_VALUE_DEPTH = 64` caps how deeply nested an object can be in YAML/JSON, which naturally limits traversal depth. However, the evaluator function lacks an explicit assertion of its own bound, violating the "every loop must have a fixed upper bound" principle from the reliability pattern. In practice the parser's `content.split('.')` operates on a single interpolation token (finite by source size), and the value tree is bounded by `MAX_VALUE_DEPTH`, so this is defense-in-depth rather than an active exploit vector.
- Fix: Add an explicit guard consistent with existing depth limits:
```rust
fn resolve_dot_path(path: &[String], scope: &Scope) -> Result<Value, MdsError> {
    debug_assert!(!path.is_empty(), "resolve_dot_path called with empty path");
    if path.len() > MAX_VALUE_DEPTH {
        return Err(MdsError::resource_limit(format!(
            "dot path depth {} exceeds maximum of {MAX_VALUE_DEPTH}",
            path.len()
        )));
    }
    // ... rest unchanged
}
```

### LOW

**`resolve_dot_path` uses `debug_assert!` for empty-path precondition** -- `src/evaluator.rs:100`
**Confidence**: 85%
- Problem: Similar to the validator issue above: `debug_assert!(!path.is_empty(), ...)` is followed by `let root = &path[0]` on the next line. In release mode, an empty `path` slice would panic at `path[0]`. The evaluator's own call stack invariant uses `assert!` (not `debug_assert!`) for the same category of safety-critical check.
- Fix: Convert to a runtime-safe check:
```rust
if path.is_empty() {
    return Err(MdsError::syntax("internal error: resolve_dot_path called with empty path"));
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`strip_type_mds` uses string matching instead of YAML parsing for `type: mds` detection** -- `src/lib.rs:342-358`
**Confidence**: 80%
- Problem: `strip_type_mds` uses line-by-line string filtering with `strip_prefix("type:")` to remove `type: mds` lines. This is fragile in edge cases: a YAML value like `type:mds` (no space after colon) or a multi-line YAML value starting with `type:` would be incorrectly matched or missed. However, since the frontmatter has already been parsed as valid YAML by `build_scope_from_frontmatter`, the raw text is known to be well-formed YAML. The real risk is a user with a non-`mds` value for `type:` (e.g., `type: mds-template`) that happens to have `mds` as the trimmed suffix -- but the filter checks exact equality (`v.trim() == "mds"`), which handles this correctly. The main gap is quoted values like `type: "mds"` which would NOT be stripped because `"mds"` != `mds` after trim. This could cause `type: "mds"` to leak into compiled output.
- Fix: Normalize the value comparison to handle YAML quoting:
```rust
.filter(|line| {
    !line
        .trim()
        .strip_prefix("type:")
        .is_some_and(|v| {
            let v = v.trim();
            v == "mds" || v == "\"mds\"" || v == "'mds'"
        })
})
```

## Pre-existing Issues (Not Blocking)

No critical pre-existing reliability issues found in reviewed files.

## Suggestions (Lower Confidence)

- **YAML non-string keys silently dropped** - `src/value.rs:66-70` (Confidence: 65%) -- `from_yaml_inner` silently skips non-string keys in YAML mappings. While MDS only supports string keys, silently dropping data could cause confusing "field not found" errors at runtime with no indication that the key was present but had a non-string type. A warning would improve debuggability.

- **Object iteration order depends on `HashMap` iteration for `into_iter` before sort** - `src/evaluator.rs:366` (Confidence: 62%) -- The `map.into_iter().collect()` followed by `entries.sort_by` is correct and produces deterministic output. However, using a `BTreeMap` instead of `HashMap` for `Value::Object` would eliminate the sort step entirely and guarantee deterministic iteration everywhere objects are used (including `Display`), not just in `@for` loops.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 1 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The changes demonstrate strong reliability practices overall. Object/map iteration has proper resource limit guards (`MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`), value depth is bounded by `MAX_VALUE_DEPTH`, and the double-fault error-preservation pattern is correctly applied to the new key-value iteration path. The main conditions for approval are:

1. Convert the three `debug_assert!` guards in `src/validator.rs` and `src/evaluator.rs` to runtime-safe checks (either `assert!` or error-returning `if` guards) to prevent index panics in release builds.
2. Consider adding an explicit depth bound on `resolve_dot_path` for defense-in-depth, though this is not strictly blocking since the value tree depth is already bounded upstream.

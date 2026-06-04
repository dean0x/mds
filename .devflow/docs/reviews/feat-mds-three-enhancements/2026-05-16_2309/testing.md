# Testing Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Missing test: runtime-supplied objects via `compile_str_with` with dot-access** - `tests/integration.rs:3338-3345`
**Confidence**: 82%
- Problem: The test `empty_object_is_falsy` passes a `Value::Object` via runtime vars, but there is no test exercising the happy path of passing a non-empty object via runtime vars and accessing its fields with dot-notation in the template body. All other object-access tests define objects in YAML frontmatter. This leaves a coverage gap for the `resolve_dot_path` path when the root value comes from `runtime_vars` rather than parsed YAML.
- Fix: Add a test like:
```rust
#[test]
fn runtime_vars_object_dot_access() {
    let source = "Hello {user.name}!\n";
    let mut vars = std::collections::HashMap::new();
    let mut user = std::collections::HashMap::new();
    user.insert("name".to_string(), mds::Value::String("Alice".to_string()));
    vars.insert("user".to_string(), mds::Value::Object(user));
    let result = mds::compile_str_with(source, None, Some(vars)).unwrap();
    assert!(result.contains("Hello Alice!"), "got: {result}");
}
```

**Missing test: `@for key, value in` over a dot-path object** - `tests/integration.rs:3289-3298`
**Confidence**: 80%
- Problem: `for_key_value_object` tests key-value iteration on a top-level object (`obj`), and `for_dot_path_iterable` tests array iteration on a nested path (`config.items`). But there is no test combining both: key-value iteration over a nested object via dot-path (e.g., `@for k, v in config.settings:`). The `evaluate_for` function resolves the iterable via `resolve_dot_path` before checking for `key_var`, so this specific combination has no direct coverage.
- Fix: Add a test:
```rust
#[test]
fn for_key_value_dot_path_object() {
    let source = "---\nconfig:\n  settings:\n    a: 1\n    b: 2\n---\n@for k, v in config.settings:\n{k}={v}\n@end\n";
    let result = mds::compile_str(source).unwrap();
    assert!(result.contains("a=1") && result.contains("b=2"), "got: {result}");
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Weak assertion uses `.contains()` where `assert_eq!` would catch regressions** - `tests/integration.rs:3231`, `3238`, `3351`
**Confidence**: 80%
- Problem: Tests like `object_single_level_access` assert `result.contains("val\n")` rather than asserting the full expected output. Since the frontmatter preservation feature was added in this same PR, the output now includes the frontmatter fences. Using `.contains()` would silently pass if the frontmatter had unexpected mutations (e.g., duplicated keys). Tests at this level of specificity would be more robust with exact equality.
- Fix: Consider using `assert_eq!` for the full expected output in the simpler cases:
```rust
#[test]
fn object_single_level_access() {
    let source = "---\nconfig:\n  key: val\n---\n{config.key}\n";
    let result = mds::compile_str(source).unwrap();
    assert_eq!(result, "---\nconfig:\n  key: val\n---\nval\n");
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing test: deeply nested object access (4+ levels)** - `tests/integration.rs:3235` (Confidence: 65%) -- `object_multi_level_access` tests 3 levels (`a.b.c`). A 4+ level test would exercise the recursive field traversal loop in `resolve_dot_path` one step further and verify there is no off-by-one in the path resolution.

- **Missing test: `MemberAccess` on intermediate object that resolves to an array** - (Confidence: 62%) -- No test exercises `{config.items}` where `items` is an array, verifying the "cannot interpolate" error path for arrays (currently only tested for objects).

- **No integration test for `parse_invalid_dot_path_interpolation_returns_error`** - `src/parser.rs:1102` (Confidence: 70%) -- The unit test covers `{a.123.b}` at the parser level, but there is no integration test calling `compile_str` with an invalid dot-path segment to verify the error propagates cleanly through the full pipeline with a user-friendly message.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The test suite is comprehensive for this PR: 44 new tests (25 integration + 7 parser unit + 12 value unit) covering object access, key-value iteration, dot-path resolution, frontmatter preservation, and error paths. The new tests follow existing patterns (compile_str/compile_str_with, clear assertions with diagnostic messages on failure). The three conditions above are minor gaps that should be addressed before merge to ensure thorough coverage of the runtime-vars-to-object path and the key-value dot-path combination. All 336 tests pass.

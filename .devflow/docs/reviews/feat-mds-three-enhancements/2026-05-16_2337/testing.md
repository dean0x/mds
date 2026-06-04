# Testing Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing tests for MAX_DOT_SEGMENTS depth guard (5 locations)** - Confidence: 92%
- `src/evaluator.rs:103`, `src/parser.rs:224`, `src/parser.rs:284`, `src/parser.rs:549`, `src/parser.rs:726`
- Problem: A new `MAX_DOT_SEGMENTS = 32` depth limit was added as a defense-in-depth guard in 5 separate locations (evaluator `resolve_dot_path`, parser `@if` condition, parser `@for` iterable, `parse_dot_expr`, and `parse_single_arg_inner`). None of these error paths have any test coverage. These guards were explicitly added for reliability — untested guards may silently regress.
- Fix: Add integration tests that exercise paths with >32 dot segments and assert the expected error message. For example:
```rust
#[test]
fn dot_path_exceeds_max_segments() {
    // Build a source with >32 dot segments in an interpolation
    let deep_path = (0..33).map(|i| format!("f{i}")).collect::<Vec<_>>().join(".");
    let source = format!("{{{deep_path}}}\n");
    let result = mds::compile_str(&source);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("exceeds maximum"), "got: {err}");
}

#[test]
fn if_condition_exceeds_max_dot_segments() {
    let deep_path = (0..33).map(|i| format!("f{i}")).collect::<Vec<_>>().join(".");
    let source = format!("@if {deep_path}:\ntrue\n@end\n");
    let result = mds::compile_str(&source);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("exceeds maximum"), "got: {err}");
}
```

### MEDIUM

**Improved error messages not asserted in tests** - `src/evaluator.rs:118`, `src/evaluator.rs:126` - Confidence: 82%
- Problem: The error message in `resolve_dot_path` was improved to report the full traversed path (`path_so_far`) instead of just the root variable name. For example, accessing `a.b.missing` now reports `"field 'missing' not found on 'a.b'"` instead of the old `"field 'missing' not found on object 'a'"`. The existing test `object_field_not_found` only checks for `err.contains("not found") && err.contains("missing")` — it does not verify that the improved path is present. This means the quality improvement could regress without detection.
- Fix: Strengthen the `object_field_not_found` test or add a multi-level variant:
```rust
#[test]
fn object_field_not_found_shows_full_path() {
    let source = "---\na:\n  b:\n    c: deep\n---\n{a.b.missing}\n";
    let result = mds::compile_str(source);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("'a.b'"), "should show traversed path, got: {err}");
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**for_key_value_dot_path_object uses weak assertions** - `tests/integration.rs:3387-3391` - Confidence: 84%
- Problem: The new `for_key_value_dot_path_object` test uses `assert!(result.contains(...))` pattern for verifying output, while the adjacent tests (`object_single_level_access`, `object_multi_level_access`) were explicitly upgraded from `contains` to `assert_eq` in this same PR. The inconsistency means the test does not verify the frontmatter preservation behavior (which is another feature in this PR).
- Fix: Use `assert_eq` with the full expected output, consistent with the assertion-strengthening done elsewhere in this PR:
```rust
assert_eq!(
    result,
    "---\nconfig:\n  settings:\n    theme: dark\n    lang: en\n---\nlang=en\ntheme=dark\n",
    "got: {result}"
);
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **No negative test for run_loop_body double-fault** - `src/evaluator.rs:349-362` (Confidence: 65%) — The extracted `run_loop_body` helper has documented double-fault behavior (preferring render error over pop error). This invariant is not directly tested, though it is indirectly covered by existing loop tests.

- **No test for key-value iteration on very large object (resource limit)** - `src/evaluator.rs:373` (Confidence: 62%) — The `MAX_LOOP_ITERATIONS` guard on object entry count is not tested for the key-value path specifically (only the array path has coverage).

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR has solid test coverage for the happy paths of the three enhancements (object access, frontmatter preservation, YAML quoting variants). Assertion quality was improved by upgrading to `assert_eq`. However, the new defense-in-depth `MAX_DOT_SEGMENTS` limit — added in 5 locations — has zero test coverage. Since these guards were explicitly introduced for reliability (preventing unbounded recursion/depth), they should be verified to actually fire.

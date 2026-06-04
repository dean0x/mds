# Code Review Summary

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16
**Reviewers**: Security (9/10), Architecture (8/10), Performance (8/10), Complexity (8/10), Consistency (9/10), Regression (9/10), Testing (7/10), Reliability (9/10), Rust (8/10)

## Merge Recommendation: CHANGES_REQUESTED

The PR introduces solid reliability and security improvements but **requires addressing the missing test coverage for MAX_DOT_SEGMENTS depth guards** before merge. Additionally, 2 medium-priority issues should be fixed: cross-layer constant import and error message wording inconsistency.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 2 | 4 | 0 | **6** |
| Should Fix | 0 | 0 | 3 | 0 | **3** |
| Pre-existing | 0 | 0 | 3 | 0 | **3** |

---

## Blocking Issues (Must Fix Before Merge)

### HIGH: Missing tests for MAX_DOT_SEGMENTS depth guard
**Reviewers**: Testing (92% confidence)
**Location**: `src/evaluator.rs:103`, `src/parser.rs:224,284,549,726`

A new `MAX_DOT_SEGMENTS = 32` limit was added at 5 locations as a defense-in-depth guard (evaluator runtime, plus parser parse-time guards for `@if`, `@for`, interpolations, and function arguments). **Zero test coverage exists for these error paths.** Since these guards were explicitly introduced for reliability, they should be verified to actually trigger.

**Fix**: Add integration tests that construct paths with >32 segments and verify the expected error message:
```rust
#[test]
fn dot_path_exceeds_max_segments() {
    let deep_path = (0..33).map(|i| format!("f{i}")).collect::<Vec<_>>().join(".");
    let source = format!("{{{deep_path}}}\n");
    let result = mds::compile_str(&source);
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("exceeds maximum"));
}

#[test]
fn if_condition_exceeds_max_dot_segments() {
    let deep_path = (0..33).map(|i| format!("f{i}")).collect::<Vec<_>>().join(".");
    let source = format!("@if {deep_path}:\ntrue\n@end\n");
    let result = mds::compile_str(&source);
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("exceeds maximum"));
}
```

---

### HIGH: Cross-layer constant import creates layering violation
**Reviewers**: Architecture (82% confidence)
**Location**: `src/evaluator.rs:103` imports `MAX_DOT_SEGMENTS` from `src/parser.rs`

The evaluator depends on a parser-internal constant, creating a cross-layer dependency. While the defense-in-depth check is sound, the constant should be defined in a shared location to respect architectural boundaries.

**Fix**: Define the constant once in a shared module:
```rust
// src/limits.rs (new)
pub(crate) const MAX_DOT_SEGMENTS: usize = 32;

// src/parser.rs
use crate::limits::MAX_DOT_SEGMENTS;

// src/evaluator.rs
use crate::limits::MAX_DOT_SEGMENTS;
```

---

### MEDIUM: Inconsistent error message wording for MAX_DOT_SEGMENTS
**Reviewers**: Consistency (85% confidence)
**Location**: `src/evaluator.rs:105` vs `src/parser.rs:226,286,551,728`

The evaluator uses "dot path depth exceeds maximum of {N} segments" while the parser uses "exceeds maximum segment count of {N}". 

**Fix**: Align all five messages to parser's more natural phrasing:
```rust
// evaluator.rs:104-106
return Err(MdsError::syntax(format!(
    "dot path exceeds maximum segment count of {MAX_DOT_SEGMENTS}"
)));
```

---

### MEDIUM: String allocation on every resolve_dot_path call
**Reviewers**: Performance (82% confidence)
**Location**: `src/evaluator.rs:112`

`path_so_far` is allocated as a `String` on every call to `resolve_dot_path`, even on the happy path where no error occurs. This is wasteful for successful traversals of 1-2 segments in hot paths.

**Fix**: Use lazy allocation — only build the path string when an error occurs:
```rust
// Omit path_so_far allocation from function start
for (i, field) in fields.iter().enumerate() {
    match current {
        Value::Object(ref map) => {
            current = map.get(field).cloned().ok_or_else(|| {
                let path_so_far = std::iter::once(root)
                    .chain(fields[..i].iter().map(|s| s.as_str()))
                    .collect::<Vec<_>>()
                    .join(".");
                MdsError::syntax(format!(
                    "field '{field}' not found on '{path_so_far}'"
                ))
            })?;
        }
        // ... rest of error handling with lazy path construction
    }
}
```

---

## Should-Fix Issues (Recommended Before Merge)

### MEDIUM: Assertion quality needs strengthening in for_key_value_dot_path_object test
**Reviewers**: Testing (84% confidence)
**Location**: `tests/integration.rs:3387-3391`

This test uses weak `assert!(result.contains(...))` pattern while adjacent tests were upgraded to `assert_eq` in this PR. The inconsistency means the test doesn't verify frontmatter preservation behavior.

**Fix**: Use `assert_eq` with full expected output:
```rust
assert_eq!(
    result,
    "---\nconfig:\n  settings:\n    theme: dark\n    lang: en\n---\nlang=en\ntheme=dark\n",
    "got: {result}"
);
```

---

### MEDIUM: Improved error messages not verified in tests
**Reviewers**: Testing (82% confidence)
**Location**: `src/evaluator.rs:118,126`

The error message for field-not-found was improved to report the full traversed path (`a.b.missing` now shows `a.b` in error). The existing `object_field_not_found` test only checks for generic presence of "not found" and "missing", not the improved path context.

**Fix**: Strengthen the test to verify the full path appears:
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

---

### MEDIUM: Unnecessary clone in run_loop_body
**Reviewers**: Rust (80% confidence)
**Location**: `src/evaluator.rs:353-357`

The function accepts `bindings: &[(&str, Value)]` and re-clones each value, creating double allocation per iteration. For deeply nested objects, this wastes resources.

**Fix**: Accept owned values directly:
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

---

## Pre-existing Issues (Informational)

| Issue | Location | Severity | Note |
|-------|----------|----------|------|
| `validate_file_type` accepts unquoted `type: mds` only | `src/resolver.rs:717-722` | MEDIUM | Safe asymmetry (strict admission). Could add quoted variant matching for consistency. |
| `evaluate_for` at 64 lines (exceeds 50-line warning) | `src/evaluator.rs:405-468` | MEDIUM | Already significantly improved from pre-extraction state. Consider extracting `evaluate_for_array` in follow-up. |
| `parse_args_inner` at 59 lines with 4 nesting levels | `src/parser.rs:634-692` | MEDIUM | State machine pattern is inherently sequential. Extract shared `validate_dot_path_parts` if further growth occurs. |
| `parser.rs` file at 1321 lines total | `src/parser.rs` | MEDIUM | Consider splitting tests or modules when next growth occurs. |
| `invoke_function` at 57 lines | `src/evaluator.rs:232-288` | MEDIUM | Coherent in purpose, well-commented. Low-priority improvement. |

---

## Strengths Observed

✅ **Security**: 9/10 — Security-positive changes (assert→Result, depth guards, YAML quote handling). No injection vectors or trust boundary violations.

✅ **Reliability**: 9/10 — Exemplary reliability engineering. All loops bounded, assertion density improved, defense-in-depth guards at both parse and eval layers.

✅ **Regression**: 9/10 — All 349 tests pass. Public API unchanged. Behavioral improvements (assert removal) are strictly safer.

✅ **Consistency**: 9/10 — Strong pattern adherence. One minor cross-module message wording inconsistency.

✅ **Error Handling**: Systematic replacement of `assert!()` with `.first().ok_or_else()` pattern. Proper `Result` error propagation throughout.

✅ **Code Clarity**: Helper extraction (`run_loop_body`, `evaluate_for_key_value`, `parse_dot_expr`) improves SRP and readability. Well-commented and named.

⚠️ **Testing**: 7/10 — Happy paths well-covered but new defense-in-depth guards (MAX_DOT_SEGMENTS) lack test coverage.

⚠️ **Performance**: 8/10 — One hot-path optimization opportunity (lazy path_so_far allocation) identified. Minor, non-blocking.

---

## Action Plan

1. **Add tests for MAX_DOT_SEGMENTS guards** (blocking) — 5 new integration tests covering interpolation, @if, @for, and function arguments with >32 segments
2. **Move MAX_DOT_SEGMENTS to limits module** (blocking) — Fix cross-layer dependency
3. **Fix error message wording** (blocking) — Standardize all 5 occurrences to parser's phrasing
4. **Fix for_key_value_dot_path_object assertions** (recommended) — Use `assert_eq` for consistency
5. **Strengthen object_field_not_found test** (recommended) — Verify full traversed path in error
6. **Optimize path_so_far allocation** (recommended) — Lazy construction in error path only
7. **Eliminate clone in run_loop_body** (recommended) — Accept owned bindings vector

---

## Summary

This PR is **well-engineered** with strong reliability improvements (assert removal, depth guards, better error messages). The refactoring is clean and follows existing patterns. However, **merge is blocked by two issues**:

1. **Missing test coverage for new MAX_DOT_SEGMENTS guards** — These defensive checks should be verified to work
2. **Cross-layer constant import** — Architectural boundary violation that should be fixed

Additionally, **error message wording should be made consistent** across the 5 guard locations.

Once these three blocking issues are resolved, the PR is merge-ready.

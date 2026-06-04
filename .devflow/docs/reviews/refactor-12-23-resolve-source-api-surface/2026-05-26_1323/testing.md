# Testing Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23

## Issues in Your Changes (BLOCKING)

_(none)_

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing non-UTF-8 `base_dir` rejection test for `compile_str_with` / `check_str_with`** - `crates/mds-core/src/lib.rs:216`
**Confidence**: 82%
- Problem: The `resolve_base_dir` function gained a new UTF-8 validation path (line 216: `MdsError::io("base_dir path is not valid UTF-8")`), but the non-UTF-8 path rejection tests (`check_rejects_non_utf8_path`, `compile_rejects_non_utf8_path`) only exercise `path_to_str` via `check()` and `compile()`. The `resolve_base_dir` UTF-8 rejection path (reached by passing a non-UTF-8 `base_dir` to `compile_str_with`, `check_str_with`, `compile_str_collecting_warnings`, or `check_str_collecting_warnings`) has no dedicated test. Since this is a new error path introduced by this PR, it should be tested directly.
- Fix: Add a `#[cfg(unix)]` test that passes a non-UTF-8 `OsStr` as `base_dir` to `compile_str_with`:
```rust
#[cfg(unix)]
#[test]
fn compile_str_with_rejects_non_utf8_base_dir() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let invalid: &OsStr = OsStrExt::from_bytes(b"/tmp/\xFF\xFE");
    let path = Path::new(invalid);

    let err = mds::compile_str_with("Hello!\n", Some(path), None)
        .expect_err("expected error for non-UTF-8 base_dir");
    let msg = err.to_string();
    assert!(
        msg.contains("not valid UTF-8"),
        "error should mention 'not valid UTF-8', got: {msg}"
    );
}
```

## Pre-existing Issues (Not Blocking)

_(none)_

## Suggestions (Lower Confidence)

- **Missing concurrent rejection test for `LazyInit`** - `packages/bundler-utils/__test__/lazy-init.spec.mjs` (Confidence: 65%) -- When multiple concurrent `get()` calls race and the factory rejects, all callers should receive the rejection and the pending promise should be cleared. A `Promise.allSettled` test with a rejecting factory would exercise this edge case.

- **`module_cache_resolve_source_accepts_str` test uses `unwrap()` on `current_dir()`** - `crates/mds-core/tests/api_surface.rs:712-716` (Confidence: 62%) -- The test calls `std::env::current_dir().unwrap().to_str().unwrap()` which could fail in unusual CI environments (non-UTF-8 cwd or missing cwd). Given this is a test file and the pattern is common in Rust tests, this is a minor concern.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | 0 |
| Pre-existing | - | - | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The test suite for this PR is strong overall:

1. **LazyInit (8 tests)**: Comprehensive coverage including single-call guarantee, concurrent dedup, rejection retry, reset semantics, TOCTOU safety (reset during in-flight get), void/null edge cases. The TOCTOU test (line 76-108) is particularly well-designed and directly validates the generation counter mechanism.

2. **API surface tests (4 new tests)**: The compile-time signature tests (`module_cache_resolve_path_accepts_str`, `module_cache_resolve_source_accepts_str`) effectively act as type-level guards -- they fail at compile time if the signatures revert. The non-UTF-8 rejection tests cover the `path_to_str` boundary correctly.

3. **Existing test suites validate the refactor**: The `transform.spec.mjs` tests (init-once, concurrent init, poisoned-promise retry) already validate the `LazyInit` integration in `createMdsTransformer` without changes needed. The webpack loader test correctly adds `await` to `_setTransformerForTesting` to match the new async signature.

4. **Prior resolutions addressed**: The cycle-1 resolution items (non-UTF-8 path rejection tests, reset-during-in-flight-get test) are present and passing.

The single should-fix is the missing `resolve_base_dir` non-UTF-8 path, which is a new error branch introduced by this PR that has no test coverage. The condition for merge: add the `resolve_base_dir` non-UTF-8 test or explicitly acknowledge the gap.

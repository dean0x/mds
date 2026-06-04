# Rust Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T12:07
**PR**: #33

## Issues in Your Changes (BLOCKING)

No CRITICAL or HIGH issues found.

### MEDIUM

**Repeated UTF-8 validation boilerplate (4 occurrences)** -- Confidence: 85%
- `lib.rs:180-182`, `lib.rs:295-297`, `lib.rs:342-344`, `lib.rs:550-552`
- Problem: Four public functions (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`) contain identical 3-line `path.to_str().ok_or_else(...)` boilerplate. This violates DRY and means the error message and conversion logic must be kept in sync across all sites. If the error format or variant changes, four call sites need updating.
- Fix: Extract a private helper function:
```rust
/// Convert an `AsRef<Path>` to a UTF-8 `&str`, returning an explicit error
/// for non-UTF-8 paths instead of silently corrupting via `display()`.
fn path_to_str(path: &Path) -> Result<&str, MdsError> {
    path.to_str()
        .ok_or_else(|| MdsError::io("path is not valid UTF-8"))
}
```
Then each call site becomes:
```rust
let path_str = path_to_str(path)?;
```

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

No issues found.

## Suggestions (Lower Confidence)

- **Consider accepting `&str` in public API signatures** - `lib.rs:108,291,338,546` (Confidence: 65%) -- The public functions (`compile`, `check`, etc.) still accept `impl AsRef<Path>` and immediately convert to `&str`. Since the internal API now operates on `&str`, the public API could accept `impl AsRef<str>` (or `&str` directly) to push UTF-8 responsibility to callers. However, `AsRef<Path>` is the Rust-idiomatic choice for filesystem-facing APIs and enables `Path::new("file.mds")` ergonomics, so the current design is defensible. The PR description and feature knowledge confirm this is intentional: UTF-8 enforcement at the boundary in `lib.rs` with `&str` internally.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This is a clean, well-motivated refactoring. The core change -- replacing `path.display().to_string()` (which silently corrupts non-UTF-8 paths with U+FFFD replacement characters) with `path.to_str().ok_or_else(...)` (which returns an explicit error) -- is correct and follows Rust best practices for boundary validation. The `&Path` to `&str` signature change in `resolve_path` and `resolve_source` is sound because the internal `FileSystem` trait already operates on `&str` keys; the old code was converting through `display()` as a lossy bridge.

Key observations:
- Error handling uses `?` propagation with `Result` types throughout -- no `unwrap()` in library code.
- The `resolve_base_dir` function properly handles the `current_dir()` fallback with chained `and_then` for UTF-8 validation.
- The `use std::path::Path` import was correctly removed from `resolver.rs` since `Path` is no longer referenced there.
- The removed `const _` assertions from `cli_import_pattern_works` were redundant duplicates -- the same assertions exist in a dedicated test at lines 181-183.
- New tests validate both the `&str` signature (`module_cache_resolve_path_accepts_str`) and the happy-path behavior (`module_cache_resolve_source_accepts_str`).
- All 314 tests pass.

The only condition for approval is addressing the DRY violation of the repeated 4-site UTF-8 conversion pattern, which is MEDIUM severity and straightforward to fix with a one-line helper.

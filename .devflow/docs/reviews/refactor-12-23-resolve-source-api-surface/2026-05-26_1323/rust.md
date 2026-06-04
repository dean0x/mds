# Rust Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23
**Prior Resolutions**: Cycle 1 resolved 6/6 issues (extract path_to_str helper, add non-UTF-8 tests, etc.)

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

No should-fix issues found.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`load_vars_file` uses `path.display()` which silently corrupts non-UTF-8 paths** - `crates/mds-core/src/lib.rs:814,819,825`
**Confidence**: 85%
- Problem: The `load_vars_file` function uses `path.display()` in three error-message format strings (lines 814, 819, 825). This is the exact same silent UTF-8 corruption pattern that this PR eliminates from `resolve_path` and `resolve_source`. `Display` on `Path` uses lossy replacement characters for non-UTF-8 bytes, so error messages could show garbled paths.
- Impact: Informational only -- the function already accepts `&Path` and does not pass the path into the resolver's `&str`-based API, so there is no data-corruption risk for the compiled output. The issue is limited to error messages containing replacement characters instead of a clear "not valid UTF-8" diagnostic. Severity is reduced because `load_vars_file` reads JSON vars (not MDS source), so the scope of impact is smaller.
- Fix: Apply the same `path_to_str` pattern used elsewhere to produce explicit errors, or continue using `display()` for error diagnostics only (acceptable trade-off). Consider in a follow-up PR:
  ```rust
  pub fn load_vars_file(path: &Path) -> Result<HashMap<String, Value>, MdsError> {
      let path_str = path_to_str(path)?; // fail-fast for non-UTF-8
      let bytes = std::fs::read(path)
          .map_err(|e| MdsError::io(format!("cannot read vars file {path_str}: {e}")))?;
      // ... remainder uses path_str in error messages ...
  }
  ```

## Suggestions (Lower Confidence)

- **`resolve_base_dir` allocates a `String` where a `Cow<str>` could suffice** - `crates/mds-core/src/lib.rs:212` (Confidence: 65%) -- The `Some(d)` arm calls `.to_str().map(str::to_owned)`, producing a heap allocation even when the caller could borrow. Since `resolve_source` immediately borrows the result via `&dir`, a `Cow<'_, str>` return could avoid the allocation in the `Some` path. Minor optimization; current approach is correct and clear.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Rust Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This is a well-executed refactoring that eliminates a real correctness hazard -- silent UTF-8 corruption via `Path::display().to_string()` at the resolver boundary. The changes are surgically scoped:

1. **API boundary shift is correct**: `resolve_path` and `resolve_source` now accept `&str` instead of `&Path`, pushing the `Path -> &str` conversion to the public API functions in `lib.rs` via the new `path_to_str` helper. This is the right design -- the resolver operates on strings internally, and the conversion should happen at the boundary with explicit error handling.

2. **Error handling follows Rust idioms**: `path_to_str` returns `Result<&str, MdsError>` with `?` propagation. No `.unwrap()` in library code. The `thiserror`-based `MdsError::Io` variant is used consistently.

3. **Borrow discipline is good**: `path_to_str` returns `&str` (borrows from the input `Path`), avoiding unnecessary allocation. `resolve_base_dir` allocates a `String` only when needed (the `None`/cwd path requires ownership).

4. **Removed `PathBuf` import**: The `use std::path::PathBuf` import was correctly removed since it is no longer needed. Clean diff.

5. **Test coverage is thorough**: Two `#[cfg(unix)]` tests verify non-UTF-8 path rejection for both `check` and `compile`. Two additional tests document the `&str` signatures at the type level. The `expect_err` cleanup in `load_vars_str_rejects_oversized_input` is a minor readability improvement.

6. **Clippy clean, all 316 tests pass**: No warnings, no regressions.

7. **Prior cycle resolutions verified**: The `path_to_str` helper extraction (DRY), non-UTF-8 rejection tests, and `expect_err` assertion improvements from Cycle 1 are all present and correctly integrated.

The single pre-existing issue (`load_vars_file` still using `display()`) is informational and not introduced by this PR. It is a natural follow-up for completeness but does not affect correctness of the compiled output.

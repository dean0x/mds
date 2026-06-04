# Complexity Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T12:07

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Duplicated UTF-8 conversion boilerplate (4 occurrences)** -- Confidence: 90%
- `crates/mds-core/src/lib.rs:180-182`, `crates/mds-core/src/lib.rs:295-297`, `crates/mds-core/src/lib.rs:342-344`, `crates/mds-core/src/lib.rs:550-552`
- Problem: Four public functions (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`) each contain the identical 3-line pattern:
  ```rust
  let path_str = path
      .to_str()
      .ok_or_else(|| MdsError::io("path is not valid UTF-8"))?;
  ```
  This is textual duplication of the same conversion logic with the same error message. Each call site also follows the same 5-line boilerplate pattern: convert path, unwrap vars, create cache, create warnings vec, call resolve_path. The UTF-8 conversion block is minor individually, but four copies in one file risks drift (e.g., one site gets a different error message or handling).
- Fix: Extract a small helper function that encapsulates the `&Path -> &str` conversion:
  ```rust
  fn path_to_str(path: &Path) -> Result<&str, MdsError> {
      path.to_str()
          .ok_or_else(|| MdsError::io("path is not valid UTF-8"))
  }
  ```
  Then each call site becomes `let path_str = path_to_str(path)?;` -- a single line instead of three, and the error message is centralized.

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

### MEDIUM

**High API surface repetition in lib.rs** -- Confidence: 85%
- `crates/mds-core/src/lib.rs` (entire file, ~1114 lines)
- Problem: The public API of `lib.rs` consists of many nearly-identical functions: `compile`, `compile_collecting_warnings`, `compile_with_deps`, `compile_str`, `compile_str_with`, `compile_str_collecting_warnings`, `compile_str_with_deps`, `check`, `check_str`, `check_str_with`, `check_collecting_warnings`, `check_str_collecting_warnings`, `compile_virtual`, `compile_virtual_collecting_warnings`, `compile_virtual_with_deps`, `check_virtual`, `check_virtual_collecting_warnings`. Each follows the same pattern: unwrap vars, create cache, create warnings vec, resolve, optionally build output. This is a combinatorial explosion of convenience wrappers. The file is above the 500-line critical threshold at ~1114 lines. This is pre-existing and not introduced by this PR, but the PR adds to the pattern by inserting the UTF-8 conversion into four of these functions.
- Fix: Consider a builder pattern or options struct in a future PR to reduce the combinatorial surface. Not blocking for this refactor.

## Suggestions (Lower Confidence)

- **`resolve_base_dir` nesting depth** - `crates/mds-core/src/lib.rs:214-228` (Confidence: 65%) -- The `None` arm chains `.map_err().and_then(|p| p.to_str().ok_or_else(...).map(str::to_owned))` reaching 3 levels of combinator nesting. Readable for Rust-experienced developers but on the boundary of clarity. Could flatten with an early `let p = std::env::current_dir().map_err(...)?;` followed by a direct `p.to_str().ok_or_else(...).map(str::to_owned)`.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

This PR is a net complexity improvement: it extracts `LazyInit<T>` from inline init/retry logic (reducing cyclomatic complexity in `createMdsTransformer` and `webpack-loader`), simplifies the `resolve_path`/`resolve_source` signatures from `&Path` to `&str` (eliminating the lossy `path.display().to_string()` call in the resolver), and reduces the webpack-loader from a manual 3-state machine to a clean `LazyInit` wrapper. The one condition is the duplicated UTF-8 conversion boilerplate -- extracting a `path_to_str` helper would prevent drift across the four identical call sites.

# Rust Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18
**PR**: #22 — SerializedError/SerializedSpan, CompileOutput with dependency tracking, FileSystem::canonicalize()

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`compile_with_deps` bypasses `FileSystem::canonicalize()` trait method** - `crates/mds-core/src/lib.rs:521-524`
**Confidence**: 85%
- Problem: `compile_with_deps` calls `path.canonicalize()` (i.e. `std::path::Path::canonicalize`) directly to compute the entry key for dependency filtering. This is inconsistent with the PR's stated goal of routing canonicalization through the `FileSystem` trait (the fix for #21). While `compile_with_deps` always uses `ModuleCache::new()` (which creates a `NativeFs`), the direct `std::path` call means the entry-key computation is coupled to the OS filesystem and cannot be overridden by a custom `FileSystem` backend.
- Impact: If a consumer creates a `ModuleCache::with_fs(custom_fs)` and then tries to replicate what `compile_with_deps` does, the entry-key filtering logic would diverge. The inconsistency also makes the abstraction leaky — half the canonicalization goes through the trait, half does not.
- Fix: Use the cache's filesystem trait method instead:
  ```rust
  let entry_key = cache.fs.canonicalize(&path.display().to_string())
      .unwrap_or_else(|_| path.display().to_string());
  ```
  This requires either making `fs` accessible or adding a delegate method on `ModuleCache`. Alternatively, since `compile_with_deps` always uses `NativeFs`, the current approach is functionally correct for its current scope — document the limitation if not fixing now.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Consider `Eq` derive on `SerializedSpan` and `SerializedError`** - `crates/mds-core/src/error.rs:13,24` (Confidence: 65%) — Both types derive `PartialEq` but not `Eq`. Since all fields are `String`, `Option<String>`, `usize`, and `Option<usize>` (all `Eq`), deriving `Eq` is free and enables use as `HashMap` keys or in `assert!` macros that require `Eq`. This is a minor API completeness point, not a bug.

- **`dependencies()` allocates a new `Vec<String>` with cloned keys** - `crates/mds-core/src/resolver.rs:112-113` (Confidence: 60%) — `self.modules.keys().cloned().collect()` clones every key string on each call. For typical use (called once after compilation), this is fine. If the method were called in a hot loop, returning `impl Iterator<Item = &str>` or `&[String]` (via `IndexMap::keys()`) would avoid allocation. Current usage pattern makes this a non-issue in practice.

- **`CompileOutput` missing `#[non_exhaustive]`** - `crates/mds-core/src/lib.rs:67-76` (Confidence: 70%) — `MdsError` already carries `#[non_exhaustive]` for forward compatibility. `CompileOutput` is a new public struct that may gain fields in the future (e.g. `source_map`, `metadata`). Adding `#[non_exhaustive]` now would prevent downstream struct literal construction from breaking when fields are added. However, this also makes the type harder to construct in tests, so it is a trade-off.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10

The code demonstrates strong Rust idioms throughout:
- Proper use of `Result` types with `?` propagation everywhere; no `unwrap()` in production code.
- `thiserror` + `miette` for library error types — textbook approach.
- `serde::Serialize` derives are correct and complete for the serialization use case.
- The `serialize()` method on `MdsError` uses exhaustive match arms (made explicit in a follow-up commit), preventing silent omission when new variants are added to the `#[non_exhaustive]` enum.
- The `FileSystem::canonicalize()` trait method has a sensible default (identity) for virtual FS, with `NativeFs` override.
- `IndexMap` swap from `HashMap` for `modules` is well-motivated (preserves insertion order for dependency tracking) and the API is used correctly.
- No `unsafe`, no `panic!`, no `expect()` in production code. `expect()` usage is correctly confined to test assertions.
- Ownership patterns are clean: borrows where possible (`&str` parameters), `Arc` for shared module data, no unnecessary cloning.
- `#[must_use]` on all three new `compile_*_with_deps` functions.
- Comprehensive test coverage for every `MdsError` variant serialization, edge cases in `compute_line_column`, and dependency graph scenarios (chain, diamond, no-import).

The single MEDIUM finding (direct `Path::canonicalize` in `compile_with_deps`) is a consistency gap rather than a correctness bug, since that function always uses `NativeFs`.

**Recommendation**: APPROVED_WITH_CONDITIONS

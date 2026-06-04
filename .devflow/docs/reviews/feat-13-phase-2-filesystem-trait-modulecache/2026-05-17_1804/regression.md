# Regression Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`resolve_source` double-canonicalization** - `crates/mds-core/src/resolver.rs:234,241` + `crates/mds-core/src/fs.rs:269-271` (Confidence: 65%) -- `resolve_source` canonicalizes `base_dir` at line 234, then passes the canonical string to `NativeFs::set_root` which canonicalizes it again (fs.rs:270). This is idempotent and not a bug, but the redundant syscall could be avoided by having `set_root` accept an already-canonical path or by adding a `set_root_unchecked` variant. Not a regression -- the old code also canonicalized once -- but the new layering introduces the extra call.

- **`compile_virtual` consumes `modules` HashMap by value** - `crates/mds-core/src/lib.rs:441` (Confidence: 60%) -- The `compile_virtual` function takes `modules: HashMap<String, String>` by value, forcing callers to give up ownership. If callers want to compile multiple entries from the same virtual filesystem, they must clone the map each time. This is a new API surface and not a regression, but taking `&HashMap` or `impl Into<HashMap>` would be more flexible. However, this may be intentional given the `VirtualFs` constructor also takes ownership.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Analysis

This PR introduces a `FileSystem` trait abstraction to decouple the module resolver from the OS filesystem, enabling virtual/in-memory filesystem backends for WASM and testing. The regression analysis found no issues with confidence >= 80%.

### Regression Checklist

- [x] **No exports removed without deprecation** -- The old `pub fn resolve(path: &Path, ...)` was renamed to `pub fn resolve_path(path: &Path, ...)`, but `ModuleCache` was previously `pub(crate)` (imported via `use resolver::ModuleCache;`, not `pub use`). Since the type itself was not part of the public API, the method rename is not a breaking change for external consumers. The new PR intentionally promotes `ModuleCache` to the public API with `pub use resolver::ModuleCache;` alongside the new `resolve_path` and `resolve_key` methods.
- [x] **Return types backward compatible** -- All existing public functions (`compile`, `check`, `compile_str`, etc.) retain their exact signatures and return types. The internal method changes (`resolve` -> `resolve_by_key`, `PathBuf` keys -> `String` keys) are encapsulated within the crate.
- [x] **Default values unchanged** -- `ModuleCache::new()` and `ModuleCache::default()` both produce a native-filesystem-backed cache, matching the behavior of the old `#[derive(Default)]`.
- [x] **Side effects preserved** -- Warning emission, stderr output, and error reporting behavior are all preserved in the public API functions.
- [x] **All consumers of changed code updated** -- All three internal callers in `lib.rs` (`check`, `compile_collecting_warnings`, `check_collecting_warnings`) correctly updated from `cache.resolve(path, ...)` to `cache.resolve_path(path, ...)`. The CLI crate does not use `ModuleCache` directly.
- [x] **Migration complete across codebase** -- No remaining references to the old `resolve()`, `check_symlink()`, `canonicalize_and_check()`, `read_validated_file()`, or standalone `resolve_path()` function. All functionality correctly moved to `FileSystem` trait implementations.
- [x] **Commit message matches implementation** -- PR describes introducing FileSystem trait, NativeFs, VirtualFs, and ModuleCache refactoring. Code delivers exactly this.
- [x] **Security properties preserved** -- Symlink rejection, path traversal prevention, file size limits, and UTF-8 validation are all preserved in `NativeFs`. The `VirtualFs` relies on its closed key-space for security (no OS access). `validate_import_path` is still called for all import resolution via `resolve_import_from`.
- [x] **Test coverage maintained** -- All 407 existing tests pass. 32 new tests added (fs.rs unit tests + virtual_fs.rs integration tests + api_surface.rs surface tests). No tests removed.
- [x] **Behavioral equivalence of `validate_file_type`** -- The refactored extension extraction (`rsplit('.').next().filter(|e| *e != filename)`) is functionally equivalent to the old `Path::extension()` approach for all practical key formats (both extract the last extension component).

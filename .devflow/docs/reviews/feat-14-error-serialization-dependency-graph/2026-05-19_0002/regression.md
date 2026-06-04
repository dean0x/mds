# Regression Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19

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

- **NativeFs::canonicalize rejects symlinked base_dir where std::fs::canonicalize resolved them** - `crates/mds-core/src/fs.rs:343` (Confidence: 65%) -- On main, `resolve_source` called `base_dir.canonicalize()` (std::path::Path), which silently resolved symlinks. The new code routes through `NativeFs::canonicalize()` which calls `check_symlink()`, rejecting symlinked directories with an ImportError. This is an intentional security hardening (issue #21), but any user whose `base_dir` was a symlink will now get a different error. The PR description documents this as a fix, so it appears intentional.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Analysis Details

### Regression Checklist

- [x] **No exports removed** -- All existing public exports preserved. Three new exports added: `SerializedError`, `SerializedSpan`, `CompileOutput`.
- [x] **Return types backward compatible** -- All 14 existing public functions (`compile`, `compile_str`, `compile_str_with`, `check`, `check_str`, `check_str_with`, `compile_collecting_warnings`, `compile_str_collecting_warnings`, `check_collecting_warnings`, `check_str_collecting_warnings`, `compile_virtual`, `compile_virtual_collecting_warnings`, `check_virtual`, `check_virtual_collecting_warnings`, `compile_file`, `load_vars_file`) retain their original signatures and return types. The three new `*_with_deps` functions are purely additive.
- [x] **Default values unchanged** -- No default value changes detected.
- [x] **Side effects preserved** -- `emit_warnings` behavior unchanged; existing compile functions still print warnings to stderr. New `*_with_deps` functions collect warnings in `CompileOutput::warnings` without stderr output, as documented.
- [x] **All consumers updated** -- CLI (`mds-cli`) does not use any changed internal APIs; it calls `compile_collecting_warnings` and `compile_str_collecting_warnings` whose signatures are unchanged.
- [x] **FileSystem trait backward compatible** -- New `canonicalize()` method has a default implementation (identity function), so existing custom `FileSystem` implementations will not break.
- [x] **Internal HashMap to IndexMap migration safe** -- `ModuleCache.modules` changed from `HashMap` to `IndexMap`. The field is private (not pub), so no external consumer is affected. The `IndexMap` API is a superset of `HashMap` for the operations used (`.get()`, `.insert()`, `.len()`, `.keys()`). The custom `Debug` impl only prints `modules_count`, not the map contents, so debug output is unchanged.
- [x] **`build_output` refactoring is behavior-preserving** -- The inline body-cleaning logic in `compile_collecting_warnings`, `compile_str_collecting_warnings`, and `compile_virtual_collecting_warnings` was extracted into a shared `build_output()` helper. The logic is identical: `clean_output` + `prepend_frontmatter`. Verified by the test `compile_with_deps_output_matches_compile` which asserts identical output between old and new paths.
- [x] **Commit messages match implementation** -- Each commit accurately describes its changes.
- [x] **Test suite passes** -- All 123 unit tests + 22 doc-tests pass.
- [x] **Existing regression tests added** -- `compile_virtual_unchanged` and `compile_str_unchanged` explicitly verify that old API return types have not changed.

### Key Changes Reviewed for Regression Risk

1. **`resolve_source` canonicalization routing** (resolver.rs:245-248): Changed from `base_dir.canonicalize()` (std library) to `self.fs.canonicalize(&base_dir_str)`. For NativeFs this calls `check_symlink()` which rejects symlinks -- an intentional security hardening per issue #21. For VirtualFs/custom FS, the default implementation is identity, preserving previous behavior for non-native backends.

2. **HashMap to IndexMap** (resolver.rs:51): Internal `modules` field changed to preserve insertion order for dependency tracking. No public API surface change. All operations used (`get`, `insert`, `keys`, `len`) have identical semantics in IndexMap.

3. **New public types** (`SerializedError`, `SerializedSpan`, `CompileOutput`): Purely additive. No existing types modified.

4. **New public functions** (`compile_with_deps`, `compile_str_with_deps`, `compile_virtual_with_deps`, `ModuleCache::dependencies`): Purely additive. No existing functions modified or removed.

5. **`MdsError::serialize()` method**: New method on existing type. No existing methods changed.

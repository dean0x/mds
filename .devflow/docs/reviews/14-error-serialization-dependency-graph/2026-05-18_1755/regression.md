# Regression Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18
**PR**: #22

## Issues in Your Changes (BLOCKING)

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`compile_with_deps` uses `std::fs::canonicalize` directly instead of `FileSystem::canonicalize`** - `crates/mds-core/src/lib.rs:521-523` (Confidence: 65%) -- The PR adds `FileSystem::canonicalize()` to abstract path resolution (fixing #21), and `resolve_source` was updated to use `self.fs.canonicalize()`. However, `compile_with_deps` still calls `path.canonicalize()` (std) to compute the entry key for filtering. This is not a bug today because `compile_with_deps` hardcodes `ModuleCache::new()` (NativeFs), so the std canonicalize matches NativeFs behavior. But it creates a latent inconsistency if a future `compile_with_deps_custom_fs` variant were added. Acceptable as-is given the current API surface.

- **`MdsError::serialize()` match lists all variants without wildcard** - `crates/mds-core/src/error.rs:537-567` (Confidence: 60%) -- The match in `serialize()` enumerates all `MdsError` variants. Because `MdsError` is `#[non_exhaustive]`, adding a new variant with a span would require updating this match. However, since `serialize()` lives in the same crate, the compiler enforces exhaustiveness at compile time, so this is a maintenance concern, not a correctness bug.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### 1. HashMap to IndexMap Migration (resolver.rs)

The change from `HashMap<String, Arc<ResolvedModule>>` to `IndexMap<String, Arc<ResolvedModule>>` is clean and regression-free:

- **API compatibility**: All operations used on `self.modules` (`.get()`, `.insert()`, `.len()`, `.keys()`) have identical signatures and semantics between HashMap and IndexMap, except that IndexMap preserves insertion order on `.keys()` -- which is the intentional new behavior for dependency tracking.
- **Performance**: IndexMap provides O(1) amortized get/insert/contains_key, same as HashMap. The constant factors are slightly larger but negligible for a module cache that typically holds tens of entries.
- **No existing code depends on iteration order**: Before this change, no code iterated over `self.modules.keys()`. The new `dependencies()` method is the only consumer of ordered iteration, and it was added in this PR.
- **indexmap was already a workspace dependency**: Used for `IndexSet` in the resolving stack since the initial implementation.

### 2. resolve_source Behavior Change (canonicalize via FileSystem trait)

The change from `base_dir.canonicalize()` (std) to `self.fs.canonicalize(&base_dir_str)` is backward-compatible:

- **NativeFs**: The `canonicalize()` override calls `Path::new(path).canonicalize()` -- identical behavior to the old `base_dir.canonicalize()`.
- **VirtualFs**: The default trait implementation returns the path unchanged (identity), which is correct for in-memory filesystems.
- **Error behavior preserved**: Both the old and new code paths produce `MdsError::Io` on failure. The error message format changed slightly (old: `"cannot resolve base directory {}: {e}"`, new: `"cannot resolve path {}: {e}"` from NativeFs::canonicalize), but this is not a breaking change since error messages are not part of the stable API.
- **Custom FileSystem implementations**: Adding `canonicalize()` with a default implementation preserves backward compatibility for existing implementors.

### 3. Existing compile/compile_str/compile_virtual Return Types

All existing public functions are unchanged:

- `compile()` -> `Result<String, MdsError>` (unchanged)
- `compile_str()` -> `Result<String, MdsError>` (unchanged)
- `compile_virtual()` -> `Result<String, MdsError>` (unchanged)
- `compile_collecting_warnings()` -> `Result<(String, Vec<String>), MdsError>` (unchanged)
- `compile_str_collecting_warnings()` -> `Result<(String, Vec<String>), MdsError>` (unchanged)
- `compile_virtual_collecting_warnings()` -> `Result<(String, Vec<String>), MdsError>` (unchanged)

The PR includes explicit regression tests (`compile_virtual_unchanged`, `compile_str_unchanged`) that assert the existing return types are preserved.

### 4. Re-exports Preserve Existing Imports

The change from `pub use error::MdsError` to `pub use error::{MdsError, SerializedError, SerializedSpan}` is purely additive. Existing `use mds::MdsError` imports continue to work. The new types are additive exports that do not shadow or conflict with existing names.

### 5. build_output Helper Refactor

The new `build_output()` helper is used by both existing functions (`compile_collecting_warnings`, `compile_str_collecting_warnings`, `compile_virtual_collecting_warnings`) and new `_with_deps` functions. The helper simply extracts the existing inline code:

```rust
let body = resolved.prompt_body.as_deref().map(clean_output).unwrap_or_default();
prepend_frontmatter(resolved.raw_frontmatter.as_deref(), body)
```

The `compile_with_deps_output_matches_compile` test explicitly verifies that the same input produces identical output from both `compile_virtual` and `compile_virtual_with_deps`.

### 6. New CompileOutput Struct

`CompileOutput` is purely additive. It has `pub` fields (`output`, `warnings`, `dependencies`), derives `Debug, Clone, PartialEq, serde::Serialize`, and is tested for JSON serialization. No existing type was modified.

### 7. Test Coverage

The PR adds comprehensive regression tests:
- 253 tests pass (zero failures)
- Dependency tracking tests cover: single file, two-file import, three-file chain, diamond dependency (deduplication), error propagation
- API surface tests verify existing function signatures are preserved
- Error serialization tests cover all MdsError variants

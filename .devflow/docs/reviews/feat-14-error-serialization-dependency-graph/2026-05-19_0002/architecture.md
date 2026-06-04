# Architecture Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19
**PR**: #22

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent entry-key exclusion strategy across compile_*_with_deps functions** - `lib.rs:530-610`
**Confidence**: 85%
- Problem: The three `compile_*_with_deps` functions use three different strategies to exclude the entry module from dependencies:
  1. `compile_with_deps` (line 531): Uses `split_last()` on the ordered Vec, relying on the invariant that post-order DFS always inserts the entry module last.
  2. `compile_str_with_deps` (line 573): Skips filtering entirely because `resolve_source` does not insert the inline source into the cache.
  3. `compile_virtual_with_deps` (line 610): Uses `.filter(|k| k != entry)` to remove the entry by key name.

  The `split_last()` approach in `compile_with_deps` is correct but couples the public API to an internal ordering invariant of `ModuleCache`. If `IndexMap` insertion order ever changes (e.g., due to a caching optimization or refactor in `resolve_by_key`), the last element may no longer be the entry module, and `split_last()` would silently remove the wrong dependency. The `filter` approach in `compile_virtual_with_deps` is more robust because it is key-based, not position-based.

- Fix: Unify on a single, explicit entry-key-based exclusion strategy. Either:
  (a) Have `ModuleCache::dependencies()` accept an optional `exclude_key: Option<&str>` parameter, or
  (b) Use the key-based filter consistently in all three functions. For `compile_with_deps`, the entry key is the first normalized key produced by `resolve_path` -- retrieve it from the cache (it is the last key in the IndexMap) and filter by name rather than position:
  ```rust
  let deps = cache.dependencies();
  let entry_key = deps.last().cloned();
  let dependencies = deps.into_iter()
      .filter(|k| entry_key.as_ref() != Some(k))
      .collect();
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**CompileOutput defined in lib.rs rather than its own module** - `lib.rs:61-76`
**Confidence**: 80%
- Problem: `CompileOutput` is a public API struct with `serde::Serialize` defined inline in `lib.rs` alongside 20+ function definitions. The codebase otherwise follows a clean module separation pattern: errors in `error.rs`, filesystem in `fs.rs`, resolution in `resolver.rs`, etc. Placing a new data type in `lib.rs` breaks this pattern. As more output-related types are added (e.g., `CheckOutput`, `LintOutput`), `lib.rs` will accumulate structural definitions that belong in a dedicated module.
- Fix: This is a minor structural concern. Consider extracting `CompileOutput` to a dedicated module (e.g., `output.rs`) or co-locating it with `resolver.rs` where the dependency data originates. Not blocking given the type is small (3 fields), but worth addressing if the output surface grows.

## Pre-existing Issues (Not Blocking)

No pre-existing architectural issues identified in the reviewed files.

## Suggestions (Lower Confidence)

- **Code duplication across compile_* function families** - `lib.rs:268-612` (Confidence: 70%) -- The `compile`, `compile_collecting_warnings`, and `compile_with_deps` families follow the same resolve-build_output pattern with minor variations. A higher-order helper or builder pattern could reduce the 6+ near-identical functions, but the current approach is explicit and each function is short.

- **dependencies() returns owned Vec on every call** - `resolver.rs:112-114` (Confidence: 65%) -- `ModuleCache::dependencies()` clones all keys into a new `Vec<String>` each time it is called. For large dependency graphs, returning an iterator or a borrowed slice-like view would be more efficient. Current usage only calls it once per compile, so this is not a practical concern yet.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The architecture of this PR is well-structured overall. The three main additions -- `SerializedError`/`SerializedSpan`, `CompileOutput` with dependency tracking, and `FileSystem::canonicalize()` -- each follow clean separation of concerns and respect the existing layering. Key strengths:

- **Drift-proof serialization**: Using `miette::Diagnostic` trait methods for code/help extraction rather than hardcoding strings prevents serialization from drifting out of sync with the error definitions. This is a strong design choice.
- **Trait-based canonicalization**: Routing `canonicalize()` through the `FileSystem` trait (with identity default) correctly extends the abstraction boundary and fixes the `std::fs::canonicalize()` bypass.
- **IndexMap for dependency ordering**: Replacing `HashMap` with `IndexMap` for the module cache is the right data structure choice -- it preserves DFS insertion order at no API cost while maintaining O(1) lookup.
- **build_output() extraction**: Factoring the duplicated output-building logic into a shared helper is a clean refactor.

The single blocking condition is the inconsistent entry-key exclusion approach across the three `_with_deps` functions. Unifying on a key-based strategy would make the code more resilient to future internal changes in cache ordering.

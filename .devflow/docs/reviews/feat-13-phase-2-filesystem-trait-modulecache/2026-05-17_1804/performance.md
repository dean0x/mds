# Performance Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**NativeFs::read reads entire file before checking size** - `crates/mds-core/src/fs.rs:248-256`
**Confidence**: 82%
- Problem: `NativeFs::read` reads the entire file into memory with `std::fs::read(path)` before checking `bytes.len() as u64 > MAX_FILE_SIZE`. For a maliciously large file (e.g. multi-GB), this allocates that entire buffer before rejecting it. MAX_FILE_SIZE is 10 MB, but nothing prevents a 4 GB file from being fully read into memory first.
- Fix: This is a pre-existing pattern (the old `read_validated_file` had the identical read-then-check approach), so the issue is inherited, not introduced. However, since this code was moved and restructured as part of this PR, it is worth noting. A metadata check before read would add a TOCTOU window; the current approach is acceptable for a template compiler where inputs are developer-controlled files. No action required for this PR.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**NativeFs::normalize always runs two canonicalize syscalls even for cache hits** - `crates/mds-core/src/fs.rs:233` / `crates/mds-core/src/resolver.rs:114,205`
**Confidence**: 85%
- Problem: Both `resolve_path` and `resolve_import_from` call `self.fs.normalize()` before checking the cache in `resolve_by_key`. For `NativeFs`, `normalize` calls `check_symlink` which performs two `canonicalize()` syscalls. This means every cache-hit resolution still pays two filesystem syscalls. In projects with many import statements referencing the same module, this redundant I/O adds up.
- Impact: For a project with N import statements across all modules, this is 2N syscalls even when most are cache hits. The old code had the same pattern (`canonicalize_and_check` ran before the cache check), so this is not a regression. However, the abstraction boundary now makes it harder to optimize: the `FileSystem` trait returns a `String` key, so the cache lookup necessarily requires normalization first.
- Fix: This is an inherent trade-off of the trait abstraction and is acceptable for a template compiler. A future optimization could add a `raw_key` method to skip canonicalization for cache lookup, but that adds complexity without evidence of a real bottleneck. No change needed now.

**VirtualFs::read clones the full file content string** - `crates/mds-core/src/fs.rs:114-119`
**Confidence**: 83%
- Problem: `VirtualFs::read` uses `.cloned()` on the HashMap value, which copies the entire source content string for every call. For cached modules this is fine (the cache check in `resolve_by_key` returns before `read` is called), but for first-time resolution of large virtual files, this is an unavoidable full copy.
- Impact: Minimal in practice. The `FileSystem::read` trait returns `String` (owned), so a clone is necessary. The alternative would be returning `Cow<str>` or `Arc<String>`, but that adds API complexity for marginal gain in a testing/WASM context where files are small templates.
- Fix: No change needed. The current design is appropriate for the use case.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Dynamic dispatch overhead from Box<dyn FileSystem>** - `crates/mds-core/src/resolver.rs:47,114,143,146,205,241` (Confidence: 65%) -- The `ModuleCache` uses `Box<dyn FileSystem>` which adds vtable indirection for every `normalize`, `read`, and `is_markdown` call. For a template compiler, this cost is negligible compared to I/O and parsing. A generic `ModuleCache<F: FileSystem>` would eliminate this, but at the cost of monomorphization bloat and less ergonomic public API. Current design is the right trade-off.

- **String key allocation on every resolve_by_key call** - `crates/mds-core/src/resolver.rs:154,186` (Confidence: 70%) -- `resolve_by_key` calls `key.to_string()` twice on the hot path (once for `resolving.insert`, once for `modules.insert`). The old code cloned `PathBuf` which has similar allocation cost. The second `to_string()` at line 186 could reuse the string from line 154, but the resolving set owns that string until pop, so reuse requires restructuring. Marginal improvement.

- **NativeFs::normalize converts PathBuf to String via display()** - `crates/mds-core/src/fs.rs:243` (Confidence: 62%) -- `canonical.display().to_string()` goes through the Display trait, which for non-UTF-8 paths uses replacement characters. `canonical.to_str().unwrap_or(...).to_string()` or `canonical.into_os_string().into_string()` would be more direct, though the difference is negligible for well-formed paths.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | - | 1 | 2 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED

The PR introduces a well-designed FileSystem trait abstraction that cleanly separates filesystem concerns from module resolution logic. The performance characteristics are preserved from the prior implementation -- the `PathBuf` to `String` key migration, the `Box<dyn FileSystem>` dynamic dispatch, and the canonicalize-before-cache pattern all carry negligible overhead for a template compiler workload. The caching strategy (Arc-wrapped modules, HashMap keyed by normalized path) remains efficient. No performance regressions are introduced. The "Should Fix" items are architectural observations about inherited patterns, not actionable blockers.

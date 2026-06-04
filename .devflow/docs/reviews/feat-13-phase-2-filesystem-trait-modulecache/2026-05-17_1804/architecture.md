# Architecture Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17
**PR**: #20

## Issues in Your Changes (BLOCKING)

### HIGH

**`set_root` breaks trait cohesion -- NativeFs-specific concern leaked into FileSystem trait** - `crates/mds-core/src/fs.rs:37`
**Confidence**: 85%
- Problem: The `set_root` method on the `FileSystem` trait exists solely because `NativeFs` needs to establish a project root for path-traversal prevention, and `resolve_source` in the resolver calls it from outside the normal `normalize` flow. `VirtualFs` ignores it entirely. This is an Interface Segregation Principle (ISP) concern: the trait forces all implementations to consider a method that only one implementation uses. It also makes the trait API less intuitive for custom implementations -- a user implementing `FileSystem` has to understand the NativeFs-specific root initialization protocol.
- Fix: Consider moving root initialization into `normalize` for all cases (NativeFs already does this on first entry-point call). For `resolve_source`, the resolver could call `normalize("", &synthetic_entry)` to trigger root detection instead of an explicit `set_root`. If that is not feasible for v0.1, the current default no-op is an acceptable compromise -- but document this as a known design debt for future trait refinement.

**`process_module` uses `key` for both `file_str` (display) and `base_key` (resolution)** - `crates/mds-core/src/resolver.rs:156`
**Confidence**: 82%
- Problem: In `resolve_by_key`, line 156 calls `self.process_module(&source, key, key, ...)` passing `key` as both the display name (`file_str`) and the resolution base (`base_key`). For `NativeFs`, the key is a canonical absolute path like `/Users/foo/project/main.mds`, which is fine for both roles. For `VirtualFs`, the key is a short virtual name like `main.mds`, which is also fine. However, a custom `FileSystem` implementation could produce normalized keys that are unsuitable as user-facing display names (e.g., content hashes, database IDs, or encoded URIs). The trait documentation does not establish a contract about whether normalized keys are human-readable.
- Fix: Either (a) document in the `FileSystem` trait that normalized keys must be human-readable path-like strings suitable for error messages, or (b) add an optional `display_name(&self, key: &str) -> String` method to the trait (with a default returning `key.to_string()`) so custom implementations can provide user-friendly names.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`resolve_source` assumes NativeFs semantics on a polymorphic `Box<dyn FileSystem>`** - `crates/mds-core/src/resolver.rs:225-250`
**Confidence**: 85%
- Problem: `resolve_source` calls `base_dir.canonicalize()` (an OS filesystem operation) and then `self.fs.set_root(...)`. This method only works correctly when the underlying `FileSystem` is `NativeFs`. If someone constructs a `ModuleCache::virtual_fs(...)` or `ModuleCache::with_fs(custom)` and then calls `resolve_source`, the `canonicalize()` call will succeed or fail based on whether the OS path exists, unrelated to the virtual filesystem's contents. The method signature accepts `&Path` which is an OS-path concept, creating a semantic mismatch with `VirtualFs`.
- Fix: Either (a) make `resolve_source` only available on `ModuleCache` when backed by `NativeFs` (via a separate impl block or a builder pattern), or (b) document that `resolve_source` is only valid for OS-backed filesystems and will produce undefined behavior with `VirtualFs`. Since the public API already has `compile_virtual` for the virtual path, option (b) with clear documentation may suffice for v0.1.

**`NativeFs::read` checks file size after reading entire file into memory** - `crates/mds-core/src/fs.rs:248-256`
**Confidence**: 80%
- Problem: The file is fully read into a `Vec<u8>` via `std::fs::read(path)` before checking `bytes.len() > MAX_FILE_SIZE`. A 10 GB file would be fully allocated in memory before being rejected. The comment in the old code mentioned this was to avoid a TOCTOU race between `metadata()` and `read()`, which is valid, but the trade-off is unbounded allocation on malicious input.
- Fix: Use `std::fs::metadata` first as a cheap pre-check (understanding TOCTOU), then read with a bounded buffer. Alternatively, use `File::open` + `take(MAX_FILE_SIZE + 1)` to read at most `MAX_FILE_SIZE + 1` bytes, then reject if the full buffer was filled. This bounds memory regardless of TOCTOU:
  ```rust
  use std::io::Read;
  let mut file = std::fs::File::open(path).map_err(...)?;
  let mut bytes = Vec::new();
  file.take(MAX_FILE_SIZE + 1).read_to_end(&mut bytes).map_err(...)?;
  if bytes.len() as u64 > MAX_FILE_SIZE {
      return Err(MdsError::resource_limit(...));
  }
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`resolver` module is `pub(crate)` but key types are re-exported** - `crates/mds-core/src/lib.rs:47,53`
**Confidence**: 80%
- Problem: The `resolver` module is declared `pub(crate)`, but `ModuleCache` is re-exported via `pub use resolver::ModuleCache`. Meanwhile `fs` is declared `pub mod fs` (fully public), exposing its internal module structure. The mixed visibility strategy is inconsistent -- both modules are part of the public API surface, but one is hidden and re-exported while the other is fully public. This matters because `pub mod fs` means external crates can write `mds::fs::VirtualFs` in addition to `mds::VirtualFs`, creating two paths to the same type.
- Fix: Either make `fs` also `pub(crate)` with re-exports (matching the `resolver` pattern), or document that `mds::fs::*` is the canonical path and the re-exports are convenience aliases.

## Suggestions (Lower Confidence)

- **`NativeFs` interior mutability via `OnceLock` is unconventional for trait objects** - `crates/mds-core/src/fs.rs:133` (Confidence: 70%) -- `NativeFs` uses `OnceLock<PathBuf>` for its `root_dir` field, requiring `&self` methods to perform interior mutation. While `OnceLock` is safe, the `FileSystem` trait takes `&self` for all methods including `set_root`, which means the trait signature implies immutability but `NativeFs` mutates. This is a design tension rather than a bug, but future implementations might be surprised by the interior mutability requirement.

- **`VirtualFs` does not validate null bytes or empty keys in `read`** - `crates/mds-core/src/fs.rs:114-119` (Confidence: 65%) -- The `normalize` method validates against null bytes and empty paths, but `read` does a bare `HashMap::get`. If `read` is ever called with a key that bypassed `normalize`, the validation gap could surface. Since the resolver always normalizes first, this is defense-in-depth rather than a live issue.

- **Dependency direction: `fs.rs` imports from `resolver.rs` constants** - `crates/mds-core/src/fs.rs:12` (Confidence: 65%) -- `fs.rs` imports `MAX_FILE_SIZE` and `MAX_TRAVERSAL_DEPTH` from `resolver.rs`. Architecturally, the filesystem abstraction (`fs`) is lower-level infrastructure that `resolver` depends on. Having `fs` import from `resolver` creates a minor dependency inversion. These constants could live in a shared `limits.rs` module (which already exists as `crate::limits`) to keep the dependency direction clean.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | - |
| Should Fix | - | 0 | 2 | - |
| Pre-existing | - | - | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The FileSystem trait abstraction is well-designed and follows DIP (Dependency Inversion Principle) correctly. The separation of `VirtualFs` and `NativeFs` is clean, the trait surface is minimal (4 methods), and the refactoring of `ModuleCache` to use `Box<dyn FileSystem>` is a textbook application of constructor injection. The `resolve_import_from` helper centralizes import resolution nicely, eliminating the duplicated `validate_import_path` + `resolve_path` pattern across multiple methods.

Conditions for merge:
1. **Document `set_root` contract** -- Either refactor it out of the trait or clearly document that custom `FileSystem` implementations may need to handle `set_root` for `resolve_source` compatibility.
2. **Document `resolve_source` OS dependency** -- Add a doc comment noting this method assumes an OS-backed filesystem.

The file-size-after-read issue (Should Fix) is a pre-existing design choice that was intentionally carried forward, so it should not block this PR but should be addressed in a follow-up.

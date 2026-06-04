# Rust Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**NativeFs::read performs file size check after full file read into memory** - `crates/mds-core/src/fs.rs:248-256`
**Confidence**: 90%
- Problem: `NativeFs::read` calls `std::fs::read(path)` which reads the entire file into a `Vec<u8>` before checking `bytes.len() as u64 > MAX_FILE_SIZE`. For a 10 MB limit this is acceptable, but the intent of a file size limit is to prevent excessive memory allocation. A malicious or accidental multi-GB file would be fully allocated in memory before the check fires and returns an error. The previous code had the same pattern (moved from `read_validated_file`), but since this is a new public-facing method in a newly extracted module, it is the right time to address it.
- Fix: Check file metadata size before reading, then read and confirm actual size matches (defense in depth against TOCTOU). The metadata pre-check avoids the allocation for obviously-too-large files:
```rust
fn read(&self, normalized: &str) -> Result<String, MdsError> {
    let path = Path::new(normalized);
    let meta = std::fs::metadata(path)
        .map_err(|e| MdsError::io(format!("cannot read {normalized}: {e}")))?;
    if meta.len() > MAX_FILE_SIZE {
        return Err(MdsError::resource_limit(format!(
            "file too large ({} bytes, max {} bytes): {normalized}",
            meta.len(), MAX_FILE_SIZE,
        )));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| MdsError::io(format!("cannot read {normalized}: {e}")))?;
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(MdsError::resource_limit(format!(
            "file too large ({} bytes, max {} bytes): {normalized}",
            bytes.len(), MAX_FILE_SIZE,
        )));
    }
    String::from_utf8(bytes)
        .map_err(|e| MdsError::io(format!("invalid UTF-8 in {normalized}: {e}")))
}
```

### MEDIUM

**`#[must_use]` attribute on `compile_virtual` return type is a string annotation, not compiler-enforced** - `crates/mds-core/src/lib.rs:439`
**Confidence**: 82%
- Problem: The function has `#[must_use = "the compiled Markdown output should be used"]` but `compile_virtual` returns `Result<String, MdsError>`, and `Result` already has `#[must_use]` in std. The attribute is redundant since `Result` itself triggers the warning. This is not wrong, but it is inconsistent with the crate pattern -- the existing `compile_str`, `check`, etc. all have this same annotation, so this is actually consistent. However, the trait methods `FileSystem::normalize`, `FileSystem::read`, and `FileSystem::set_root` all return `Result` without `#[must_use]`, which is the expected Rust idiom since `Result` is inherently `must_use`.
- This is a stylistic observation and consistent with existing patterns. No action needed.

**`VirtualFs::is_markdown` uses `ends_with(".md")` which matches `foo.someprefix.md` but also matches keys like `readme.md.bak` -- wait, no, `.md` is a suffix check** - Actually upon re-examination, `ends_with(".md")` correctly matches files ending in `.md`. However, it would also match a key like `file.cmd`. Wait -- no, `.md` requires the dot. This is actually fine. Withdrawn.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`NativeFs::set_root` double-canonicalizes: caller already canonicalizes in `resolve_source`** - `crates/mds-core/src/resolver.rs:234-241` and `crates/mds-core/src/fs.rs:268-274`
**Confidence**: 85%
- Problem: `resolve_source` canonicalizes `base_dir` (line 234), converts it to a string (`canonical_str`), then calls `self.fs.set_root(&canonical_str)`. `NativeFs::set_root` then calls `Path::new(base).canonicalize()` again on the already-canonical path. This is an unnecessary syscall since the path is already canonical. While harmless (canonicalizing an already-canonical path is idempotent), it creates a subtle contract confusion: the method name `set_root` suggests it just sets a value, but it internally re-canonicalizes.
- Fix: Either (a) document that `set_root` expects a non-canonical path and let it handle canonicalization exclusively, or (b) remove the canonicalize call from `set_root` since the caller is responsible. Option (a) is cleaner -- move the canonicalization entirely into `set_root` and pass the raw `base_dir.display().to_string()` from `resolve_source`:
```rust
// In resolve_source, simplify to:
let base_str = base_dir.display().to_string();
self.fs.set_root(&base_str)?;
let base_key = format!("{}/{}", 
    Path::new(&base_str).canonicalize()
        .map_err(|e| MdsError::io(format!("cannot resolve base directory {}: {e}", base_dir.display())))?
        .display(),
    "<source>");
```
Or simpler: just remove `.canonicalize()` from `NativeFs::set_root` and rename the parameter to clarify it expects a canonical path.

**`process_module` passes `key` as both `file_str` and `base_key` for normal resolution** - `crates/mds-core/src/resolver.rs:156`
**Confidence**: 80%
- Problem: In `resolve_by_key`, the call is `self.process_module(&source, key, key, ...)` -- the same `key` serves as both the display path for error messages (`file_str`) and the resolution base (`base_key`). For `NativeFs`, `key` is an absolute canonical path like `/Users/foo/project/main.mds`, which makes error messages verbose with full absolute paths. Previously, the code had separate `file_str` (display path) and `base_dir` (resolution directory), giving the ability to show cleaner paths in errors. This regression makes error messages less user-friendly for native filesystem usage.
- Fix: Consider extracting a shorter display name (relative to project root or just the filename) for `file_str` while keeping the full canonical path as `base_key`. This could be done in `resolve_by_key`:
```rust
let display_name = key_display_name(key); // Already exists as a helper
self.process_module(&source, display_name, key, is_md, runtime_vars, warnings)
```
However, note that `key_display_name` returns only the filename, which loses directory context for error messages when multiple files share names. A better approach might be to compute a relative path from the project root.

## Pre-existing Issues (Not Blocking)

No critical pre-existing issues found.

## Suggestions (Lower Confidence)

- **Consider accepting `&str` instead of `HashMap<String, String>` ownership in `VirtualFs::new`** - `crates/mds-core/src/fs.rs:54` (Confidence: 65%) -- The constructor takes ownership of the HashMap. For testing scenarios where the same module set is reused, accepting `impl Into<HashMap<String, String>>` or providing a `from_iter` constructor could reduce cloning. However, since VirtualFs is primarily for single-use test/WASM scenarios, current ownership semantics are reasonable.

- **`validate_file_type` extension extraction could mis-parse dotfiles** - `crates/mds-core/src/resolver.rs:704-705` (Confidence: 70%) -- The expression `filename.rsplit('.').next().filter(|e| *e != filename)` handles the no-extension case by checking `*e != filename`. However, for a key like `.mds` (a dotfile with no stem), `filename` would be `.mds`, `rsplit('.').next()` returns `"mds"`, and the filter passes since `"mds" != ".mds"`. This means a dotfile named `.mds` would be treated as having extension `mds` and pass validation, which may or may not be intended. Edge case with low practical impact.

- **Missing `Debug` impl on `NativeFs` and `VirtualFs`** - `crates/mds-core/src/fs.rs:48,132` (Confidence: 72%) -- Neither struct derives `Debug`. `ModuleCache` also lacks `Debug` (it contains `Box<dyn FileSystem>`). Adding `Debug` to the trait bound or deriving it on the structs would improve diagnostics. The `OnceLock<PathBuf>` in `NativeFs` supports `Debug`, and `HashMap<String, String>` in `VirtualFs` does too, so `#[derive(Debug)]` would work on both.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The FileSystem trait abstraction is well-designed with clean separation of concerns. The trait surface is minimal (4 methods, one with a default), the implementations are correct, and the security properties (symlink rejection, traversal prevention, file size limits) are properly preserved through the refactor. The `OnceLock` usage in `NativeFs` for thread-safe root initialization is idiomatic. The `VirtualFs` path normalization with segment-based `..` resolution is correct and well-tested. Error handling uses `thiserror`-derived `MdsError` consistently with `Result` propagation via `?` throughout -- no `.unwrap()` in non-test code. The codebase passes all 407 tests and has zero clippy warnings.

Conditions: Address the HIGH issue (pre-read file size check) before merge. The MEDIUM should-fix items are improvements that could be addressed in this PR or a follow-up.

# Security Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**NativeFs::normalize lacks null-byte validation** - `crates/mds-core/src/fs.rs:222`
**Confidence**: 82%
- Problem: `VirtualFs::normalize` explicitly rejects null bytes in the `relative` parameter (line 68-70), but `NativeFs::normalize` does not. While the OS-level `canonicalize()` call in `check_symlink` will reject null bytes (causing a `FileNotFound` error), this is an implicit defense rather than an explicit one. The error message for a null-byte path would say "file not found" rather than clearly indicating "null byte in import path", which obscures the attack vector and makes debugging harder. Additionally, this inconsistency between the two `FileSystem` implementations means the `FileSystem` trait contract is ambiguous about whether implementors must reject null bytes.
- Fix: Add null-byte rejection at the start of `NativeFs::normalize`, mirroring the `VirtualFs` check:
```rust
fn normalize(&self, base: &str, relative: &str) -> Result<String, MdsError> {
    if relative.contains('\0') {
        return Err(MdsError::import_error("import path contains null byte"));
    }
    let path = if base.is_empty() {
    // ...
```

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### HIGH

**FileSystem trait doc does not specify security obligations for implementors** - `crates/mds-core/src/fs.rs:14-19`
**Confidence**: 85%
- Problem: The `FileSystem` trait is `pub` and `ModuleCache::with_fs` is `pub`, meaning external crate consumers can provide custom `FileSystem` implementations. The trait doc mentions that "Security properties (symlink rejection, traversal prevention) are implementation-specific" but does not document the security obligations that custom implementations SHOULD meet. The resolver relies on `FileSystem::normalize` to prevent path traversal and `FileSystem::read` to enforce size limits. A custom implementation that skips these checks would silently bypass all security controls while appearing to work correctly. This is an insecure-by-default API pattern.
- Fix: Add a `# Safety` or `# Security` doc section to the `FileSystem` trait specifying the minimum security contract:
```rust
/// # Security Contract
///
/// Implementations MUST enforce at minimum:
/// - **Path traversal prevention**: `normalize` must not resolve paths outside
///   the intended project boundary.
/// - **Null-byte rejection**: `normalize` must reject paths containing `\0`.
/// - **File size limits**: `read` should enforce [`MAX_FILE_SIZE`] or a
///   comparable bound to prevent resource exhaustion.
///
/// The built-in [`NativeFs`] and [`VirtualFs`] implementations satisfy these
/// requirements. Custom implementations that skip them bypass all security
/// controls in the module resolver.
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**TOCTOU window between normalize and read in NativeFs** - `crates/mds-core/src/fs.rs:233,248`
**Confidence**: 80%
- Problem: `NativeFs::normalize` canonicalizes the path and checks for symlinks, then later `NativeFs::read` reads from the canonical path. Between these two operations, an attacker could replace the regular file with a symlink, bypassing the symlink check. This is an inherent limitation of the normalize-then-read pattern with filesystem operations. This is pre-existing behavior (the old `canonicalize_and_check` + `read_validated_file` split had the same window) and was not introduced by this PR.
- Fix: This is difficult to fix without an `O_NOFOLLOW` open approach or reading the file within `normalize`. Consider documenting this as a known limitation, or combining normalize+read into a single `open_and_read` method in a future hardening pass.

## Suggestions (Lower Confidence)

- **VirtualFs root entry point accepts unsanitized keys** - `crates/mds-core/src/fs.rs:72-75` (Confidence: 65%) -- When `base.is_empty()`, `VirtualFs::normalize` returns `relative` without any traversal or sanitization checks. While VirtualFs's closed HashMap key-space prevents actual file access, keys like `"../../../etc/passwd"` would be stored in the resolving set and error messages, which could be confusing in diagnostic output.

- **resolve_key bypasses validate_import_path** - `crates/mds-core/src/resolver.rs:213-220` (Confidence: 70%) -- The public `resolve_key` method delegates directly to `resolve_by_key` without calling `validate_import_path`. This is by design (entry-point keys are not relative import paths), but it means callers of `resolve_key` with untrusted input could pass arbitrary key strings. The security boundary here relies entirely on the `FileSystem` implementation's `read` method to reject invalid keys.

- **File read into memory before size check** - `crates/mds-core/src/fs.rs:248-256` (Confidence: 62%) -- `NativeFs::read` reads the entire file into memory via `std::fs::read(path)` before checking the size limit. A malicious file could temporarily allocate up to the OS memory limit before being rejected. The 10MB `MAX_FILE_SIZE` limit keeps this bounded in practice, but a streaming approach would be more defensive. This is pre-existing behavior preserved from the old code.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 1 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Security Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The refactoring preserves all existing security controls (symlink rejection, path traversal prevention, file size limits, import depth limits, cycle detection) and correctly migrates them into the new `FileSystem` trait architecture. The VirtualFs implementation adds appropriate traversal guards for its virtual key-space. The two reported issues are: (1) an asymmetry in null-byte validation between NativeFs and VirtualFs, and (2) missing security documentation on the public `FileSystem` trait for custom implementors. Neither is exploitable in the current codebase (NativeFs gets OS-level null-byte rejection via `canonicalize`, and no external consumers exist yet), but both should be addressed before the public API stabilizes.

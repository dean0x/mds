# Security Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**NativeFs::normalize does not explicitly reject empty relative paths** - `crates/mds-core/src/fs.rs:253`
**Confidence**: 82%
- Problem: The `FileSystem` trait's security contract (line 31) states that `normalize` must reject empty paths, and `VirtualFs` enforces this (line 81-83), but `NativeFs::normalize` lacks an explicit empty-path check. An empty `relative` string would flow through to `check_symlink(Path::new(""))` where `file_name()` returns `None`, producing a `FileNotFound` error. While the empty path IS rejected, the error message is misleading -- it says "file not found" rather than indicating an empty/invalid path. This is a defense-in-depth inconsistency between the two implementations and violates the contract's own documentation.
- Fix: Add an explicit empty-path guard at the top of `NativeFs::normalize`, matching `VirtualFs`:
```rust
fn normalize(&self, base: &str, relative: &str) -> Result<String, MdsError> {
    if relative.is_empty() {
        return Err(MdsError::import_error("import path is empty"));
    }
    if relative.contains('\0') {
        // ... existing null byte check
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Security contract enforceability via with_fs()** - `crates/mds-core/src/fs.rs:23-35` (Confidence: 65%) -- The `FileSystem` trait documents a security contract that custom implementations MUST uphold (traversal prevention, null-byte rejection, size limits, empty-path rejection). Since `with_fs()` is a public API, there is no compile-time or runtime enforcement that custom implementations meet these obligations. This is a documentation-only contract. Consider whether a wrapper/decorator pattern could enforce some invariants (e.g., null-byte checks, size limits) at the `ModuleCache` level, independent of the `FileSystem` implementation. This would provide defense-in-depth for third-party backends.

- **VirtualFs root entry point accepts unsanitized keys** - `crates/mds-core/src/fs.rs:88-91` (Confidence: 62%) -- When `base` is empty, `VirtualFs::normalize` returns `relative` as-is, including any `..` segments. While this is safe because VirtualFs uses a closed HashMap key-space (lookups can only match explicitly inserted keys), it means the normalized key may contain path-traversal segments that are semantically misleading. If a future refactor added filesystem-backed fallback to VirtualFs, these unsanitized keys could become exploitable. The current code is safe but worth noting for maintainability.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

## Security Analysis Notes

The FileSystem trait abstraction is well-designed from a security perspective. Key strengths:

1. **Defense in depth**: NativeFs enforces multiple independent security layers -- null-byte rejection, symlink detection (TOCTOU-safe via parent canonicalization), path traversal prevention against project root, and file size limits (both pre-read metadata check and post-read byte-length check).

2. **OnceLock for root_dir**: Using `OnceLock` for the project root prevents a second entry point from widening the security boundary. Once set, the root cannot be changed, even in concurrent scenarios.

3. **Import path validation**: The resolver's `validate_import_path()` enforces that all import paths must start with `./` or `../`, blocking absolute path injection before it reaches `NativeFs::normalize`. Even if this guard were bypassed, `check_path_traversal` would reject paths outside the project root.

4. **VirtualFs closed key-space**: VirtualFs is inherently safe against path traversal because it can only return content that was explicitly inserted into its HashMap. No filesystem access is possible.

5. **TOCTOU-safe file reads**: NativeFs::read uses a metadata pre-check (optimization) combined with a post-read size check (defense-in-depth), correctly handling the TOCTOU window where a file could grow between the two calls.

6. **Trait security contract documentation**: The `FileSystem` trait documents explicit security obligations for custom implementations. While not enforceable at compile time, this is good practice for a public API.

The single blocking finding (empty-path check inconsistency in NativeFs) is low-impact since the empty path is still rejected, just with a less informative error message. The condition warrants fixing for correctness and consistency with the documented security contract.

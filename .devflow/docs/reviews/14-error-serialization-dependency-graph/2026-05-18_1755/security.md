# Security Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18
**PR**: #22 — SerializedError/SerializedSpan, CompileOutput with dependency tracking, FileSystem::canonicalize()

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**NativeFs::canonicalize() follows symlinks without rejection** - `crates/mds-core/src/fs.rs:343-348`
**Confidence**: 85%
- Problem: The new `NativeFs::canonicalize()` calls `std::fs::canonicalize()` which silently resolves symlinks. This is used in `resolve_source()` (resolver.rs:248) to canonicalize the `base_dir`. If `base_dir` is a symlink pointing outside the project root, the resolved path becomes the symlink target, and `set_root()` then establishes that target as the trusted root. Subsequent imports would then be checked against the symlink-target root, not the original project directory -- potentially allowing access to files outside the intended project boundary.

  In contrast, the existing `normalize()` path uses `check_symlink()` which explicitly detects and rejects symlinks. The new `canonicalize()` method bypasses this protection.

  The practical risk is bounded because `resolve_source()` is only called for `compile_str_with` / `check_str_with` (not for file-based compilation), and the `base_dir` is typically provided by the caller (not from untrusted input). However, if a caller passes a user-controlled `base_dir` containing a symlink, the root jail could be re-anchored to an attacker-chosen directory.

- Fix: Apply symlink detection to `canonicalize()` or document that `base_dir` must not be a symlink:
  ```rust
  fn canonicalize(&self, path: &str) -> Result<String, MdsError> {
      let p = Path::new(path);
      // Reject if path itself is a symlink (mirrors check_symlink logic)
      let meta = std::fs::symlink_metadata(p)
          .map_err(|e| MdsError::io(format!("cannot stat {path}: {e}")))?;
      if meta.file_type().is_symlink() {
          return Err(MdsError::import_error(format!(
              "symlinks are not allowed: {path}"
          )));
      }
      p.canonicalize()
          .map(|p| p.display().to_string())
          .map_err(|e| MdsError::io(format!("cannot resolve path {path}: {e}")))
  }
  ```

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**compile_with_deps uses std::fs::canonicalize directly, bypassing FileSystem trait** - `crates/mds-core/src/lib.rs:521-524`
**Confidence**: 82%
- Problem: `compile_with_deps()` calls `path.canonicalize()` (std library) directly rather than going through `self.fs.canonicalize()`. This is inconsistent with the PR's stated goal of routing all canonicalization through the `FileSystem` trait (issue #21). While this specific call is only used for entry-key comparison (not for path resolution or security checks), it creates a subtle asymmetry: if a custom `FileSystem` implementation overrides `canonicalize()` to produce different results, the entry key filtering in `dependencies` would mismatch the cache keys, causing the entry module to appear in the dependency list.

  More importantly, `path.canonicalize()` follows symlinks without any check, so if the entry file is a symlink, the entry key is the symlink target while the cache may have stored it under a different key (depending on normalize behavior).

- Fix: Since `compile_with_deps` only has access to a `ModuleCache` (not the inner `fs`), consider exposing a method on `ModuleCache` that delegates to `self.fs.canonicalize()`, or use the first key from `cache.dependencies()` as the entry key instead:
  ```rust
  // Alternative: use the first dependency key as the entry key
  let all_deps = cache.dependencies();
  let entry_key = all_deps.first().cloned().unwrap_or_default();
  let dependencies = all_deps.into_iter().filter(|k| k != &entry_key).collect();
  ```

## Pre-existing Issues (Not Blocking)

(none found at CRITICAL level in unchanged code)

## Suggestions (Lower Confidence)

- **Error information leakage in SerializedError** - `crates/mds-core/src/error.rs:25-30` (Confidence: 65%) — The `SerializedError` struct exposes full error messages, file paths, byte offsets, and help text via `serde::Serialize`. In a server context where MDS compilation errors are returned as JSON API responses, this could leak internal file paths or source structure to end users. Consider documenting that callers should sanitize `SerializedError` before exposing to untrusted consumers.

- **FileSystem trait security contract is advisory only** - `crates/mds-core/src/fs.rs:27-42` (Confidence: 62%) — The doc comment on the `FileSystem` trait documents a security contract (path traversal prevention, null-byte rejection, size limits), but there is no enforcement mechanism. A custom `FileSystem` implementation via `ModuleCache::with_fs()` can silently skip all security checks. Consider noting this risk in the public API docs for `with_fs()`.

- **NativeFs::canonicalize does not validate input** - `crates/mds-core/src/fs.rs:343-348` (Confidence: 60%) — Unlike `normalize()`, the new `canonicalize()` does not check for null bytes or empty strings before passing to `std::fs::canonicalize`. Currently the only caller (`resolve_source`) passes `base_dir.display().to_string()` which is unlikely to contain null bytes, but this is not enforced.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The codebase demonstrates strong security fundamentals: existing symlink rejection in `normalize()`, null-byte guards, path traversal prevention via `check_path_traversal()`, file size limits with TOCTOU-safe patterns, and well-tested boundary validation. No `unsafe` code exists anywhere in the diff.

The one blocking issue is that the new `NativeFs::canonicalize()` follows symlinks (via `std::fs::canonicalize`) without the symlink rejection that `normalize()` applies via `check_symlink()`. Since `canonicalize()` is used to establish the root directory in `resolve_source()`, a symlink in `base_dir` could re-anchor the security boundary. The practical exploitability is limited (the caller controls `base_dir`), but the inconsistency with the existing symlink-rejection policy warrants a fix before merge.

The serialization changes (`SerializedError`, `SerializedSpan`, `CompileOutput`) are clean from a security perspective -- no deserialization is introduced (only `Serialize`), no user input flows into the serialization logic, and the `compute_line_column` function has proper bounds checking.

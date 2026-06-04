# Security Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Path traversal via `mds.json` `output_dir` field** - `src/main.rs:136`
**Confidence**: 85%
- Problem: The `output_dir` value from `mds.json` is joined directly to `config_dir` without any path traversal validation. A malicious or misconfigured `mds.json` with `"output_dir": "../../sensitive_dir"` would cause the compiler to create directories and write files outside the project boundary. While import paths have `validate_import_path` (rejects non-relative, checks null bytes) and the resolver has `root_dir` boundary enforcement, the output path has no equivalent protection. The `load_config` function walks up the directory tree, so a `mds.json` placed in a parent directory could direct output to arbitrary relative locations.
- Fix: Validate that the resolved output directory remains within the project root or the config directory:
```rust
// In resolve_output_path, after step 5 computes `dir`:
let dir = config_dir.join(output_dir);
// Canonicalize and verify it stays within config_dir
let canonical_dir = dir.canonicalize().unwrap_or(dir.clone());
if !canonical_dir.starts_with(config_dir) {
    return Err(miette::miette!(
        "mds.json output_dir escapes project directory: {}",
        output_dir
    ));
}
```
Alternatively, reject `output_dir` values that contain `..` components, matching the pattern already used for `mds init` filenames.

**No file size limit on `mds.json` reads** - `src/main.rs:51`
**Confidence**: 82%
- Problem: `load_config` calls `std::fs::read_to_string(&candidate)` with no size check. The library enforces `MAX_FILE_SIZE = 10MB` for all compiled files and vars files, and stdin has `MAX_STDIN_SIZE`. However, `mds.json` is read without any size guard. A maliciously large `mds.json` (e.g., symlinked to `/dev/zero` on Linux or a multi-GB file) could cause memory exhaustion before the JSON parser even starts. While `mds.json` is a user-authored config file (reducing the attack surface), the pattern of size-guarding all file reads is established elsewhere in the codebase and should be applied consistently.
- Fix: Read as bytes with a size check before parsing:
```rust
let bytes = std::fs::read(&candidate).map_err(|e| {
    miette::miette!("cannot read {}: {e}", candidate.display())
})?;
if bytes.len() > 1_048_576 {  // 1 MB — generous for a config file
    return Err(miette::miette!(
        "mds.json too large ({} bytes): {}",
        bytes.len(),
        candidate.display()
    ));
}
let raw = String::from_utf8(bytes).map_err(|e| {
    miette::miette!("invalid UTF-8 in {}: {e}", candidate.display())
})?;
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`validate_and_read_file` performs security checks but is bypassed for cache hits that are stale** - `src/resolver.rs:155-167`
**Confidence**: 80%
- Problem: The refactored `resolve()` calls `validate_and_read_file` before checking the cache. This means every cache hit still pays the cost of `canonicalize()` + symlink detection + `std::fs::read()` + size check, even though the file was already fully validated and cached. While this is primarily a performance concern (not blocking), there is a subtle security implication: the file is read from disk on every call, but the cached `ResolvedModule` (not the fresh read) is returned. This means the security checks run against the current file state, but stale cached data is returned. If a file is modified between the first resolve and a subsequent cache-hit resolve, the validation checks pass against the new content but the old cached module is served. This is a TOCTOU-class inconsistency -- though exploitation requires the attacker to modify files mid-compilation, which is a narrow window.
- Fix: Move the cache check before the file read to avoid the redundant I/O, and document the design choice:
```rust
pub fn resolve(...) -> Result<Arc<ResolvedModule>, MdsError> {
    // Quick canonicalize for cache key
    let canonical = path.canonicalize()
        .map_err(|_| MdsError::file_not_found(path.display().to_string()))?;
    
    // Check cache first (before I/O)
    if let Some(cached) = self.modules.get(&canonical) {
        return Ok(Arc::clone(cached));
    }
    
    // Full validation + read for cache misses
    let (source, canonical, is_md) = self.validate_and_read_file(path)?;
    // ... rest of resolve
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Symlink detection only covers the final path component** - `src/resolver.rs:75-110`
**Confidence**: 82%
- Problem: The new two-step symlink detection strategy (canonicalize parent + compare with full canonicalize) only detects symlinks in the final path component. If a directory in the middle of the import path is a symlink (e.g., `./dir_symlink/module.mds` where `dir_symlink` is a symlink to `../../outside`), the `canonical_parent` step resolves it transparently and the root_dir check catches the escape. However, the symlink error message ("symlinks are not allowed in imports") is not triggered for directory-level symlinks -- only the generic "import path escapes project directory" message appears. This is functionally safe (the root_dir check is the real boundary) but the symlink policy is incompletely enforced, which could confuse users who expect consistent symlink rejection.

**`serde_yaml` replaced with `serde_yml` -- new dependency chain** - `Cargo.toml:11`, `Cargo.lock:491-503`
**Confidence**: 80%
- Problem: The migration from `serde_yaml` (deprecated) to `serde_yml 0.0.12` is reasonable, but `serde_yml` depends on `libyml 0.0.5` (a Rust port of libyaml). The `0.0.x` version numbers indicate pre-stability. While this is a pure Rust crate (no `unsafe-libyaml`), YAML parsers are historically a source of denial-of-service vulnerabilities (billion laughs, deep recursion). The codebase does have `MAX_VALUE_DEPTH = 64` which bounds post-parse traversal, but the `serde_yml::from_str` call itself runs before depth checking. Worth monitoring for CVEs.

## Suggestions (Lower Confidence)

- **`load_config` does not canonicalize the walk path** - `src/main.rs:46-69` (Confidence: 70%) -- The upward directory walk uses raw `current.parent()` without canonicalization. On a filesystem with symlinked directories in the path, this could skip `mds.json` or find an unexpected one. Low practical impact since the walk always starts from the input file's parent.

- **`--out-dir` and `-o` accept arbitrary paths without sandboxing** - `src/main.rs:97-157` (Confidence: 65%) -- Unlike import paths, CLI output paths have no boundary enforcement. A user running `mds build foo.mds -o /etc/cron.d/evil` would write there if permissions allow. This is by design for CLI tools (the user explicitly chose the path), but contrasts with the import path sandboxing. Not an issue for normal use.

- **`MdsError` now derives `Clone`** - `src/error.rs:21` (Confidence: 62%) -- Deriving `Clone` on `MdsError` means source strings embedded in error spans can be cloned, potentially doubling memory for large source files in error paths. Not a security vulnerability but worth noting for resource-exhaustion awareness.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The security posture of this PR is strong overall. The existing security guards (symlink detection, path traversal prevention via root_dir, import path validation, MAX_FILE_SIZE, MAX_IMPORT_DEPTH, resource limits) are well-maintained through the refactoring. The new features (EvalContext, CapturedScope, IndexSet, Arc wrapping) are structural improvements that do not introduce security regressions.

The two blocking MEDIUM issues both relate to the newly added `mds.json` config feature:
1. The `output_dir` field lacks path traversal validation, unlike import paths which have multiple layers of defense.
2. The config file read lacks a size guard, unlike all other file reads in the codebase.

Both are straightforward fixes that align with existing patterns in the codebase. The `validate_and_read_file` cache ordering issue in "Should Fix" is a TOCTOU inconsistency with a very narrow exploitation window, but fixing it also improves performance.

Conditions for approval:
- Add path traversal validation for `mds.json` `output_dir` (reject `..` components or verify resolved path stays within project boundary)
- Add a size limit to `mds.json` file reads

# Security Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Deprecated `serde_yaml` crate (0.9.34+deprecated) creates supply-chain risk** - `Cargo.toml:9`
**Confidence**: 90%
- Problem: The `serde_yaml` crate is officially deprecated by its maintainer (David Tolnay). The version string itself contains `+deprecated`. This crate will receive no further security patches. YAML parsing is a known attack surface — the underlying `unsafe-libyaml` (0.2.11) contains `unsafe` C bindings. Future CVEs discovered in the YAML parsing layer will not be fixed upstream.
- Fix: Migrate to the maintained successor crate. The community-recommended replacement is `serde_yml` or, for simpler needs, parse YAML frontmatter manually (the project already does line-by-line `type: mds` detection). Alternatively, adopt a safe YAML parser like `yaml-rust2`.
```toml
# Cargo.toml — replace:
# serde_yaml = "0.9"
# with:
serde_yml = "0.0.12"  # or yaml-rust2
```

### MEDIUM

**Symlink check is racy (TOCTOU) — symlink created between `symlink_metadata` and `canonicalize`** - `src/resolver.rs:72-84`
**Confidence**: 82%
- Problem: The resolver checks `symlink_metadata(path)` to reject symlinks, then separately calls `path.canonicalize()`. Between these two calls, an attacker with local filesystem access could replace a regular file with a symlink, bypassing the symlink check. The canonical path would then follow the symlink to an arbitrary location. The subsequent `starts_with(root)` check mitigates full path-traversal, but the symlink ban itself is bypassable.
- Fix: Since `canonicalize()` already resolves symlinks, the defense-in-depth approach is to also compare the pre-canonicalized path with the post-canonicalized path. If they differ (meaning a symlink was followed), reject the import. This eliminates the TOCTOU window because the comparison uses the result of the same `canonicalize` call.
```rust
let canonical = path.canonicalize()
    .map_err(|_| MdsError::file_not_found(path.display().to_string()))?;

// If canonicalize resolved any symlinks, the canonical path will differ
// from the absolute version of the original path.
let abs_path = std::fs::canonicalize(path.parent().unwrap_or(Path::new(".")))
    .map(|parent| parent.join(path.file_name().unwrap_or_default()));
if let Ok(expected) = abs_path {
    if expected != canonical {
        return Err(MdsError::import_error(format!(
            "symlinks are not allowed in imports: {}",
            path.display()
        )));
    }
}
```

**`mds init` does not reject absolute paths for the output filename** - `src/main.rs:304-313`
**Confidence**: 85%
- Problem: The `mds init` command validates against `..` path components but does not reject absolute paths (e.g., `mds init /etc/cron.d/malicious`). An attacker who can invoke the CLI (e.g., via a wrapper script) could write the starter template to an arbitrary absolute path on the filesystem. While the content written is benign (a starter MDS template), allowing arbitrary path writes is a security anti-pattern.
- Fix: Add an absolute path check alongside the existing `..` component check.
```rust
Commands::Init { filename, force } => {
    if filename.is_absolute() {
        return Err(miette::miette!(
            "init filename must be a relative path"
        ));
    }
    if filename
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(miette::miette!(
            "init filename must not contain '..' components"
        ));
    }
    // ...
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`resolve_source` passes un-canonicalized `base_dir` to `process_module` while `root_dir` is canonicalized** - `src/resolver.rs:165-181`
**Confidence**: 80%
- Problem: When `resolve_source` is called, it canonicalizes `base_dir` for `root_dir` (line 176), but passes the original un-canonicalized `base_dir` to `process_module` (line 180). Inside `process_module`, `resolve_path(base_dir, path)` joins the un-canonicalized `base_dir` with the import path. The resulting path is then passed to `self.resolve()`, which will canonicalize it and check `starts_with(root)`. This works correctly because `canonicalize` resolves the path fully. However, if `base_dir` itself does not exist on disk (e.g., a temp directory that was deleted), `canonicalize` will fail with a misleading "file not found" error on the imported file rather than on the base directory. This is a correctness/clarity issue more than a direct security hole, but inconsistent path handling near security boundaries is a risk multiplier.
- Fix: Pass the canonicalized base_dir to `process_module`:
```rust
let canonical_base = base_dir.canonicalize().map_err(|e| MdsError::Io {
    message: format!("cannot resolve base directory {}: {e}", base_dir.display()),
})?;
if self.root_dir.is_none() {
    self.root_dir = Some(canonical_base.clone());
}
self.process_module(source, "<source>", &canonical_base, false, runtime_vars, warnings)
```

## Pre-existing Issues (Not Blocking)

(none -- all code is new in this branch)

## Suggestions (Lower Confidence)

- **YAML billion-laughs / entity expansion** - `src/resolver.rs:558` (Confidence: 65%) -- The `serde_yaml::from_str` call parses arbitrary YAML from frontmatter. While the 10 MB file size limit bounds the raw input, deeply nested YAML anchor/alias references could expand beyond the input size. The `serde_yaml` / `unsafe-libyaml` stack does have some internal limits, but this is not explicitly bounded by the application. Consider documenting that the file size limit is the primary defense, or adding an explicit YAML size/complexity check.

- **No explicit size limit on `warnings` vector** - `src/evaluator.rs:24-37` (Confidence: 62%) -- The `warnings` vector grows unboundedly during compilation. A malicious template with thousands of `@include` directives targeting modules with empty bodies could generate a large number of warning strings. Unlikely to be exploitable for DoS given other limits, but a cap (e.g., 1000 warnings) would be a defense-in-depth measure.

- **`mds build -o` output path not validated** - `src/main.rs:263-265` (Confidence: 70%) -- The `-o` flag accepts any path (including absolute paths and paths with `..` components). While this is expected behavior for a CLI tool (users control their own filesystem), it means a wrapper script that passes user input to `-o` could be used for path traversal. Consider whether documentation should warn about this when embedding in automated pipelines.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The codebase demonstrates strong security awareness overall. Path traversal prevention (canonicalization + `starts_with` root check), symlink rejection, file size limits, import depth limits, loop iteration caps, output size caps, and nesting depth bounds are all present and well-implemented. The identifier validation (ASCII-only) reduces injection surface. The TOCTOU-safe file read pattern (read bytes first, then check size) is a good practice.

The blocking HIGH issue is the deprecated `serde_yaml` dependency, which creates ongoing supply-chain risk. The two MEDIUM blocking issues (TOCTOU in symlink check and missing absolute path rejection in `mds init`) should be addressed before merge as defense-in-depth improvements. None of these represent immediately exploitable vulnerabilities in the current context (local CLI tool), but they are the kind of issues that become security holes when the tool is embedded in larger systems or exposed to less-trusted input.

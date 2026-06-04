# Regression Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Error messages show filename-only instead of full canonical path for NativeFs** - `crates/mds-core/src/resolver.rs:157`
**Confidence**: 82%
- Problem: In `resolve_by_key`, `file_str` is set to `key_display_name(key)` which extracts just the filename (e.g., `"template.mds"` instead of `"/Users/project/templates/template.mds"`). Previously, `file_str` was set to `canonical.display().to_string()` (the full path). This value flows into tokenizer, parser, and validator error messages. When two files in different directories share the same name, error messages become ambiguous.
- Fix: Use the full key as `file_str` for NativeFs (where the key is the canonical path), and `key_display_name` only for VirtualFs or for cycle-string display:
```rust
// In resolve_by_key, line 157:
let ctx = ModuleCtx {
    file_str: key,  // Full key preserves NativeFs paths; VirtualFs keys are already short
    source: &source,
    base_key: key,
    runtime_vars,
};
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`validate_file_type` extension parsing differs from `Path::extension()` for dotfiles** - `crates/mds-core/src/resolver.rs:709-712`
**Confidence**: 80%
- Problem: The new string-based extension extraction using `rsplit('.')` treats dotfiles like `.mds` differently than the old `Path::extension()`. For a key `".mds"`: old code returned `None` (no extension, rejected); new code returns `Some("mds")` (accepted as valid `.mds` file). While extremely unlikely in practice, this changes the acceptance boundary for edge-case filenames.
- Fix: Add a leading-dot guard to match `Path::extension()` behavior:
```rust
let ext = filename
    .rsplit('.')
    .next()
    .filter(|e| *e != filename)
    .filter(|_| !filename.starts_with('.') || filename.matches('.').count() > 1);
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`resolve_key` bypasses all security checks when used with NativeFs** - `crates/mds-core/src/resolver.rs:219` (Confidence: 65%) -- The public `resolve_key` method delegates directly to `resolve_by_key` without calling `normalize()`. If a caller uses this with a NativeFs-backed cache, symlink checks, root initialization, and path traversal prevention are bypassed. The doc comment says "for virtual filesystems" but the type system does not enforce this. Consider making it available only on VirtualFs-specific constructors or adding a runtime guard.

- **`NativeFs::read` adds metadata pre-check creating TOCTOU window** - `crates/mds-core/src/fs.rs:284-291` (Confidence: 60%) -- The added `metadata()` call before `read()` introduces a TOCTOU window where the file size could change between the two calls. The post-read size check (lines 295-301) mitigates this, but the pre-check could give a false sense of security. Not a functional regression since the post-read check was always present.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

This is a well-executed refactoring that moves filesystem operations behind a `FileSystem` trait, enabling VirtualFs for WASM/testing. The migration is thorough:

- All 362 original tests pass unchanged, plus 45 new tests added (419 total).
- `ModuleCache` was not previously in the public API (`use` not `pub use`), so the `resolve()` -> `resolve_path()` rename is not a breaking change for external consumers.
- Security checks (symlink rejection, path traversal prevention, file size limits) are preserved and properly relocated to `NativeFs`.
- The `Default` implementation correctly delegates to `Self::new()` (NativeFs-backed).
- Import resolution path validation is centralized in `resolve_import_from()` without gaps.
- The `resolve_source` base_key sentinel pattern (`"{canonical_dir}/<source>"`) is tested and correct.

The one HIGH finding (filename-only error messages) is a behavioral change that could degrade the debugging experience for users with complex directory structures. The MEDIUM finding (dotfile extension parsing) is an extremely low-probability edge case but represents a subtle acceptance boundary change.

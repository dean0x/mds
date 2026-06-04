# Security Review Report

**Branch**: feat-workspace-split -> main
**Date**: 2026-05-17
**PR**: #10

## Issues in Your Changes (BLOCKING)

No CRITICAL or HIGH severity security issues found in the changed lines.

## Issues in Code You Touched (Should Fix)

No security issues identified in code adjacent to changes.

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing security issues identified.

## Suggestions (Lower Confidence)

No lower-confidence suggestions.

## Analysis Notes

This PR is a structural refactoring that converts a single-crate Rust project into a Cargo workspace with two members (`mds-core` library + `mds-cli` binary). The security analysis focused on the following areas:

### 1. Dependency and Supply Chain (OWASP A06)

The workspace split redistributes the same dependencies across two crates without introducing new ones. The dependency versions remain identical. `mds-core` correctly omits `clap` (CLI-only), and `mds-cli` correctly omits `thiserror`, `serde_yml`, and `indexmap` (library-only). The `miette` dependency in `mds-core` drops the `fancy` feature (appropriate for a library), while `mds-cli` retains it. No new attack surface introduced.

### 2. Import Consolidation in main.rs

The only behavioral code change is consolidating two separate `use` statements into one:
- `use mds::MAX_TRAVERSAL_DEPTH;` (was a standalone `use` with doc comment)
- `use mds::MAX_FILE_SIZE as MAX_STDIN_SIZE;` (was a standalone `use` with doc comment)

Both are now `use mds::{MdsError, MAX_FILE_SIZE as MAX_STDIN_SIZE, MAX_TRAVERSAL_DEPTH};` at line 9. This is purely cosmetic. The same constants are imported with the same values. The `MAX_STDIN_SIZE` alias (used for stdin size limiting) and `MAX_TRAVERSAL_DEPTH` (used for directory walk bounding in `load_config`) continue to be enforced at the same call sites with identical semantics.

### 3. Result Type Unification in main.rs

All function signatures were changed from `std::result::Result<T, miette::Error>` to `Result<T>` (using `miette::Result` alias). This is a type-level cosmetic change with zero runtime impact. All error handling paths remain identical.

### 4. Security Controls Preserved

The following pre-existing security controls were verified to be preserved across the workspace split (all source files moved without modification):

- **Path traversal prevention** (`resolver.rs`): `check_path_traversal`, `find_project_root`, `canonicalize_and_check` -- unchanged
- **Symlink detection** (`resolver.rs`): `check_symlink` -- unchanged
- **Import depth limiting** (`resolver.rs`): `MAX_IMPORT_DEPTH = 64` -- unchanged
- **File size limiting** (`resolver.rs`): `MAX_FILE_SIZE = 10MB`, TOCTOU-safe read-then-check -- unchanged
- **Resource limits** (`evaluator.rs`): call depth, loop iterations, output size, warning cap -- unchanged
- **Stdin size limiting** (`main.rs`): `MAX_STDIN_SIZE` via `take()` + length check -- unchanged
- **Config size limiting** (`main.rs`): `MAX_CONFIG_SIZE`, TOCTOU-safe -- unchanged
- **Directory traversal depth** (`main.rs`, `resolver.rs`): `MAX_TRAVERSAL_DEPTH = 256` -- unchanged
- **Init filename traversal rejection** (`main.rs`): `..` component check -- unchanged
- **Directory input rejection** (`main.rs`): `reject_directory_input` -- unchanged
- **Dot-path segment limiting** (`limits.rs`, `parser.rs`, `evaluator.rs`): `MAX_DOT_SEGMENTS = 32` -- unchanged

### 5. Integration Test Change

The `not_mds_file_error` test was updated to use a local fixture (`not_mds.md`) instead of referencing `spec.md` via `CARGO_MANIFEST_DIR`. The new fixture is a minimal 5-line markdown file with benign content. This is a correct adaptation to the workspace layout change (the spec file is no longer at the crate root).

### 6. New Fixture File

`crates/mds-cli/tests/fixtures/not_mds.md` contains only a YAML frontmatter block with `title: Plain Markdown` and one line of body text. No security concern.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 10/10
**Recommendation**: APPROVED

This PR is a clean structural refactoring with zero behavioral changes. All security controls from the original single-crate layout are preserved without modification in their new workspace locations. The only code changes are cosmetic (import consolidation and Result type alias unification in main.rs). No new attack surface, no weakened controls, no credential exposure, no dependency additions.

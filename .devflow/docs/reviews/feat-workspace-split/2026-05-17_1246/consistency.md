# Consistency Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing `[workspace.dependencies]` for shared dependency versions** - `Cargo.toml:1-3`
**Confidence**: 82%
- Problem: Both `mds-core` and `mds-cli` declare overlapping dependencies (`serde`, `serde_json`, `miette`, `tempfile`) with independent version specifiers. The workspace root `Cargo.toml` does not use `[workspace.dependencies]` to centralize these. As the workspace grows, this risks version drift between crates.
- Current versions are aligned today (`serde = "1"`, `serde_json = "1"`, `miette = "7"`, `tempfile = "3"`), so this is not blocking.
- Fix: Add a `[workspace.dependencies]` section to the root `Cargo.toml` and reference them with `workspace = true` in each crate:
  ```toml
  # Cargo.toml (root)
  [workspace]
  members = ["crates/mds-core", "crates/mds-cli"]
  resolver = "2"

  [workspace.dependencies]
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  miette = "7"
  tempfile = "3"
  ```
  ```toml
  # crates/mds-core/Cargo.toml
  [dependencies]
  serde = { workspace = true }
  serde_json = { workspace = true }
  miette = { workspace = true }
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Workspace-level metadata inheritance** - `crates/mds-core/Cargo.toml:1`, `crates/mds-cli/Cargo.toml:1` (Confidence: 65%) -- Both crates repeat `version`, `edition`, `rust-version`, `license`, `repository`. These could be inherited via `[workspace.package]` to ensure they stay in sync as the project evolves.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | 0 |
| Pre-existing | - | - | 0 | 0 |

**Consistency Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

This is a clean workspace split with strong consistency characteristics:

1. **Result type unification** -- The branch actively improved consistency by replacing a mix of `std::result::Result<T, miette::Error>` and `Result<T, miette::Error>` with the uniform `Result<T>` alias across all 11 functions in `main.rs`. This is a positive consistency change.

2. **Import consolidation** -- Scattered `use mds::MAX_TRAVERSAL_DEPTH` and `use mds::MAX_FILE_SIZE as MAX_STDIN_SIZE` items were hoisted into a single grouped `use mds::{...}` statement at the top of the file, matching Rust community conventions.

3. **Public API preservation** -- The `[lib] name = "mds"` in `mds-core` and `[[bin]] name = "mds"` in `mds-cli` ensure that both `use mds::*` imports and the `mds` CLI binary name are unchanged. Zero consumer-facing regression.

4. **Metadata alignment** -- Both crates share identical `version`, `edition`, `rust-version`, `license`, `readme`, `repository`, and `keywords`. Categories are correctly split (`template-engine`/`text-processing` for core, `command-line-utilities` for CLI).

5. **Dependency feature split** -- `miette` correctly has `features = ["fancy"]` only in the CLI crate (user-facing error rendering), while the library uses the base crate. This is intentional and well-reasoned.

The only condition is the `[workspace.dependencies]` deduplication, which prevents version drift as the workspace grows. All other patterns are consistent and the refactoring preserves behavioral parity.

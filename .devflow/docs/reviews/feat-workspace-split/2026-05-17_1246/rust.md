# Rust Review Report

**Branch**: feat-workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Workspace metadata duplication -- use `[workspace.package]` inheritance** - `Cargo.toml`, `crates/mds-core/Cargo.toml:3-9`, `crates/mds-cli/Cargo.toml:3-9`
**Confidence**: 85%
- Problem: `version`, `edition`, `rust-version`, `license`, and `repository` are duplicated verbatim across both crate manifests. When the version bumps to 0.2.0, both files must be updated in lockstep -- a maintenance hazard that Cargo workspace inheritance was designed to eliminate.
- Fix: Add a `[workspace.package]` section to the root `Cargo.toml` and use `field.workspace = true` in each member:
```toml
# Root Cargo.toml
[workspace]
members = ["crates/mds-core", "crates/mds-cli"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
license = "MIT"
repository = "https://github.com/deanshrn/mdl"
```
```toml
# Each crate's Cargo.toml
[package]
name = "mds-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
# ...crate-specific fields remain here
```

**Shared dependency versions not centralized via `[workspace.dependencies]`** - `crates/mds-core/Cargo.toml:17-23`, `crates/mds-cli/Cargo.toml:19-22`
**Confidence**: 82%
- Problem: `serde`, `serde_json`, and `miette` appear in both crate manifests with independent version specifiers. As the workspace grows, keeping these synchronized requires manual cross-file updates. Cargo's `[workspace.dependencies]` centralizes version pins.
- Fix: Add shared dependencies to root `Cargo.toml` and reference them with `.workspace = true`:
```toml
# Root Cargo.toml
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
miette = "7"
tempfile = "3"
```
```toml
# crates/mds-cli/Cargo.toml
[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
miette = { workspace = true, features = ["fancy"] }
```

## Pre-existing Issues (Not Blocking)

No critical pre-existing issues in reviewed files.

## Suggestions (Lower Confidence)

- **`serde_yml` at 0.0.x is a pre-release dependency** - `crates/mds-core/Cargo.toml:20` (Confidence: 65%) -- The comment acknowledges this is pre-release. Consider checking if a stable alternative exists (e.g., `serde_yaml` or a newer `serde_yml` release) before publishing to crates.io, as downstream users inherit this transitive dependency.

- **CLI `serde` dependency may be removable** - `crates/mds-cli/Cargo.toml:20` (Confidence: 60%) -- The CLI only uses `#[derive(Deserialize)]` for `MdsConfig` and `BuildConfig`. If these structs were moved into `mds-core` (where `serde` already lives), the CLI could drop its direct `serde` dependency entirely. This is a minor optimization and depends on whether you want the config types in the library.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 2 | - |
| Pre-existing | - | - | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The workspace split is clean and well-executed. The binary name (`mds`) is preserved via `[[bin]]`, the library name (`mds`) is preserved via `[lib] name`, test fixtures moved correctly, and the single behavioral test change (switching from `spec.md` to a dedicated `not_mds.md` fixture) is a strict improvement. All 354 tests pass, clippy reports zero warnings, and there is no unsafe code.

The two MEDIUM findings are both about using Cargo workspace inheritance (`[workspace.package]` and `[workspace.dependencies]`) to avoid metadata and version duplication across crates. These are best-practice hygiene items worth addressing while the workspace is small (2 members), but they do not affect correctness or behavior.

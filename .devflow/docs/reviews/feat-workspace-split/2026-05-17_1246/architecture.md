# Architecture Review Report

**Branch**: feat-workspace-split -> main
**Date**: 2026-05-17
**PR**: #10

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Duplicated dependency versions without `[workspace.dependencies]`** - `Cargo.toml`, `crates/mds-core/Cargo.toml`, `crates/mds-cli/Cargo.toml`
**Confidence**: 85%
- Problem: The workspace `Cargo.toml` defines no `[workspace.dependencies]` section. Shared dependencies (`serde`, `serde_json`, `miette`) are declared independently in both crate manifests with version strings that must be kept in sync manually. With only 2 crates this is manageable, but it violates the DRY principle and creates a maintenance hazard as the workspace grows. The `miette` dependency already has a subtle divergence: `mds-core` uses `version = "7"` (no features) while `mds-cli` uses `version = "7", features = ["fancy"]` -- this is intentional and correct (library vs binary), but having both declared independently makes it harder to verify intentionality at a glance.
- Fix: Add `[workspace.dependencies]` to the root `Cargo.toml` and reference them via `dep.workspace = true` in each crate:
  ```toml
  # Root Cargo.toml
  [workspace]
  members = ["crates/mds-core", "crates/mds-cli"]
  resolver = "2"

  [workspace.dependencies]
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  miette = "7"
  thiserror = "2"
  indexmap = "2.2"
  clap = { version = "4", features = ["derive"] }
  tempfile = "3"

  # crates/mds-cli/Cargo.toml
  [dependencies]
  serde = { workspace = true }
  serde_json = { workspace = true }
  miette = { workspace = true, features = ["fancy"] }
  ```

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

### MEDIUM

**MdsConfig / BuildConfig types duplicated in CLI** - `crates/mds-cli/src/main.rs:14-23`
**Confidence**: 82%
- Problem: The `MdsConfig` and `BuildConfig` structs for parsing `mds.json` are defined only in the CLI binary. If a future consumer (e.g., an LSP, a build system plugin, or a library-level "project mode" API) needs to discover and load `mds.json`, these types and the `load_config` function would need to be duplicated or extracted. For a v0.1 split this is acceptable -- keeping CLI-only concerns out of the core library is the right instinct. This is noted as a future extraction candidate, not a current deficiency.
- Recommendation: If `mds.json` discovery becomes needed outside the CLI, extract `MdsConfig`, `BuildConfig`, and `load_config` into `mds-core` behind a `project` or `config` feature flag.

## Suggestions (Lower Confidence)

- **Version field sync risk** - `crates/mds-core/Cargo.toml:3`, `crates/mds-cli/Cargo.toml:3` (Confidence: 70%) -- Both crates declare `version = "0.1.0"` independently. Consider whether these should stay in lockstep (use a workspace-level `version` field or a release script) or diverge independently (semver for each crate). The choice should be documented.

- **Missing `[workspace.package]` for shared metadata** - `Cargo.toml` (Confidence: 65%) -- Fields like `edition`, `rust-version`, `license`, `repository` are duplicated across both crate manifests. Cargo supports `[workspace.package]` inheritance which would deduplicate these.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

This is a clean, well-executed workspace split. The architectural decisions are sound:

1. **Correct dependency direction**: `mds-cli` depends on `mds-core`, with no reverse dependency. The library crate (`mds-core`) has zero knowledge of the CLI.

2. **Clean separation of concerns**: CLI-specific types (`MdsConfig`, `BuildConfig`, `Cli`, `Commands`) stay in the binary crate. The library exposes only compilation and validation APIs. The `lib` name `mds` is preserved via `[lib] name = "mds"`, maintaining backward compatibility for downstream `use mds::*` imports.

3. **No behavioral changes**: The diff is purely structural -- file moves, import consolidation, and one test fixture replacement (`spec.md` -> `not_mds.md`). The PR description's claim of "zero behavioral changes" is confirmed by the diff.

4. **Correct feature gating**: `miette`'s `fancy` feature (which pulls in terminal formatting dependencies) is enabled only in the CLI crate, not the library -- appropriate since library consumers should control their own error rendering.

5. **Import cleanup**: The consolidation of `MAX_FILE_SIZE` and `MAX_TRAVERSAL_DEPTH` imports into the top-level `use` block and unification of `Result` types from mixed `std::result::Result<T, miette::Error>` to `miette::Result<T>` improves consistency.

The single blocking item (missing `[workspace.dependencies]`) is a low-risk maintenance concern that should be addressed before the workspace grows, but does not affect correctness or the architectural quality of the split itself.

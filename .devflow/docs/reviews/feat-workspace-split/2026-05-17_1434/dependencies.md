# Dependencies Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Pre-release version comment misplaced** - `Cargo.toml:17`
**Confidence**: 82%
- Problem: The `serde_yml` pre-release tracking comment (`# Pre-release (0.0.x); track for 0.1.x stability milestone`) was removed from the root `Cargo.toml` `[workspace.dependencies]` section where the version is actually pinned. It now only exists in `crates/mds-core/Cargo.toml:19` which uses `workspace = true` (meaning the version is not specified there). The comment should live where the version is defined, so that maintainers see the tracking note when updating versions.
- Fix: Move the comment to the root `Cargo.toml` above the `serde_yml` entry:
  ```toml
  [workspace.dependencies]
  indexmap = "2.2"
  serde = { version = "1", features = ["derive"] }
  # Pre-release (0.0.x); track for 0.1.x stability milestone
  serde_yml = "0.0.12"
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Pre-release dependency: serde_yml 0.0.12** - `Cargo.toml:17`
**Confidence**: 85%
- Problem: `serde_yml` is at version `0.0.12` (pre-release, no semver stability guarantees). Any `0.0.x` bump could contain breaking changes. The comment in the codebase already tracks this, but it remains a supply-chain risk for long-term maintenance.
- Fix: Monitor for `serde_yml` 0.1.x release and upgrade when available. Consider evaluating `serde_yaml` (archived but stable at 0.9.x) vs `serde_yml` maturity.

## Suggestions (Lower Confidence)

- **Version range width for `miette = { version = "7" }`** - `Cargo.toml:19` (Confidence: 65%) -- Using `"7"` is equivalent to `>=7.0.0, <8.0.0`. Since miette follows semver, this is acceptable, but you could pin to `"7.6"` for narrower range if desired. Not a real issue given the lockfile is committed.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Dependencies Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Observations

- Workspace dependency centralization is well-structured with `[workspace.dependencies]`
- Feature flag separation for `miette` is correct (base features in workspace, `fancy` added only in CLI crate)
- No new external dependencies introduced -- purely a reorganization
- Lockfile committed and minimal diff (no unexpected transitive dependency changes)
- `resolver = "2"` is correctly specified
- The `mds-core` path dependency uses `package = "mds-core"` alias correctly
- All crate metadata properly inherits from workspace (`version.workspace = true`, etc.)

### Condition for Merge

The misplaced pre-release comment is a minor documentation concern. Recommend moving the `serde_yml` tracking comment to the workspace root where the version is defined, but this does not need to block the merge.

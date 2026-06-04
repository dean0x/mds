# Dependencies Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

No should-fix issues found.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Pre-release dependency: serde_yml 0.0.12** - `Cargo.toml:12`
**Confidence**: 90%
- Problem: `serde_yml` is pinned at `0.0.12`, a pre-release version (0.0.x semver). Pre-release packages have no stability guarantees -- any patch bump can contain breaking changes. The comment added in this PR acknowledges this (`# Pre-release (0.0.x); track for 0.1.x stability milestone`), which is good documentation but does not mitigate the risk. Additionally, `serde_yml` pulls in `libyml v0.0.5` (also pre-release) which depends on `anyhow` -- a somewhat heavy error-handling crate for a serialization library.
- Fix: Continue tracking for 0.1.x as noted. No immediate action needed, but consider evaluating `serde_yaml` (the more established crate) or monitoring `serde_yml` release cadence. The added comment is the right approach for now.

## Suggestions (Lower Confidence)

- **Version range consistency** - `Cargo.toml:7-16` (Confidence: 65%) -- The project uses a mix of major-version pinning (`"4"`, `"1"`, `"7"`, `"2"`) and minor-version pinning (`"2.2"`, `"0.0.12"`). Consider adopting a consistent policy (e.g., always `major.minor` for all deps) to make version management predictable.

- **No cargo-audit in CI** - (Confidence: 70%) -- `cargo audit` is not installed, suggesting no automated vulnerability scanning in the CI pipeline. For a project with 8 dependencies (and their transitive tree), periodic auditing would catch known CVEs early.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Changes in this PR**: The dependency changes are minimal and well-considered:

1. **indexmap "2" -> "2.2"**: Tightens the minimum version constraint. The lockfile-resolved version (2.14.0) is unchanged, so this has zero practical impact on the build. It documents that the codebase requires at least 2.2 features (IndexSet was stabilized in early 2.x but 2.2 is a reasonable floor). Good hygiene.

2. **Added comment on serde_yml**: Documents the pre-release risk with a tracking note. This is purely informational and improves maintainability.

No new dependencies were added. No dependencies were removed. The lockfile is unchanged. The dependency tree remains lean (8 direct dependencies, reasonable transitive tree).

**Dependencies Score**: 9/10
**Recommendation**: APPROVED

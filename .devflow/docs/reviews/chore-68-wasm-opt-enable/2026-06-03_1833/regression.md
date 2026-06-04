# Regression Review Report

**Branch**: chore/68-wasm-opt-enable -> main
**Date**: 2026-06-03
**PR**: #73

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Analysis Details

### Regression Checklist

- [x] No exports removed without deprecation -- no exports were changed
- [x] Return types backward compatible -- no API changes
- [x] Default values unchanged (or documented) -- wasm-opt default changed from `false` to `["-Oz", ...]` which is an intentional re-enablement, not a regression
- [x] Side effects preserved -- build outputs the same WASM modules, now optimized for size
- [x] All consumers of changed code updated -- CI and release workflows both updated consistently
- [x] Migration complete across codebase -- no stale references to `wasm-opt = false` or disabled wasm-opt remain
- [x] CLI options preserved or deprecated -- N/A
- [x] API endpoints preserved or versioned -- N/A
- [x] Commit message matches implementation -- verified: PR re-enables wasm-opt with explicit feature flags, code does exactly that
- [x] Breaking changes documented -- CLAUDE.md and RELEASING.md both updated with new Binaryen requirement

### Intent vs Reality Verification

The PR description states: "Re-enables wasm-opt with -Oz and explicit post-MVP feature flags. Installs Binaryen v129 via phi-ag/setup-binaryen (SHA-pinned). Pins wasm-pack to v0.14.0. Adds release-profile WASM test step and binary size reporting."

All claims verified in the diff:
1. wasm-opt re-enabled with `-Oz` and 4 explicit feature flags in `crates/mds-wasm/Cargo.toml` -- confirmed
2. Binaryen v129 installed via `phi-ag/setup-binaryen@f7f99985a69ad20f08b12ad725865b14a7f875a4` (SHA-pinned) in both `ci.yml` (WASM job and JS job) and `release.yml` (publish-npm job) -- confirmed
3. wasm-pack pinned to `v0.14.0` (from `latest`) in all 3 workflow locations -- confirmed
4. Release-profile WASM test step added (`wasm-pack test --node --release`) -- confirmed
5. Binary size reporting added with `::notice::` annotations -- confirmed

### Consistency Analysis (applies ADR-005)

All three workflow touchpoints are consistent:
- `ci.yml` WASM job: wasm-pack v0.14.0 + Binaryen v129 + SHA-pinned action
- `ci.yml` JS job (ubuntu/macos/windows): wasm-pack v0.14.0 + Binaryen v129 + SHA-pinned action
- `release.yml` publish-npm job: wasm-pack v0.14.0 + Binaryen v129 + SHA-pinned action

The JS job runs on all 3 OS targets (ubuntu-latest, macos-latest, windows-latest), providing cross-platform coverage for the new Binaryen dependency.

### Documentation Consistency

- `CLAUDE.md` gotcha updated: old text about "Local WASM builds require Binaryen" replaced with more specific guidance including the npm workspace command
- `RELEASING.md` notes section updated: old "wasm-opt is currently disabled" replaced with the new wasm-opt configuration and Binaryen install instructions
- Both docs consistently reference `brew install binaryen` (macOS) and `apt install binaryen` (Linux)

### Risk Assessment

- **wasm-pack pin from `latest` to `v0.14.0`**: Low risk. Pinning is more stable than floating on `latest`. No regression -- this is a reliability improvement.
- **New Binaryen dependency in CI**: Low risk. SHA-pinned action, specific version. `phi-ag/setup-binaryen` supports all 3 CI platforms.
- **wasm-opt re-enablement**: This is a build optimization change. The new `--release` test step (line 73 of ci.yml) explicitly validates that wasm-opt-processed binaries work correctly. The `continue-on-error: true` on the size reporting step is appropriate since it is informational only.
- **No behavioral regression**: The WASM module's API surface is unchanged. Only the binary size and optimization level change.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 10/10
**Recommendation**: APPROVED

This is a clean, well-structured infrastructure change. All modified locations (Cargo.toml, ci.yml, release.yml, CLAUDE.md, RELEASING.md) are internally consistent. The migration from `wasm-opt = false` to the new configuration is complete with no stale references. The new release-profile test step provides a CI safety net that validates wasm-opt-processed binaries before merge. Documentation is updated to reflect the new Binaryen requirement for local development.

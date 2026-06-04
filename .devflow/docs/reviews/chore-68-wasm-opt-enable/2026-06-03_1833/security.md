# Security Review Report

**Branch**: chore/68-wasm-opt-enable -> main
**Date**: 2026-06-03

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Inconsistent action pinning strategy** - `ci.yml:58`, `ci.yml:108`, `release.yml:254`
**Confidence**: 82%
- Problem: The newly added `phi-ag/setup-binaryen` action is correctly SHA-pinned, but the adjacent `jetli/wasm-pack-action@v0.4.0` (which this PR changed from `latest` to `v0.14.0`) uses a mutable tag reference rather than a commit SHA. Tag-based references can be force-pushed by the upstream maintainer, creating a supply chain attack vector (OWASP A08 - Software and Data Integrity Failures). Other pre-existing actions (`actions/checkout@v6`, `Swatinem/rust-cache@v2`, `actions/setup-node@v6`) are similarly tag-pinned, but those are first-party GitHub or high-trust ecosystem actions with tag immutability policies.
- Fix: Consider SHA-pinning `jetli/wasm-pack-action` to match the security posture of the new `phi-ag/setup-binaryen` reference. This is a pre-existing pattern issue and should not block this PR.

## Suggestions (Lower Confidence)

(none)

## Positive Security Observations

- **SHA-pinned third-party action** (`phi-ag/setup-binaryen@f7f99985a69ad20f08b12ad725865b14a7f875a4`): Full commit SHA pinning with human-readable version comment is the gold standard for GitHub Actions supply chain security. Consistently applied across all 3 workflow locations (ci.yml wasm job, ci.yml js job, release.yml publish-npm job). Applies ADR-005 (full CI validation for build tooling changes).
- **wasm-pack version pinned**: Changed from `version: latest` to `version: v0.14.0`, eliminating the risk of a malicious `latest` release being pulled into CI.
- **No secrets exposure**: No new secrets references or environment variables introduced. Binary size reporting script uses only hardcoded paths and derived numeric values in `::notice::` annotations -- no user-controlled input reaches workflow commands.
- **Static wasm-opt configuration**: The `wasm-opt` flags in `crates/mds-wasm/Cargo.toml` are static TOML values with no interpolation or injection surface.
- **`debug-panics` feature guard preserved**: The feature that leaks filesystem paths in panic messages remains properly documented as never-enable-in-production, and the wasm-opt changes do not interact with it.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

This PR improves the security posture of the CI pipeline by SHA-pinning a new third-party action and replacing a `latest` tag with a fixed version. The binary size reporting script introduces no injection surface. The one pre-existing observation (inconsistent pinning of `jetli/wasm-pack-action`) is informational and should not block merge.

# Consistency Review Report

**Branch**: chore/68-wasm-opt-enable -> main
**Date**: 2026-06-03

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Action pinning style inconsistency between `jetli/wasm-pack-action` and `phi-ag/setup-binaryen`** - `ci.yml:58`, `ci.yml:61`, `ci.yml:108`, `ci.yml:111`, `release.yml:254`, `release.yml:257`
**Confidence**: 82%
- Problem: `phi-ag/setup-binaryen` is SHA-pinned with a tag comment (`@f7f99985...  # v1.0.8`), while `jetli/wasm-pack-action` uses tag-only pinning (`@v0.4.0`). Both are third-party actions (not official `actions/*` or well-known ecosystem tools like `dtolnay/rust-toolchain`). The two actions are added adjacent to each other in all three locations, making the inconsistency visually prominent.
- Fix: Either SHA-pin both third-party actions consistently, or use tag-only pinning for both. SHA-pinning `jetli/wasm-pack-action` would be the more secure direction:
  ```yaml
  - uses: jetli/wasm-pack-action@<SHA>  # v0.4.0
  ```
  Or, if the team considers tag pinning acceptable for third-party actions, switch `setup-binaryen` to tag-only: `@v1.0.8`. Either approach is fine as long as both match.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Verify step only in `wasm` job, not `js` job** - `ci.yml:64-65` vs `ci.yml:111-113` (Confidence: 65%) -- The `wasm` job has a `Verify wasm-opt` step (`wasm-opt --version`) after installing Binaryen, but the `js` job does not. The verification is a nice-to-have diagnostic step. Its absence in the `js` job is a minor asymmetry, though the `js` job exercises wasm-opt implicitly via `wasm-pack build`. Not blocking since the `js` job would fail at the build step if wasm-opt were missing.

- **CLAUDE.md gotcha slightly less detailed than RELEASING.md** - `CLAUDE.md:37` vs `RELEASING.md:137-139` (Confidence: 62%) -- CLAUDE.md says "Local WASM builds require Binaryen for wasm-opt" without mentioning the version (v129), while RELEASING.md says "Binaryen v129 via `phi-ag/setup-binaryen`". The CLAUDE.md gotcha is intentionally terse (gotcha style), and `brew install binaryen` installs whatever version is current, so omitting "v129" is reasonable. Still, noting the current tested version could help developers troubleshoot version mismatches.

- **Binary size report step reports on `pkg/` but the debug build earlier writes to `pkg/` too** - `ci.yml:74-83` (Confidence: 60%) -- The binary size reporting step uses `if: always()` and reports sizes from `crates/mds-wasm/pkg/mds_wasm_bg.wasm`. The debug-mode test (`wasm-pack test --node`) builds to `pkg/` first, then the release test (`wasm-pack test --node --release`) may overwrite it. The `--release` flag on the test step should produce the optimized binary, but the interaction between `wasm-pack build` (which explicitly wrote to `pkg/`) and `wasm-pack test --release` (which may use a different output) could mean the size report reflects the initial `wasm-pack build` output rather than the release-optimized one. Confidence is low because `wasm-pack build` already uses the release profile by default.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | 0 |
| Pre-existing | - | - | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR is well-structured with strong internal consistency:
- All three `setup-binaryen` references use identical SHA pins, version numbers, and comment format
- All three `wasm-pack-action` references use identical version (`v0.14.0`)
- Documentation updates in CLAUDE.md, RELEASING.md, and Cargo.toml comments are aligned and tell the same story
- The wasm-opt flags in Cargo.toml match what the documentation describes
- CI workflow changes apply symmetrically across `wasm`, `js`, and `release` jobs where appropriate (applies ADR-005)

The one should-fix item is the mixed pinning strategy between two adjacent third-party actions. This is a minor style inconsistency, not a functional problem.

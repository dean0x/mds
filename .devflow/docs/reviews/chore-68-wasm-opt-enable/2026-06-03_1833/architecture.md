# Architecture Review Report

**Branch**: chore/68-wasm-opt-enable -> main
**Date**: 2026-06-03

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Duplicated CI toolchain setup block across 3 jobs** - `ci.yml:58-63`, `ci.yml:108-113`, `release.yml:254-259`
**Confidence**: 82%
- Problem: The wasm-pack + Binaryen setup sequence (action refs, version pins, SHA hashes) is copy-pasted identically into three separate workflow jobs (wasm, js, and publish-npm). This is content coupling at the CI layer -- when Binaryen or wasm-pack versions need updating, all three locations must be changed in lockstep. A drift between any two (e.g., one job gets updated to Binaryen v130 while another stays on v129) would produce silent build inconsistencies. This change introduced 2 of the 3 copies.
- Fix: GitHub Actions supports reusable workflows (`workflow_call`) and composite actions. Extract the wasm-pack + Binaryen setup into a local composite action (e.g., `.github/actions/setup-wasm/action.yml`) that encapsulates the version pins in one place:

```yaml
# .github/actions/setup-wasm/action.yml
name: Setup WASM toolchain
runs:
  using: composite
  steps:
    - uses: jetli/wasm-pack-action@v0.4.0
      with:
        version: v0.14.0
    - uses: phi-ag/setup-binaryen@f7f99985a69ad20f08b12ad725865b14a7f875a4  # v1.0.8
      with:
        version: 129
```

Then each job simply does `- uses: ./.github/actions/setup-wasm`. Version updates become single-point changes. (applies ADR-005 -- full CI across targets means this setup block runs in multiple OS/job contexts and must stay synchronized)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing `wasm-opt --version` verification in JS job and release workflow** - `ci.yml:114`, `release.yml:260` (Confidence: 65%) -- The dedicated WASM job has an explicit `wasm-opt --version` smoke-check step (ci.yml:64-65), but the JS job (ci.yml) and the publish-npm job (release.yml) skip it. If `setup-binaryen` silently fails or the PATH is misconfigured on a specific OS in the matrix, the build would fail with an opaque wasm-opt error rather than a clear "tool not found" message. Low-severity because wasm-pack itself will fail fast if wasm-opt is missing, but the diagnostic quality differs.

- **Binary size reporting only runs on debug-profile artifacts** - `ci.yml:74-83` (Confidence: 62%) -- The "Report WASM binary sizes" step runs with `if: always()` and measures `pkg/` and `pkg-web/` outputs. These are produced by `wasm-pack build` (which defaults to `--release`), so the sizes do reflect optimized output. However, the step runs even when the build step fails (`if: always()`), in which case `wc -c` on missing files produces confusing error output. The `continue-on-error: true` suppresses this, but the annotation stream will contain noise. Consider guarding with `if: success()` or checking file existence first.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The change is well-scoped and architecturally sound. It correctly separates build-time optimization config (Cargo.toml) from CI toolchain provisioning (workflow files), and updates documentation (CLAUDE.md, RELEASING.md) to reflect the new dependency. The single MEDIUM finding -- duplicated CI setup blocks -- is a DRY concern that becomes more important as the project grows but does not block this merge. The wasm-opt feature flags are correctly explicit rather than using a blanket `--enable-all` (which does not exist in Binaryen v129), showing good understanding of the tool's interface contract. SHA-pinning the `setup-binaryen` action with a version comment is a security best practice. Pinning `wasm-pack` to `v0.14.0` (away from `latest`) improves build reproducibility.

Condition: Consider extracting the duplicated wasm-pack + Binaryen setup into a composite action before adding any further jobs that need WASM tooling.

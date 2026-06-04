# Reliability Review Report

**Branch**: chore/68-wasm-opt-enable -> main
**Date**: 2026-06-03

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Binary size reporter silently produces incorrect output when WASM files are missing** - `.github/workflows/ci.yml:78-83`
**Confidence**: 82%
- Problem: The "Report WASM binary sizes" step runs with `if: always()` and `continue-on-error: true`, meaning it executes even when previous build/test steps fail and the `.wasm` files do not exist. When `wc -c < "$f"` fails on a missing file, the shell error is swallowed and the step emits no annotation (or a malformed one with empty values) without any visible indication that the data is invalid. The `for` loop itself is bounded (2 hardcoded paths), which is fine, but the lack of a file existence guard means CI annotations could be misleading on failed runs.
- Fix: Add a file existence check inside the loop:
  ```yaml
  run: |
    for f in crates/mds-wasm/pkg/mds_wasm_bg.wasm crates/mds-wasm/pkg-web/mds_wasm_bg.wasm; do
      if [ ! -f "$f" ]; then
        echo "::warning::WASM file missing: $f (build may have failed)"
        continue
      fi
      raw=$(wc -c < "$f" | tr -d ' ')
      gz=$(gzip -c "$f" | wc -c | tr -d ' ')
      label="${f#crates/mds-wasm/}"
      echo "::notice::WASM ${label}: ${raw} bytes raw, ${gz} bytes gzipped"
    done
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing `wasm-opt --version` verification in the JS job** - `.github/workflows/ci.yml:111-113` (Confidence: 70%) -- The `wasm` job verifies `wasm-opt --version` after installing Binaryen (line 64-65), but the `js` job installs the same action without a verification step. If Binaryen installation fails silently in the JS matrix, the WASM build step would produce a confusing error. Consider adding the same verification step for consistency.

- **No wasm-opt verification in release workflow** - `.github/workflows/release.yml:257-259` (Confidence: 65%) -- The release `publish-npm` job installs Binaryen but does not verify installation before building the WASM package. A silent installation failure during a release would produce a hard-to-diagnose build failure. The `wasm-pack build` would likely fail with a cryptic wasm-opt error rather than a clear "wasm-opt not found" message. Consider adding a verification step as in the CI wasm job.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Positive reliability observations**:
- Pinning wasm-pack to `v0.14.0` instead of `latest` eliminates a class of non-deterministic CI failures (applies ADR-005 -- build tooling changes validated with full CI)
- SHA-pinning `phi-ag/setup-binaryen` to a specific commit prevents supply-chain drift
- The `wasm-opt --version` verification step in the wasm CI job is good defensive practice
- The release-profile test step (`wasm-pack test --node --release`) validates the wasm-opt output actually works, not just that it compiles
- The `for` loop in the size reporter is bounded (2 hardcoded paths) -- no unbounded iteration risk
- `continue-on-error: true` on the reporter prevents an annotation failure from blocking the CI pipeline

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The single MEDIUM finding (missing file guard in the size reporter) is a minor robustness gap that only manifests on failed builds. It does not affect correctness, release safety, or production artifacts. The overall change is a clear reliability improvement: it pins tool versions, adds verification steps, and validates optimized output with a release-profile test. Recommend addressing the file guard for completeness, but this should not block merge.

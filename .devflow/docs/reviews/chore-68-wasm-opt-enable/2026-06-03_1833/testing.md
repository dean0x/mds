# Testing Review Report

**Branch**: chore/68-wasm-opt-enable -> main
**Date**: 2026-06-03

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Release-profile test does not exercise the `--release` build output** - `.github/workflows/ci.yml:72-73`
**Confidence**: 82%
- Problem: The new `wasm-pack test --node --release crates/mds-wasm` step runs tests against a release build, which exercises wasm-opt. However, the preceding build step (`wasm-pack build ... --out-dir pkg`) already produces a release build into `pkg/`. The test step rebuilds from scratch with `--release` internally (wasm-pack test compiles its own test harness), so it does validate that wasm-opt runs successfully on a release binary. The concern is that the build step (line 67-69) and the test step (line 73) are independent compilations -- if the build step's wasm-opt invocation fails (e.g., Binaryen rejects a flag), the test step could still pass because `wasm-pack test --release` compiles its own test binary, which may have different codegen characteristics. There is no test that loads and exercises the *built artifacts* from `pkg/` or `pkg-web/` in release mode.
- Fix: Consider adding a smoke test that imports and calls the built `pkg/mds_wasm.js` artifact after the build step, to verify the wasm-opt-processed binary is functional:
  ```yaml
  - name: Smoke test release build (nodejs)
    run: node -e "const m = require('./crates/mds-wasm/pkg/mds_wasm.js'); const r = m.compile('Hello!\n', null); console.log(r.output);"
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Binary size reporter silently swallows errors via `continue-on-error: true`** - `.github/workflows/ci.yml:75-76`
**Confidence**: 80%
- Problem: The binary size reporting step uses `if: always()` combined with `continue-on-error: true`. If the `.wasm` files don't exist (e.g., build failed), the `wc -c < "$f"` redirection will fail with a shell error, but `continue-on-error: true` suppresses it. This is acceptable for a reporting step, but it means a subtler issue -- like a renamed output file path -- would also be silently ignored, producing no size annotation without any visible warning. This is informational since the step was designed to be best-effort.
- Fix: Add a file existence check for clearer diagnostics:
  ```yaml
  run: |
    for f in crates/mds-wasm/pkg/mds_wasm_bg.wasm crates/mds-wasm/pkg-web/mds_wasm_bg.wasm; do
      if [ ! -f "$f" ]; then
        echo "::warning::WASM binary not found: $f"
        continue
      fi
      raw=$(wc -c < "$f" | tr -d ' ')
      gz=$(gzip -c "$f" | wc -c | tr -d ' ')
      label="${f#crates/mds-wasm/}"
      echo "::notice::WASM ${label}: ${raw} bytes raw, ${gz} bytes gzipped"
    done
  ```

### LOW

**No WASM binary size regression gate** - `.github/workflows/ci.yml:74-83`
**Confidence**: 80%
- Problem: The binary size reporting step emits `::notice::` annotations but does not fail the build if sizes exceed a threshold. Since the primary goal of re-enabling wasm-opt is size reduction, a regression in WASM binary size would go unnoticed until manual inspection. This is informational -- the step is a good first step, and a size gate can be added in a follow-up.
- Fix: Consider adding a threshold check in a follow-up PR:
  ```yaml
  if [ "$raw" -gt 500000 ]; then
    echo "::error::WASM binary $label exceeds 500KB threshold: ${raw} bytes"
    exit 1
  fi
  ```

## Suggestions (Lower Confidence)

- **Missing release-profile test in JS job** - `.github/workflows/ci.yml:114-115` (Confidence: 65%) -- The `js` job installs Binaryen but only builds WASM (no `--release` test like the `wasm` job has). If the JS job's WASM build uses release profile by default (wasm-pack build does), this is fine, but there is no explicit test of the release-profile binary in the JS pipeline. Applies ADR-005 (full CI across all OS targets).

- **No test asserting wasm-opt feature flags are complete** - `crates/mds-wasm/Cargo.toml:41` (Confidence: 62%) -- The explicit feature flags (`--enable-bulk-memory`, `--enable-sign-ext`, etc.) are manually maintained. If a future Rust/LLVM upgrade emits a new post-MVP feature, wasm-opt will reject it and CI will catch it at build time. However, there is no test that validates the feature flag list is complete relative to the Rust compiler's output. The current approach (CI catches it when it breaks) is pragmatic.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 1 | 1 |

**Testing Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This PR adds meaningful testing improvements:

1. **Release-profile WASM test** (`wasm-pack test --node --release`) -- This is the key addition. It exercises wasm-opt during test compilation, catching flag mismatches and Binaryen compatibility issues. The iterative commit history (4 fix commits for wasm-opt flags) demonstrates this test caught real issues.

2. **wasm-opt verification step** -- Confirms Binaryen is installed before builds run, providing clear diagnostics on setup failure.

3. **Binary size reporting** -- Good observability addition for tracking the size impact of wasm-opt.

The one condition for approval: consider whether the release-profile test truly validates the *shipping artifact* or only a test harness binary (see Should-Fix item above). The existing test coverage (497 lines in `web.rs`, ~30 test functions covering compile, check, scan_imports, error paths, and options validation) is solid and runs against both debug and release profiles.

Applies ADR-005: the CI changes span all relevant jobs (wasm, js across 3 OS targets, release pipeline), ensuring Binaryen availability and wasm-opt compatibility are verified across the full CI matrix.

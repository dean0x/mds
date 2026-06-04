# Code Review Summary

**Branch**: chore/68-wasm-opt-enable -> main
**Date**: 2026-06-03_1833

## Merge Recommendation: CHANGES_REQUESTED

This PR re-enables wasm-opt optimization for WASM binaries with explicit post-MVP feature flags, pins build tooling versions (wasm-pack, Binaryen), and adds CI validation via release-profile testing and binary size reporting. The change is well-scoped and architecturally sound, but two MEDIUM-severity issues in code you touched should be resolved before merge:

1. **Duplicated CI toolchain setup** across 3 jobs (architecture concern, DRY violation)
2. **Missing file guard in binary size reporter** (reliability concern when builds fail)

Both are straightforward to fix and will improve code maintainability and CI robustness.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 0 | 1 | 0 | 1 |
| Should Fix | 0 | 0 | 1 | 0 | 1 |
| Pre-existing | 0 | 0 | 2 | 1 | 3 |

---

## Blocking Issues

**Duplicated CI toolchain setup block** — `.github/workflows/ci.yml:58-63`, `ci.yml:108-113`, `release.yml:254-259` (Architecture, 82% confidence)

The wasm-pack + Binaryen setup sequence (action refs, version pins, SHA hashes) is copy-pasted identically into three separate workflow jobs (wasm, js, and publish-npm). This creates content coupling at the CI layer — when versions need updating, all three locations must change in lockstep. A drift between any two (e.g., one job on Binaryen v130, another on v129) would produce silent build inconsistencies.

**Fix**: Extract the setup into a GitHub Actions composite action (`.github/actions/setup-wasm/action.yml`):
```yaml
name: Setup WASM toolchain
runs:
  using: composite
  steps:
    - uses: jetli/wasm-pack-action@v0.4.0
      with:
        version: v0.14.0
    - uses: phi-ag/setup-binaryen@f7f99985a69ad20f08b12ad725865b14a7f875a4
      with:
        version: 129
```

Then each job simply does `- uses: ./.github/actions/setup-wasm`. This is a best practice for GitHub Actions (DRY principle) and applies ADR-005 (full CI validation for build tooling changes). Version updates become single-point changes.

---

## Should-Fix Issues

**1. Release-profile test does not exercise the shipped build artifact** — `.github/workflows/ci.yml:72-73` (Testing, 82% confidence)

The new `wasm-pack test --node --release` step runs tests against a release build that exercises wasm-opt. However, the preceding `wasm-pack build` step (line 67-69) already produces optimized artifacts into `pkg/`. The test step rebuilds from scratch with its own test harness, so it validates that wasm-opt runs successfully *on a test binary*, but not that the *shipped artifacts* in `pkg/` and `pkg-web/` are functional after optimization.

**Fix**: Add a smoke test that imports and exercises the built artifacts:
```yaml
- name: Smoke test release build (nodejs)
  run: node -e "const m = require('./crates/mds-wasm/pkg/mds_wasm.js'); const r = m.compile('Hello!\n', null); console.log(r.output);"
```

This validates the wasm-opt-processed binary is loadable and callable, not just that it compiles.

---

**2. Binary size reporter silently fails when WASM files are missing** — `.github/workflows/ci.yml:78-83` (Reliability, 82% confidence)

The "Report WASM binary sizes" step runs with `if: always()` and `continue-on-error: true`, meaning it executes even when previous build/test steps fail. When `.wasm` files don't exist, `wc -c < "$f"` fails silently and the step emits no annotation, producing misleading CI output or no output at all without indicating the data is invalid.

**Fix**: Add a file existence check inside the loop:
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

This ensures CI annotations are always accurate, even when builds fail.

---

## Suggestions (Lower Confidence)

| Issue | Location | Confidence | Category | Note |
|-------|----------|------------|----------|------|
| Action pinning style inconsistency | `ci.yml:58`, `ci.yml:61` (and others) | 82% | Consistency | `phi-ag/setup-binaryen` is SHA-pinned while adjacent `jetli/wasm-pack-action` uses tag-only pinning. Either SHA-pin both or tag-pin both for visual consistency. |
| Missing `wasm-opt --version` in JS job | `ci.yml:111-113` | 70% | Reliability | The `wasm` job verifies wasm-opt installation but the `js` job does not. Add verification step for symmetry. |
| Missing `wasm-opt --version` in release workflow | `release.yml:257-259` | 65% | Reliability | The publish-npm job installs Binaryen without verification. Silent failures would produce cryptic build errors. |
| Binary size reporter file existence | `ci.yml:74-83` | 60% | Reliability | Step runs `if: always()` but doesn't check if files exist before measuring. (Fix provided above.) |
| Verify step only in `wasm` job | `ci.yml:64-65` vs `ci.yml:111-113` | 65% | Consistency | Minor asymmetry; JS job would fail at build step if wasm-opt missing, so this is defensive but not critical. |
| CLAUDE.md gotcha less detailed than RELEASING.md | `CLAUDE.md:37` vs `RELEASING.md:137-139` | 62% | Consistency | CLAUDE.md doesn't mention Binaryen v129, while RELEASING.md does. Consider aligning specificity. |
| Missing release-profile test in JS job | `ci.yml:114-115` | 65% | Testing | JS job builds WASM but doesn't run release-profile tests like the `wasm` job does. (JS job runs on all 3 OS targets, so this amplifies the concern.) |
| No size regression gate | `ci.yml:74-83` | 60% | Performance | Binary size reporting emits `::notice::` but doesn't fail build if sizes exceed a threshold. Future improvement. |
| Consider `--enable-reference-types` for future upgrades | `crates/mds-wasm/Cargo.toml:41` | 65% | Rust | Current flags (bulk-memory, sign-ext, nontrapping-float-to-int, mutable-globals) are precise for Rust 1.88 / LLVM 20, but future upgrades may emit reference-types. Explicit-flags approach requires maintenance. Well-documented; not blocking. |
| Duplicate WASM compilation in JS job | `ci.yml:111-115` | 70% | Performance | WASM is built on all 3 OS matrix targets (ubuntu, macos, windows), but WASM compilation is platform-independent. Could be cached or artifact-shared from the `wasm` job. (Future optimization; acceptable now.) |
| Missing wasm-opt test assertion | `crates/mds-wasm/Cargo.toml:41` | 62% | Testing | No test validates that the feature flag list is complete relative to Rust's output. Current approach (CI catches it when it breaks) is pragmatic. |

---

## Pre-existing Issues (Not Blocking)

| Issue | Location | Severity | Note |
|-------|----------|----------|------|
| Inconsistent action pinning strategy | `ci.yml:58`, `ci.yml:108`, `release.yml:254` | MEDIUM | `jetli/wasm-pack-action` uses tag pinning (`@v0.4.0`) while `phi-ag/setup-binaryen` is SHA-pinned. Not introduced by this PR, but visible in adjacent lines. Should be resolved across the board for supply-chain consistency. |
| Binary size reporter swallows errors | `.github/workflows/ci.yml:75-76` | MEDIUM | Using `continue-on-error: true` suppresses shell errors on missing files. This is acceptable for informational steps, but could mask a subtle issue (e.g., renamed file path). Fix suggested above. |
| No size regression gate | `.github/workflows/ci.yml:74-83` | LOW | Reporting step doesn't fail on size overages. Future improvement, not blocking. |

---

## Positive Observations

- **Security**: SHA-pinned third-party action (`phi-ag/setup-binaryen`) with version comment is gold-standard practice. wasm-pack pinned to `v0.14.0` (away from `latest`) eliminates build non-determinism. No secrets or injection surfaces introduced. Applies ADR-005.
- **Architecture**: Well-scoped change; cleanly separates build-time optimization config (Cargo.toml) from CI tooling (workflow files). Documentation (CLAUDE.md, RELEASING.md) updated to reflect new dependency. wasm-opt feature flags are correctly explicit, not using a blanket (non-existent) `--enable-all`.
- **Performance**: Re-enables `wasm-opt -Oz` with correct post-MVP feature flags (bulk-memory, sign-ext, nontrapping-float-to-int, mutable-globals). These are the exact features Rust 1.88+ emits and are universally supported. Estimated 10-20% WASM size reduction. Workspace already has optimal release-profile settings (`opt-level = "z"`, `strip = true`, `codegen-units = 1`, `lto = true`); this PR completes the optimization chain.
- **Regression**: Clean, internally consistent change. Migration from `wasm-opt = false` to the new config is complete with no stale references. Release-profile test validates wasm-opt-processed binaries work correctly. No API changes, no breaking changes.
- **Testing**: Release-profile test step is the key addition—exercises wasm-opt during test compilation, catching flag mismatches and Binaryen compatibility issues. Commit history shows this test caught real issues during development. Existing test coverage (497 lines in web.rs, ~30 test functions) is solid.
- **Rust**: Zero Rust source file changes; only build metadata (`crates/mds-wasm/Cargo.toml`). Flag selection is precise for Rust 1.88 / LLVM 20 output. No `.unwrap()`, `panic!`, or unsafe blocks introduced. Comment quality is good, explaining *why* each flag exists.

---

## Convergence Status

**Cycle**: 1
**Prior Resolution**: none
**Prior FP Ratio**: N/A (first cycle)
**Assessment**: First cycle — new branch under review.

All 9 reviewers (security, architecture, performance, complexity, consistency, regression, testing, reliability, rust) have completed their analysis. The findings are well-distributed across domains:

- **Architecture** and **Consistency** reviewers flag the same duplicated setup issue (boosted to 82% confidence)
- **Testing** and **Reliability** reviewers flag the missing file guard issue (boosted to 82% confidence)
- **Reliability** reviewer flags missing wasm-opt verification in JS and release jobs (70%, 65%)
- Multiple reviewers note the asymmetry in CI verification steps across jobs (applies ADR-005 full CI validation principle)
- No conflicting findings across reviewers—consistency of findings across domains adds confidence

---

## Action Plan

1. **CRITICAL**: Extract wasm-pack + Binaryen setup into `.github/actions/setup-wasm/action.yml` composite action and use in all three workflow jobs. This eliminates the DRY violation and single-point-of-failure for version management.

2. **CRITICAL**: Add file existence check to binary size reporter step (ci.yml lines 78-83). This ensures CI annotations are accurate even on failed builds.

3. **RECOMMENDED**: Add smoke test that imports and exercises the built `pkg/mds_wasm.js` artifact to validate the wasm-opt-processed binary is functional in the shipped context.

4. **RECOMMENDED**: Add `wasm-opt --version` verification steps to the JS job (after Binaryen install) and release workflow (publish-npm job) for consistency and diagnostics.

5. **FOLLOW-UP (lower priority)**: Consider SHA-pinning `jetli/wasm-pack-action` to match the security posture of `phi-ag/setup-binaryen`. Update documentation in CLAUDE.md to mention Binaryen v129.

---

## Summary

This is a well-structured, focused change that correctly re-enables WASM optimization with explicit feature flags and adds CI validation. The two MEDIUM-severity issues (duplicated setup block and missing file guard) are straightforward to fix and will improve maintainability and robustness. Once fixed, this PR is a solid improvement to the WASM build pipeline with no breaking changes and no blocking security, regression, or performance concerns.

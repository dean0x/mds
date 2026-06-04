# Performance Review Report

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

(none)

## Suggestions (Lower Confidence)

- **Binary size reporting measures debug-mode output for `wasm-pack test --node`** - `ci.yml:71` (Confidence: 65%) — The `wasm-pack test --node` step (line 71, no `--release`) builds in dev mode. This is not an issue for the size report (which reads from `pkg/` and `pkg-web/` produced by the release-profile `wasm-pack build` on lines 68-69), but the dev-mode test build does add a full WASM compilation cycle that does not benefit from Rust cache sharing with the release build. Consider whether this test can be dropped in favor of the release-mode test on line 73, which validates both correctness and wasm-opt.

- **Duplicate WASM compilation in JS job across 3 OS targets** - `ci.yml:111-115` (Confidence: 70%) — The JS job installs Binaryen and builds WASM on all 3 OS matrix targets (ubuntu, macos, windows). Since WASM compilation is platform-independent (the output is the same `.wasm` binary regardless of host OS), the WASM build could theoretically be cached or shared as an artifact from the dedicated `wasm` job rather than rebuilt 3 times. This would save approximately 2 redundant WASM compilations worth of CI minutes. However, the current structure is simpler and the cost may be acceptable for this project's scale. (applies ADR-005 — full CI across all 3 OS targets is required, but the WASM binary itself is host-independent)

- **No size regression gate** - `ci.yml:74-83` (Confidence: 60%) — The binary size reporting step emits `::notice::` annotations but does not fail the build if sizes exceed a threshold. For a WASM library shipped to npm, establishing a size budget (e.g., fail if gzipped exceeds N KB) would prevent accidental size regressions. This is a future improvement, not blocking.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 9/10
**Recommendation**: APPROVED

### Rationale

This PR is a net positive for performance. It re-enables `wasm-opt -Oz` with the correct post-MVP feature flags, which recovers an estimated 10-20% reduction in shipped WASM binary size. The workspace `Cargo.toml` already has optimal release-profile settings for the WASM target (`opt-level = "z"`, `strip = true`, `codegen-units = 1`, `lto = true`), and this PR completes the optimization chain by adding Binaryen's wasm-opt as the final pass.

Key performance observations:
1. **wasm-opt `-Oz` is the correct flag** for size-optimized WASM, matching the Cargo `opt-level = "z"` in the workspace profile.
2. **Post-MVP feature flags** (`--enable-bulk-memory`, `--enable-sign-ext`, `--enable-nontrapping-float-to-int`, `--enable-mutable-globals`) are the exact set emitted by Rust 1.88+/LLVM 20 for `wasm32-unknown-unknown`. These are universally supported in modern runtimes and enable wasm-opt to process the binary without rejecting valid instructions.
3. **Binary size reporting** via `::notice::` annotations provides visibility into both raw and gzipped sizes, enabling manual regression detection.
4. **CI time impact** is moderate: Binaryen installation is fast (pre-built binary download), and the added release-mode test step is justified as the primary verification that wasm-opt produces a valid binary.
5. **wasm-pack pinned to v0.14.0** prevents unexpected behavior from version drift, which is good practice for build reproducibility.

No blocking performance issues found. The three suggestions are all below the 80% confidence threshold and relate to CI optimization opportunities, not shipped artifact performance.

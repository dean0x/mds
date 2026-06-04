# Rust Review Report

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

- **Consider `--enable-reference-types` for future Rust toolchain upgrades** - `crates/mds-wasm/Cargo.toml:41` (Confidence: 65%) -- Rust nightly / future stable versions may emit reference-types instructions (e.g., `externref`). The current set of four `--enable-*` flags covers Rust 1.88 / LLVM 20 emissions precisely, but if the MSRV bumps or the target changes, a new flag may be needed. The explicit-flags approach (vs a blanket `--enable-all`) is correct for Binaryen v129 but requires maintenance when the Rust WASM backend evolves. The comment on line 38-40 documents this well.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Rust Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

### What was reviewed

This PR has **zero Rust source file changes** (no `.rs` diffs). The Rust-specific scope is limited to `crates/mds-wasm/Cargo.toml` build metadata, specifically the `[package.metadata.wasm-pack.profile.release]` section that configures `wasm-opt` flags.

### Cargo.toml change assessment

The change at `crates/mds-wasm/Cargo.toml:37-41` replaces `wasm-opt = false` with:
```toml
wasm-opt = ["-Oz", "--enable-bulk-memory", "--enable-sign-ext", "--enable-nontrapping-float-to-int", "--enable-mutable-globals"]
```

This is correct and well-documented:

1. **Flag selection is precise**: The four `--enable-*` flags match exactly the post-MVP WASM features that Rust 1.88 / LLVM 20 emits for `wasm32-unknown-unknown`: bulk-memory, sign-ext, nontrapping-float-to-int, and mutable-globals. No unnecessary features are enabled.

2. **`-Oz` is appropriate**: Size optimization is the right choice for a WASM library distributed via npm where download size matters.

3. **Comment quality is good**: Lines 38-40 explain *why* the flags exist (Rust 1.88+ post-MVP features), *what* is needed (Binaryen), and *where* to get it (CI action vs local install).

4. **No `unsafe` blocks introduced**: No Rust source code was modified.

5. **No `.unwrap()` or panic paths added**: No runtime code changes.

6. **Dependency versions unchanged**: The `Cargo.toml` dependency section is untouched.

### Decisions context

- Applies **ADR-005** (Branch + full CI + examples gate before merge): The CI changes add release-profile WASM testing and binary size reporting, which strengthens the validation gate for build tooling changes. The PR adds Binaryen as a new CI dependency, exactly the kind of change ADR-005 mandates full CI validation for.
- Applies **ADR-003** (Automated release via workflow_dispatch): The `release.yml` changes are consistent with the existing automated release pipeline, adding Binaryen installation in the same pattern as the CI workflow.

### Why no blocking issues

- No Rust source code was changed -- only build metadata
- The `wasm-opt` configuration is a `[package.metadata]` section, which is opaque to `cargo` itself and only consumed by `wasm-pack`
- The flags are individually correct for Binaryen v129 and the Rust 1.88 WASM output
- The change is protected by a new release-profile test step in CI that exercises the exact `wasm-opt` pipeline

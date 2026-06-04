# Performance Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

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

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

This PR converts a single-crate Rust project into a Cargo workspace with two members (`mds-core` library + `mds-cli` binary). The changes are purely structural with no performance-relevant behavioral modifications:

1. **No algorithmic changes** -- All compiler logic (lexer, parser, evaluator, resolver, validator) moved via git rename with zero modifications. The N+1-free module caching (`Arc<ResolvedModule>`), bounded iteration limits (MAX_LOOP_ITERATIONS=100k, MAX_TOTAL_ITERATIONS=1M), and depth enforcement (MAX_VALUE_DEPTH=64) are unchanged.

2. **Dependency improvement** -- The `miette` crate's `"fancy"` feature (terminal rendering) is now only pulled by the CLI binary, not the library. Library consumers (`mds-core`) get plain `miette` without the heavier display dependencies. This reduces compile time and binary size for library-only users.

3. **Workspace resolver = "2"** -- Using Cargo resolver v2 enables proper feature unification (features activated by dev-dependencies don't leak into normal builds), which can slightly reduce final binary size and compile times.

4. **Test structure** -- Integration tests that call `mds::compile()` directly (language, objects, imports, frontmatter) avoid subprocess overhead. Only CLI-behavior tests (~51 tests) spawn the binary via `mds_bin()`, which is the correct split for test performance.

5. **No new allocations, no new I/O paths** -- The `BuildArgs` struct consolidation in `main.rs` is a zero-cost refactoring at runtime (struct is consumed immediately, no additional copies).

No performance regressions detected. The workspace split is performance-neutral for the binary and mildly positive for library consumers.

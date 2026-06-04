# Reliability Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

No blocking reliability issues found.

## Issues in Code You Touched (Should Fix)

No should-fix reliability issues found.

## Pre-existing Issues (Not Blocking)

No critical pre-existing reliability issues identified.

## Suggestions (Lower Confidence)

- **Unbounded `read_dir` collection in `auto_detect_mds_file`** - `crates/mds-cli/src/main.rs:197-204` (Confidence: 65%) -- The `filter_map(...).collect()` on `read_dir` is unbounded, meaning a directory with an extremely large number of `.mds` files would collect all of them before slicing. In practice, this is user-invoked on a working directory (not attacker-controlled) and the operation is fast (filename extension check only), so risk is very low. Consider adding a `.take(1000)` or early-exit if the goal is only detecting 0, 1, or "multiple" files.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

This PR is a structural refactoring (workspace split) with no behavioral changes to resource limits or error handling. The reliability posture is strong:

1. **Bounded Iteration** -- All existing bounds are preserved intact:
   - `MAX_TRAVERSAL_DEPTH` (256) bounds the config file walk in `load_config` (line 51)
   - `MAX_FILE_SIZE` (10 MB) bounds stdin reading via `.take(MAX_STDIN_SIZE + 1)` (line 419)
   - `MAX_CONFIG_SIZE` (1 MB) bounds config file parsing (line 59)
   - All evaluator limits (MAX_LOOP_ITERATIONS=100k, MAX_TOTAL_ITERATIONS=1M, MAX_CALL_DEPTH=128, MAX_OUTPUT_SIZE=50MB, MAX_WARNINGS=1000) remain unchanged in the core crate

2. **Assertion Density** -- The `pub const` re-exports of `MAX_FILE_SIZE` and `MAX_TRAVERSAL_DEPTH` in `lib.rs` are validated by `api_surface.rs` tests (lines 172-176, 194-199), catching accidental removal of bounds

3. **Allocation Discipline** -- `clean_output` pre-allocates with `String::with_capacity(s.len())` (lib.rs:387). No new allocations in hot loops introduced

4. **Indirection Limits** -- Module visibility was tightened (`pub mod` to `pub(crate) mod`), reducing indirection surface without changing data flow

5. **Metaprogramming Restraint** -- No new generics, macros, or reflection added. The workspace Cargo.toml uses standard workspace inheritance patterns

The `BuildArgs` struct grouping is a net reliability improvement -- it makes the parameter contract explicit and prevents argument ordering bugs in `run_build`.

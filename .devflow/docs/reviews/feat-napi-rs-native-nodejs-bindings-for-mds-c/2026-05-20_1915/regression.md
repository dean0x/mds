# Regression Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20T19:15

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**MSRV bump from 1.80 to 1.88 is larger than necessary** - `Cargo.toml:8`
**Confidence**: 82%
- Problem: The workspace `rust-version` was bumped from 1.80 to 1.88 across all four crates (mds-core, mds-cli, mds-wasm, mds-napi). The only language feature used from a newer Rust is `Option::is_none_or` in `parser.rs`, which was stabilized in Rust 1.82. The jump to 1.88 is 6 minor versions beyond what `is_none_or` requires. While the PR description attributes this to napi-rs requirements, napi-rs 3.9.0 does not document a 1.88 MSRV. This unnecessarily constrains downstream consumers of `mds-core` and `mds-cli` who may be on Rust 1.82-1.87.
- Fix: Verify the actual minimum Rust version required by the napi-rs dependency chain. If 1.82 suffices, use that instead. If a higher version is genuinely needed by napi-rs, document the reason in the Cargo.toml comment:
  ```toml
  # rust-version 1.88 required by: is_none_or (1.82) + napi-rs 3.9.0 (1.XX)
  rust-version = "1.88"
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Regression Checklist

- [x] No exports removed without deprecation -- no exports were removed from any existing crate
- [x] Return types backward compatible -- the only change (`map_or` -> `is_none_or`) is semantically identical
- [x] Default values unchanged -- no default values were modified
- [x] Side effects preserved -- no event handlers, logging, or emit calls changed
- [x] All consumers of changed code updated -- `parse_interpolation_expr` is internal (`fn`, not `pub fn`); no external consumers
- [x] Migration complete across codebase -- only one `map_or(true, ..)` instance existed and it was migrated
- [x] CLI options preserved -- mds-cli has zero changes
- [x] API endpoints preserved -- mds-wasm has zero changes
- [x] Commit messages match implementation -- all 4 commits accurately describe their changes
- [x] Breaking changes documented -- the MSRV bump is noted in the clippy lint commit message

## Verification

All existing tests pass with zero failures:
- `mds-core`: 282 pass, 0 fail
- `mds-cli`: 218 pass, 0 fail
- `mds-wasm`: 0 tests (unchanged)

The `parser.rs` change from `paren_opt.map_or(true, |p| dot_pos < p)` to `paren_opt.is_none_or(|p| dot_pos < p)` is a **semantically identical** refactor:
- Both return `true` when `paren_opt` is `None`
- Both return `dot_pos < p` when `paren_opt` is `Some(p)`

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The single condition: verify the MSRV bump to 1.88 is justified by the napi-rs dependency chain (not just by `is_none_or` which only needs 1.82). If 1.88 is indeed required by napi-rs, add a brief comment documenting this. If not, lower to the minimum version that satisfies all dependencies.

No regression risk was found in existing functionality. The parser change is provably equivalent, all existing tests pass, no exports were removed, no return types changed, and no files were deleted. The new mds-napi crate is purely additive.

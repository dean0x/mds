# Regression Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01
**PR**: #52

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Visibility narrowing of `MAX_ELSEIF_BRANCHES` from `pub` to `pub(crate)`** - `crates/mds-core/src/limits.rs:18`
**Confidence**: 82%
- Problem: `MAX_ELSEIF_BRANCHES` was previously `pub const` in `ast.rs` (part of the crate's public API surface). It is now `pub(crate) const` in `limits.rs`. Any downstream crate or external consumer importing `mds_core::ast::MAX_ELSEIF_BRANCHES` would break at compile time.
- Impact: LOW in practice -- grep confirms no external crate (`mds-cli`, `mds-wasm`, `mds-napi`) imports this constant, and the project is pre-1.0 with zero external consumers. However, this is a technically breaking public API change that should be documented.
- Fix: Either add a `pub const MAX_ELSEIF_BRANCHES` re-export in `lib.rs` (mirroring the pattern used for `MAX_FILE_SIZE` and `MAX_TRAVERSAL_DEPTH`), or explicitly note the removal in the changelog as an intentional API surface reduction. Given this is pre-release with no consumers, documenting it is sufficient.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Verification Performed

1. **All 48 parser tests preserved**: Original `parser.rs` had 48 `#[test]` functions; `parser_tests.rs` has exactly 48. No test was dropped.
2. **All 21 helper functions preserved**: Every function from the original `parser.rs` appears in `parser_helpers.rs` with matching signatures (except the intentional `_offset` removal from `parse_export_directive`).
3. **Full test suite passes**: `cargo test --workspace` reports 591 pass, 0 fail, 0 skip.
4. **Clippy clean**: `cargo clippy --workspace --all-targets -- -D warnings` produces 0 warnings.
5. **No stale import paths**: No references to `resolver::MAX_FILE_SIZE`, `resolver::MAX_TRAVERSAL_DEPTH`, `ast::MAX_ELSEIF_BRANCHES`, or `parser::MAX_NESTING_DEPTH` remain.
6. **Re-exports intact**: `lib.rs` re-exports `MAX_FILE_SIZE` and `MAX_TRAVERSAL_DEPTH` via `limits::*` (previously via `resolver::*`). External crate consumers (`mds-cli`, `mds-wasm`, `mds-napi`) all compile and resolve correctly.
7. **`is_valid_identifier` visibility preserved**: `pub(crate)` in original `parser.rs`, re-exported as `pub(crate)` from the `parser` module via `pub(crate) use helpers::is_valid_identifier`.
8. **`parse_export_directive` signature change safe**: The removed `_offset: usize` parameter was unused (underscore-prefixed). The only call site is within `parser.rs:197`, which was updated to match. No external callers exist.
9. **Constant values unchanged**: All five consolidated constants retain their original values (verified by pinning test in `limits.rs`): `MAX_DOT_SEGMENTS=32`, `MAX_NESTING_DEPTH=64`, `MAX_ELSEIF_BRANCHES=256`, `MAX_FILE_SIZE=10MB`, `MAX_TRAVERSAL_DEPTH=256`.
10. **SECURITY.md location references updated**: The resource limits table correctly points to `limits.rs` for `MAX_FILE_SIZE` and `MAX_NESTING_DEPTH` (previously `resolver.rs` and `parser.rs`).

## Decisions Compliance

- **applies ADR-001**: This refactoring PR should pass lint/format gates before merge (verified: clippy and tests pass).
- **applies ADR-002**: PR claims to close #35 (consolidate constants) and #36 (split parser). Verified: constants are consolidated into `limits.rs` (addresses #35), and `parser.rs` is split into 3 files totaling ~1824 lines from the original ~1820 (addresses #36). Both issues are substantively addressed by the diff.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The single MEDIUM finding (visibility narrowing of `MAX_ELSEIF_BRANCHES`) is a technical public API reduction with no practical impact given the pre-1.0 status and zero external consumers. Condition: document the change in the changelog or add a `lib.rs` re-export if public API preservation is desired. This is a clean, well-executed structural refactoring with zero behavioral changes and full test coverage preservation.

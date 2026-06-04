# Reliability Review Report

**Branch**: feat-workspace-split -> main
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

- **Missing `[workspace.dependencies]` for shared dependency deduplication** - `Cargo.toml` (Confidence: 65%) -- Both `mds-core` and `mds-cli` independently specify `serde = "1"`, `serde_json = "1"`, and `miette = "7"`. Using `[workspace.dependencies]` in the root `Cargo.toml` would provide a single source of truth for shared dependency versions, preventing version drift as the workspace grows. Not a reliability risk today with 2 crates and a lockfile, but becomes one as more crates are added.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED

### Rationale

This is a clean structural refactor (single crate to Cargo workspace) with zero behavioral changes. All reliability properties from the pre-split codebase are preserved:

1. **Bounded iteration** -- The `load_config` directory traversal loop remains bounded by `MAX_TRAVERSAL_DEPTH` (256), imported from `mds-core`. All other loops iterate finite, CLI-provided data.

2. **Resource limits** -- `MAX_CONFIG_SIZE` (1 MB) for `mds.json`, `MAX_STDIN_SIZE` (10 MB, aliased from `MAX_FILE_SIZE`) for stdin input, and `MAX_TRAVERSAL_DEPTH` (256) for directory walking are all preserved unchanged. The consolidation of these imports into a single `use` statement at line 9 of `main.rs` is a clarity improvement.

3. **Assertion density** -- Pre-existing precondition checks (path traversal rejection, directory rejection, size guards) are all intact. No assertions were removed or weakened.

4. **Allocation discipline** -- No new allocations introduced. The only code changes are import consolidation and `Result` type alias unification (`miette::Result<T>` instead of `std::result::Result<T, miette::Error>`), which are purely cosmetic.

5. **Indirection limits** -- No change in indirection depth. The `mds-cli` depends on `mds-core` via path dependency, a single level of indirection.

6. **Test coverage** -- The one test change (`not_mds_file_error`) replaces a dependency on `spec.md` at the workspace root with a self-contained fixture file (`not_mds.md`), making the test more reliable and portable. All 354 tests pass.

# Testing Review Report

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

### MEDIUM

**Integration test for `not_mds_file_error` uses weak assertion** - `crates/mds-cli/tests/integration.rs:181`
**Confidence**: 82%
- Problem: The assertion `assert!(err.contains("not an MDS file") || err.contains("not_mds"))` uses an OR pattern with a very broad match on `"not_mds"` (which is part of the fixture filename, not the error message). This means the test would pass even if the error message changed entirely, as long as the filename appears in the output. This is a pre-existing pattern -- the disjunction existed before this PR (the old version tested against `spec.md` so it would match `"spec"` in the path).
- Fix: Tighten to assert on the specific error message: `assert!(err.contains("not an MDS file"))` and remove the overly permissive alternative.

**All 205 integration tests live in a single 3,617-line file** - `crates/mds-cli/tests/integration.rs`
**Confidence**: 80%
- Problem: The monolithic test file makes it harder to locate related tests and adds cognitive load. Test groups (lexer, parser, imports, CLI, resource limits, etc.) could benefit from being split into separate test files under a `tests/` directory. This is a pre-existing organizational concern not introduced by this PR.
- Fix: Consider splitting into `tests/compile.rs`, `tests/cli.rs`, `tests/errors.rs`, `tests/imports.rs`, etc. in a future PR.

## Suggestions (Lower Confidence)

- **No workspace-level test for cross-crate API surface** - (Confidence: 70%) -- The workspace split cleanly separates `mds-core` (library, 136 unit tests) from `mds-cli` (binary, 218 integration tests). All integration tests use the library API (`mds::compile`, `mds::check`, etc.) and exercise the CLI binary via `mds_bin()`. There are no tests that explicitly verify the library's public API surface or re-export correctness (e.g., that `MAX_TRAVERSAL_DEPTH` is accessible from the CLI crate). Currently this is implicitly tested because `main.rs` imports and uses these symbols, but an explicit API surface test could catch accidental visibility regressions during future refactoring.

- **New `not_mds.md` fixture duplicates test intent with `type_mds_md_file.md`** - (Confidence: 65%) -- The new `not_mds.md` fixture (plain markdown without `type: mds`) was added because `spec.md` no longer exists relative to the CLI crate's `CARGO_MANIFEST_DIR`. The existing `type_mds_md_file.md` fixture already tests `.md` files *with* `type: mds`. Both fixtures test the `.md` file type boundary. This is reasonable -- one tests the positive case and the other the negative -- but worth noting to avoid fixture proliferation.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 2 | 0 |

**Testing Score**: 9/10
**Recommendation**: APPROVED

### Rationale

This PR is a pure structural refactoring (single crate to Cargo workspace) with zero behavioral changes. From a testing perspective:

1. **All 354 tests pass** (136 in mds-core, 218 in mds-cli) across both crates.
2. **Test migration is clean**: All test files and fixtures were relocated correctly. The integration test file has only one meaningful code change (line 177: fixture path updated from `spec.md` to `not_mds.md`) along with a new 5-line fixture file.
3. **No test coverage was lost**: The 99% similarity index on `integration.rs` and 100% renames on all core source files confirm no test logic was altered.
4. **Fixture co-location is correct**: All 86 test fixtures now live under `crates/mds-cli/tests/fixtures/`, co-located with the integration test that uses them. The old `tests/` directory at root was fully removed.
5. **The `not_mds.md` fixture addition is appropriate**: It replaces a dependency on `spec.md` (which sits at the workspace root and is not part of the CLI crate's manifest directory), making the test self-contained within the crate.
6. **No test infrastructure concerns**: `CARGO_MANIFEST_DIR` and `CARGO_BIN_EXE_mds` macros resolve correctly in the workspace context. The `mds_bin()` helper and `fixture()` helper both use the correct crate-relative paths.

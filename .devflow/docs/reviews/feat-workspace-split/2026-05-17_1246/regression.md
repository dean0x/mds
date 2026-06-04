# Regression Review Report

**Branch**: feat-workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**KNOWLEDGE.md file paths reference non-existent `src/` and `tests/` directories** - `.features/mds-compiler/KNOWLEDGE.md:6-19, 620-644`
**Confidence**: 95%
- Problem: The KNOWLEDGE.md frontmatter and body reference paths like `src/lib.rs`, `src/main.rs`, `tests/integration.rs`, and directories `[src/, tests/]`. After the workspace split, these files moved to `crates/mds-core/src/` and `crates/mds-cli/src/` and `crates/mds-cli/tests/`. The old `src/` and `tests/` directories no longer exist at the repository root. This file was modified in this PR (description and keywords were updated, `src/limits.rs` was added to referencedFiles), but the path references were not updated to reflect the workspace restructure.
- Impact: Any tooling or developer workflow that uses KNOWLEDGE.md file paths to navigate or index the codebase will fail to locate the referenced files. The `directories` and `referencedFiles` frontmatter fields are stale.
- Fix: Update the frontmatter to reflect the new workspace structure:
  ```yaml
  directories: [crates/mds-core/src/, crates/mds-cli/src/, crates/mds-cli/tests/]
  referencedFiles:
    - crates/mds-core/src/lib.rs
    - crates/mds-core/src/ast.rs
    - crates/mds-core/src/lexer.rs
    - crates/mds-core/src/parser.rs
    - crates/mds-core/src/validator.rs
    - crates/mds-core/src/resolver.rs
    - crates/mds-core/src/evaluator.rs
    - crates/mds-core/src/scope.rs
    - crates/mds-core/src/value.rs
    - crates/mds-core/src/error.rs
    - crates/mds-core/src/limits.rs
    - crates/mds-cli/src/main.rs
    - crates/mds-cli/tests/integration.rs
  ```
  Similarly update all `src/` references in the Key Files and Related sections to use the correct `crates/mds-core/src/` or `crates/mds-cli/src/` prefixes.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Condition
Update KNOWLEDGE.md file path references to reflect the new workspace structure before merge.

### Regression Checklist

- [x] No exports removed without deprecation -- public API surface (`pub fn`, `pub const`, `pub use`) is identical before and after
- [x] Return types backward compatible -- all function signatures preserved exactly
- [x] Default values unchanged -- no behavioral changes to any defaults
- [x] Side effects preserved -- warning emission, stderr output, exit codes all unchanged
- [x] All consumers of changed code updated -- CLI binary correctly depends on `mds-core` via `mds = { package = "mds-core" }` with `[lib] name = "mds"`, so `use mds::*` works identically
- [x] Migration complete across codebase -- all `mds::` references resolve to the same library API
- [x] CLI options preserved -- binary name remains `mds`, all subcommands/flags identical
- [x] CLI binary name preserved -- `[[bin]] name = "mds"` in mds-cli/Cargo.toml
- [x] Commit message matches implementation -- PR claims "zero behavioral changes" and "all 354 tests pass", which is verified
- [ ] Documentation updated -- KNOWLEDGE.md file paths are stale (see Should Fix above)
- [x] `miette` feature split is correct -- `fancy` (terminal coloring) moved to CLI only; library has base `miette` only, which is appropriate
- [x] Test fixture relocation is complete -- all `.mds` fixtures moved to `crates/mds-cli/tests/fixtures/`; new `not_mds.md` fixture replaces reliance on root `spec.md`
- [x] Integration test adapted correctly -- `CARGO_MANIFEST_DIR` now resolves to `crates/mds-cli`, and the `fixture()` helper and `mds_bin()` helper work correctly
- [x] All 354 tests pass

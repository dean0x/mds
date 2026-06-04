# Regression Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Module path `mds::error::MdsError` and `mds::value::Value` no longer accessible** - `crates/mds-core/src/lib.rs:40-49`
**Confidence**: 85%
- Problem: `pub mod error` and `pub mod value` were changed to `pub(crate) mod`. External consumers who imported via the module path (e.g., `use mds::error::MdsError`) will get a compile error. The types are still available via top-level re-exports (`mds::MdsError`, `mds::Value`).
- Impact: Any downstream code using `mds::error::*` or `mds::value::*` module paths will break. Since this is a pre-release project with zero users, risk is minimal.
- Fix: Intentionally documented in commit `bd011ed` with rationale. The API surface test (`api_surface.rs`) validates the supported import pattern. No fix needed given zero external consumers, but note this is technically a breaking change that would require a semver bump if published.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`cargo install --path .` no longer works** - `Cargo.toml:1-3`
**Confidence**: 82%
- Problem: The root `Cargo.toml` is now a workspace manifest, not a package. Running `cargo install --path .` (a common pattern in READMEs) will fail with "cannot install workspace". Users must now use `cargo install --path crates/mds-cli`.
- Impact: Anyone following a hypothetical README install instruction would hit an error. The binary name remains `mds` (correct), but the install path changed.
- Fix: If a README or documentation references `cargo install --path .`, update it to `cargo install --path crates/mds-cli`. Alternatively, consider documenting `cargo install --workspace` if targeting workspace-level installs.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Package name change from `mds` to `mds-cli`/`mds-core`** - `crates/mds-cli/Cargo.toml:2`, `crates/mds-core/Cargo.toml:2` (Confidence: 65%) — If this were published to crates.io, `cargo install mds` would no longer work (the package is now `mds-cli`). Not relevant for pre-release, but worth noting for future publishing strategy.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions
1. Acknowledge the `pub(crate) mod` change is an intentional API narrowing (already documented in commit message).
2. Ensure any install documentation is updated to reference `crates/mds-cli` path.

### Positive Findings

- All 205 integration tests preserved (exact count match verified)
- All 362 total tests pass (205 integration + 149 unit + 8 new API surface tests)
- Binary name remains `mds` (via `[[bin]] name = "mds"`)
- Library crate name remains `mds` (via `[lib] name = "mds"`)
- All public functions preserved with identical signatures: `compile`, `compile_file`, `compile_str`, `compile_str_with`, `compile_collecting_warnings`, `compile_str_collecting_warnings`, `check`, `check_str`, `check_str_with`, `check_collecting_warnings`, `check_str_collecting_warnings`, `load_vars_file`
- Public types (`MdsError`, `Value`) and constants (`MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`) preserved
- All fixture files renamed (100% similarity), zero content loss
- No dependencies added or removed (Cargo.lock confirms)
- CLI behavior unchanged (same commands, flags, exit codes)

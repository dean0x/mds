# Architecture Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**CLI `load_vars_file` shadows library function** - `crates/mds-cli/src/main.rs:380`
**Confidence**: 82%
- Problem: The CLI defines a local function `load_vars_file` (lines 380-385) that wraps `mds::load_vars_file` with an `Option<PathBuf>` signature and error remapping. While this is not a naming collision (Rust's module scoping prevents ambiguity), having `fn load_vars_file` in the CLI that delegates to `mds::load_vars_file` creates confusion for maintainers — both have the same name but different signatures and error types.
- Fix: Rename the CLI helper to clarify intent:
```rust
fn load_optional_vars_file(
    path: Option<PathBuf>,
) -> Result<Option<HashMap<String, mds::Value>>> {
    path.map(|p| mds::load_vars_file(&p).map_err(|e| miette::miette!("{e}")))
        .transpose()
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`miette` feature split may surprise downstream library consumers** - `Cargo.toml:19` / `crates/mds-cli/Cargo.toml:22` (Confidence: 65%) — The workspace-level `miette` dependency omits `features = ["fancy"]`, and only the CLI enables it. This is correct for keeping the library crate lean, but worth documenting explicitly in the library crate's README or Cargo.toml comment since `MdsError` implements `miette::Diagnostic` and downstream consumers may expect formatted output without realizing they need to opt-in to the `fancy` feature themselves.

- **`mds-cli` declares `serde` as a dependency but uses it only for config deserialization** - `crates/mds-cli/Cargo.toml:20` (Confidence: 62%) — The CLI only uses `serde::Deserialize` for `MdsConfig` and `BuildConfig` (2 structs). If the CLI grows, this is fine, but currently it pulls in the `derive` feature via workspace inheritance for minimal use. Not blocking — just a future simplification opportunity if config parsing is ever refactored.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

## Rationale

This workspace split is architecturally sound and well-executed:

1. **Clean separation of concerns** — The library crate (`mds-core`) exposes a pure compiler API with no I/O side-effects beyond file reading, while the CLI crate handles user interaction, argument parsing, config discovery, and output routing. Each crate has a single responsibility.

2. **Correct dependency direction** — Dependencies flow strictly inward: CLI depends on library, library depends on nothing project-internal. No circular dependencies.

3. **Information hiding** — Internal modules use `pub(crate)` visibility, with types re-exported at the crate root. External consumers cannot reach into `mds::error` or `mds::value` module paths directly.

4. **API stability guard** — The `api_surface.rs` test in `mds-core` catches accidental visibility regressions via compile-time assertions on all public items.

5. **Workspace-level dependency consolidation** — Shared dependencies are declared once in `[workspace.dependencies]`, ensuring version consistency across crates.

6. **Test placement follows Cargo conventions** — Integration tests that need `CARGO_BIN_EXE_mds` live in `mds-cli`, while library-level tests are in `mds-core`. The `common/mod.rs` helper avoids duplication.

The single MEDIUM finding (shadowed function name) is a readability concern, not a correctness issue. The overall architecture cleanly follows the Dependency Inversion Principle and maintains a well-defined port boundary between the library and CLI layers.

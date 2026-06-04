# Consistency Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Unused import generates compiler warning** - `crates/mds-cli/tests/objects.rs:2`
**Confidence**: 95%
- Problem: `use common::fixture;` is imported but never used. The file only uses `mds::compile_str` and inline `std::collections::HashMap` for its tests. This triggers a `warning: unused import: 'common::fixture'` in `cargo test`.
- Fix: Remove the unused import line.
```rust
mod common;
// Remove: use common::fixture;
```

**KNOWLEDGE.md references non-existent `integration.rs`** - `.features/mds-compiler/KNOWLEDGE.md:419,632,644`
**Confidence**: 92%
- Problem: Three references to `crates/mds-cli/tests/integration.rs` remain in the feature knowledge file, but that file was deleted and split into 9 categorized test files (`language.rs`, `objects.rs`, `imports.rs`, `errors.rs`, `cli_build.rs`, `cli_commands.rs`, `security.rs`, `frontmatter.rs`, `warnings.rs`). This creates stale documentation that will mislead future contributors.
- Fix: Update all three references to describe the split test structure:
  - Line 419: Change `tests/integration.rs` to `the appropriate test file in tests/` (e.g., `tests/language.rs`, `tests/security.rs`)
  - Lines 632, 644: Replace the single file description with a summary of the split test modules or remove the stale reference

### LOW

**Inconsistent `HashMap` import style across test files** - `crates/mds-cli/tests/objects.rs:126-158`, `crates/mds-cli/tests/frontmatter.rs:50`
**Confidence**: 82%
- Problem: Some test files (`language.rs`, `errors.rs`, `security.rs`) import `HashMap` at the top via `use std::collections::HashMap;` and use it unqualified. Others (`objects.rs`, `frontmatter.rs`) use the fully qualified path `std::collections::HashMap` inline. This creates two competing patterns within the same test suite.
- Fix: Add `use std::collections::HashMap;` to the top of `objects.rs` and `frontmatter.rs` (matching the pattern in the majority of files), then use unqualified `HashMap` in the function bodies.

**`security.rs` manually constructs fixture path instead of using helper** - `crates/mds-cli/tests/security.rs:131-134`
**Confidence**: 80%
- Problem: `security.rs` uses `std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("loop.mds")` instead of the shared `fixture("loop.mds")` helper that does the same thing. It also doesn't import `fixture` from common despite other files using it for the same purpose.
- Fix: Add `fixture` to the import (`use common::{fixture, mds_bin};`) and replace the manual path construction with `fixture("loop.mds")`.

## Issues in Code You Touched (Should Fix)

### LOW

**Inconsistent qualified path for `mds_bin` in `errors.rs`** - `crates/mds-cli/tests/errors.rs:199`
**Confidence**: 80%
- Problem: `errors.rs` imports `use common::fixture;` at the top but uses `common::mds_bin()` (qualified) on line 199 instead of adding `mds_bin` to the import. Other files that use both (`cli_build.rs`, `cli_commands.rs`, `warnings.rs`) import both: `use common::{fixture, mds_bin};`.
- Fix: Change line 2 to `use common::{fixture, mds_bin};` and use `mds_bin()` unqualified on line 199.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Shared `HashMap` import in common helper** - `crates/mds-cli/tests/common/mod.rs` (Confidence: 65%) -- Consider re-exporting `std::collections::HashMap` from `common/mod.rs` so test files needing it can use `use common::HashMap;` for a single import line, reducing per-file boilerplate.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 2 |
| Should Fix | - | 0 | 0 | 1 |
| Pre-existing | - | - | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The workspace split is executed cleanly: Cargo.toml metadata is consistent between crates, `miette` features are correctly layered (fancy only in CLI), all public API functions retain `#[must_use]`, the `Result` return type alias is unified in the CLI, and module visibility is properly locked down with `pub(crate)`. The few consistency issues found are minor: a dead import triggering a compiler warning, stale documentation referencing deleted files, and minor style variations in test file imports. None are blocking.

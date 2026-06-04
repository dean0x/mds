# Documentation Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**KNOWLEDGE.md references non-existent `crates/mds-cli/tests/integration.rs`** - `.features/mds-compiler/KNOWLEDGE.md:632,644`
**Confidence**: 95%
- Problem: The KNOWLEDGE.md "Key Files" section (line 632) and "Related" section (line 644) both reference `crates/mds-cli/tests/integration.rs`, but this file does not exist. The integration tests were split into 9 categorized files (`cli_build.rs`, `cli_commands.rs`, `errors.rs`, `frontmatter.rs`, `imports.rs`, `language.rs`, `objects.rs`, `security.rs`, `warnings.rs`) as part of this PR. The documentation references a file path that actively misleads contributors.
- Fix: Update both references to reflect the split test structure. For example:

  Line 632 (Key Files):
  ```markdown
  - `crates/mds-cli/tests/` — end-to-end integration tests split into categorized modules: `language.rs` (core features), `objects.rs` (map/dot-notation), `imports.rs` (module system), `errors.rs` (diagnostics), `cli_build.rs` (build command), `cli_commands.rs` (check/init/flags), `security.rs` (resource limits/guards), `frontmatter.rs` (output behavior), `warnings.rs` (warning collection)
  ```

  Line 644 (Related):
  ```markdown
  - `crates/mds-cli/tests/` — covers all directive combinations including object access, key-value iteration, dot-path conditions, frontmatter preservation, nested function calls, CLI stdin/quiet mode, auto-detect, error help-text, scope/export visibility rules, re-export error scenarios, default file output, `--out-dir`, `mds.json` config behavior, and all resource limit scenarios
  ```

### MEDIUM

**"Adding a New Directive" guide references stale test paths** - `.features/mds-compiler/KNOWLEDGE.md:419`
**Confidence**: 90%
- Problem: Step 7 of the "Adding a New Directive" integration pattern says: "Add integration test fixture in `tests/fixtures/` and a test in `tests/integration.rs`". The fixtures are now at `crates/mds-cli/tests/fixtures/` and the tests are split across multiple files. A developer following this guide would create files in the wrong location.
- Fix: Update step 7 to:
  ```
  7. Add integration test fixture in `crates/mds-cli/tests/fixtures/` and a test in the appropriate categorized test file under `crates/mds-cli/tests/` (e.g. `language.rs` for core features, `imports.rs` for module system)
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**README `cargo install --path .` is incorrect for workspace layout** - `README.md:10`
**Confidence**: 85%
- Problem: The README instructs users to run `cargo install --path .` to install from source. With the workspace split, the root `Cargo.toml` is now a `[workspace]` manifest without a `[package]` section. Running `cargo install --path .` from the workspace root will fail because Cargo cannot find a package to install. The binary is defined in `crates/mds-cli/Cargo.toml`.
- Fix: Update the install command in README.md:
  ```bash
  cargo install --path crates/mds-cli
  ```

## Pre-existing Issues (Not Blocking)

None.

## Suggestions (Lower Confidence)

- **Missing workspace-level documentation** - `README.md` (Confidence: 65%) — The README does not mention the workspace structure or the existence of two crates (`mds-core` library and `mds-cli` binary). Contributors looking to understand the project layout may benefit from a brief "Project Structure" section.

- **Root Cargo.toml removed `description` and `categories` fields** - `Cargo.toml` (Confidence: 60%) — The root workspace manifest no longer carries `description` or `categories`. These are properly set in the sub-crate manifests, but if the project ever publishes from the workspace root, these would be needed. Likely fine as-is since workspace manifests cannot be published.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The KNOWLEDGE.md was comprehensively updated for the workspace split — path references throughout the document are correct. However, two stale references to `integration.rs` (which no longer exists after the test split) actively mislead developers. The README install command also needs updating for the workspace layout. The doc comment fixes for rustdoc angle-bracket warnings are correctly applied.

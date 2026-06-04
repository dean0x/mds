# Documentation Review Report

**Branch**: main (d0624a2...HEAD)
**Date**: 2026-05-15
**Scope**: Full codebase review for PUBLIC RELEASE readiness

## Issues in Your Changes (BLOCKING)

### CRITICAL

**No README.md exists** - project root
**Confidence**: 100%
- Problem: The project has no README.md file. For a project targeting public release, this is the single most important documentation artifact. Users who find the project on GitHub or crates.io will see nothing explaining what MDS is, how to install it, or how to use it.
- Fix: Create `README.md` with at minimum:
  - Project description (what MDS is, who it's for)
  - Installation instructions (`cargo install mds` or build from source)
  - Quick start example (a simple .mds file and its compiled output)
  - Link to `spec.md` for the full language reference
  - Link to the `test_playground/` directory for more examples
  - License information

**No LICENSE file exists** - project root
**Confidence**: 100%
- Problem: The project has no LICENSE file, and `Cargo.toml` has no `license` field. This makes the project legally ambiguous for anyone who wants to use it. Without a license, the code is technically "all rights reserved" by default. This is a hard blocker for public release and for publishing to crates.io (which requires a `license` or `license-file` field).
- Fix: Choose a license (MIT, Apache-2.0, or dual MIT/Apache-2.0 are common for Rust projects), add a `LICENSE` file, and add the `license` field to `Cargo.toml`.

### HIGH

**Cargo.toml missing standard package metadata for crates.io** - `Cargo.toml:1-6`
**Confidence**: 95%
- Problem: `Cargo.toml` is missing several fields required or strongly recommended for publishing to crates.io: `license`, `repository`, `homepage`, `authors`, `keywords`, `categories`, and `documentation`. Without `license`, `cargo publish` will fail. Without the others, the crate page will be sparse and hard to discover.
- Fix: Add the missing metadata fields:
  ```toml
  [package]
  name = "mds"
  version = "0.1.0"
  edition = "2021"
  description = "MDS (Markdown Script) compiler — composable LLM prompt templates"
  license = "MIT"
  repository = "https://github.com/<owner>/mds"
  homepage = "https://github.com/<owner>/mds"
  authors = ["<Author Name> <email>"]
  keywords = ["markdown", "template", "llm", "prompt", "compiler"]
  categories = ["template-engine", "command-line-utilities", "text-processing"]
  ```

**spec.md CLI section is out of date with actual CLI** - `spec.md:342-349`
**Confidence**: 95%
- Problem: The CLI section (Section 7) of `spec.md` shows only 4 basic commands, but the actual CLI has significantly more features that are undocumented in the spec:
  - `mds init` command (creates starter template) — not mentioned
  - `--set KEY=VALUE` flag (inline variable overrides) — not mentioned
  - `--out-dir` flag (output directory) — not mentioned
  - `-` stdin support (`mds build -`) — not mentioned
  - Auto-detection of `.mds` files in current directory — not mentioned
  - `mds.json` project configuration — not mentioned
  - `-q/--quiet` flag — not mentioned
  - Exit code semantics (0/1/2/3) — not mentioned
- Fix: Expand Section 7 of `spec.md` to cover all CLI features, or create a separate CLI reference document that describes every command, flag, and option.

**No CHANGELOG.md** - project root
**Confidence**: 90%
- Problem: For public release, a CHANGELOG is important to communicate what each version contains. This is the initial release, so a CHANGELOG would be short, but establishing the pattern now sets a good precedent. This becomes increasingly important with subsequent releases.
- Fix: Create `CHANGELOG.md` following the Keep a Changelog format:
  ```markdown
  # Changelog

  ## [0.1.0] - 2026-05-15

  ### Added
  - MDS language compiler with lexer, parser, evaluator, resolver, and CLI
  - Template directives: @if, @for, @define, @import, @export, @include
  - Module system with alias, merge, and selective imports
  - Runtime variable overrides via --vars and --set
  - Project config via mds.json
  - mds init command for quick start
  - 286 tests (integration + unit)
  ```

### MEDIUM

**spec.md Section 12 status is stale** - `spec.md:540-542`
**Confidence**: 95%
- Problem: Section 12 reads "v0.1 -- Draft specification. Subject to change during implementation." Implementation is complete with 286 tests. The status should reflect the current state.
- Fix: Update to something like: "v0.1 -- Released. Implementation complete with full test coverage."

**No examples/ directory — test_playground serves as de facto examples** - project root
**Confidence**: 85%
- Problem: The `test_playground/` directory contains excellent examples (15 numbered scenarios from basic interpolation to complex multi-module prompts), but its name suggests internal testing rather than user-facing documentation. Users browsing the repository will not immediately recognize this as the examples directory.
- Fix: Either rename `test_playground/` to `examples/` (or keep test_playground and create a symlink/separate curated `examples/` directory). The numbered files (`01_basic.mds` through `15_runtime_vars.mds`) with their corresponding `output/` directory are well-structured for learning.

**No CONTRIBUTING.md** - project root
**Confidence**: 80%
- Problem: For an open-source public release, a CONTRIBUTING.md helps potential contributors understand how to set up the development environment, run tests, and submit changes. Without it, the barrier to contribution is higher.
- Fix: Create `CONTRIBUTING.md` covering at minimum:
  - Prerequisites (Rust toolchain version)
  - How to build (`cargo build`)
  - How to run tests (`cargo test`)
  - How to run the compiler locally
  - Code style and conventions

**spec.md does not document mds.json project configuration** - `spec.md` (absent)
**Confidence**: 90%
- Problem: The `mds.json` project configuration file is a user-facing feature (controls default output directory, is auto-discovered by walking up the directory tree) but is completely undocumented in the spec. A user who creates an `mds.json` with `{"build": {"output_dir": "dist"}}` would have no way to discover this feature from the documentation.
- Fix: Add a new section to `spec.md` (e.g., Section 7.1 or a new Section 8) documenting:
  - The `mds.json` file format and its fields
  - Auto-discovery behavior (walks up directory tree)
  - How `output_dir` interacts with `-o` and `--out-dir` precedence

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Public `error` module items lack module-level rustdoc** - `src/error.rs:1`
**Confidence**: 80%
- Problem: The `error` module is public (`pub mod error` in lib.rs) but has no `//!` module-level doc comment. While `MdsError` itself is well-documented with `#[diagnostic]` attributes, the module lacks an overview explaining the error hierarchy and how errors flow from lexer through evaluator.
- Fix: Add a module doc comment at the top of `src/error.rs`:
  ```rust
  //! Error types for the MDS compiler.
  //!
  //! All errors are represented by [`MdsError`], which covers every
  //! failure mode from syntax errors through circular imports.
  //! Errors carry optional source spans for rich diagnostic output
  //! via the `miette` crate.
  ```

**Public `value` module items lack module-level rustdoc** - `src/value.rs:1`
**Confidence**: 80%
- Problem: The `value` module is public (`pub mod value` in lib.rs) but has no `//!` module-level doc comment. `Value` is a core public type that users of the library API need to understand.
- Fix: Add a module doc comment at the top of `src/value.rs`:
  ```rust
  //! Runtime value types for MDS template variables.
  //!
  //! [`Value`] represents the types available in MDS templates:
  //! strings, numbers, booleans, arrays, and null. Values are
  //! created from YAML frontmatter or JSON runtime variables.
  ```

## Pre-existing Issues (Not Blocking)

### LOW

**Rustdoc examples use `no_run` where runnable examples would be better** - `src/lib.rs:17-20`
**Confidence**: 65% (moved to Suggestions)

## Suggestions (Lower Confidence)

- **Some rustdoc examples could be runnable** - `src/lib.rs:17` (Confidence: 65%) — Several doc examples use `no_run` because they reference filesystem paths. Consider adding runnable in-memory examples using `compile_str` alongside the `no_run` file-based examples.

- **test_playground output files could include comments** - `test_playground/output/` (Confidence: 60%) — The expected output files are bare markdown with no comments explaining what they demonstrate. Adding a brief comment header (even just in a companion file) would help users understand what each example tests.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 2 | 3 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

### What's Done Well

- **Excellent rustdoc on lib.rs**: The crate-level documentation has a proper Quick Start section with working examples covering all major use cases (in-memory, file-based, with variables, validation).
- **Every public function in lib.rs has doc comments** with examples, parameter documentation, and `#[must_use]` annotations.
- **Outstanding error messages**: Error output includes source spans, contextual labels, error codes (`mds::syntax`, `mds::undefined_var`, etc.), and actionable help text — a quality bar matching `rustc` itself.
- **CLI help text is thorough**: All subcommands have descriptions, examples in `after_help`, and clear option documentation.
- **spec.md is comprehensive**: 542 lines covering syntax, semantics, scoping rules, compilation model, error format, grammar summary, and editor integration — a strong language reference.
- **Internal code is well-documented**: Even `pub(crate)` modules have doc comments on key types and functions, with explanations of design decisions (e.g., why `CapturedScope` uses owned values to break reference cycles).
- **Zero `cargo doc` warnings**: All public items compile clean.

**Documentation Score**: 4/10
**Recommendation**: CHANGES_REQUESTED

The code documentation (rustdoc, error messages, CLI help, spec) is strong — well above average for a v0.1 project. However, the project is missing the fundamental "wrapper" documentation required for public release: no README, no LICENSE, no CHANGELOG, incomplete Cargo.toml metadata, and several CLI features undocumented in the spec. The two CRITICAL issues (README and LICENSE) must be resolved before any public release; the HIGH issues should be resolved concurrently.

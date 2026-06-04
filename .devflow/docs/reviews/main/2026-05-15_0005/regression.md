# Regression Review Report

**Branch**: main (d0624a2...HEAD)
**Date**: 2026-05-15
**Scope**: Full codebase review (new project, 2 commits)

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing `#[non_exhaustive]` on public enums `MdsError` and `Value`** - `src/error.rs:22`, `src/value.rs:10`
**Confidence**: 95%
- Problem: Both `MdsError` (14 variants) and `Value` (5 variants) are public enums exposed via `pub mod error` and `pub mod value`, but neither is marked `#[non_exhaustive]`. Any downstream crate that exhaustively matches on these enums will break when a new variant is added (e.g., adding a `Map` variant to `Value` or a `Deprecation` warning variant to `MdsError`). For a v0.1 project explicitly planning future additions (spec section 10 lists deferred features like object/map types, `@elseif`, built-in functions), this is near-certain to cause breaking changes.
- Fix: Add `#[non_exhaustive]` to both enums before any public release:
  ```rust
  #[non_exhaustive]
  #[derive(Error, Debug, Diagnostic, Clone)]
  pub enum MdsError { ... }

  #[non_exhaustive]
  #[derive(Debug, Clone, PartialEq)]
  pub enum Value { ... }
  ```

**Public API leaks transitive dependency types (`serde_yml::Value`, `serde_json::Value`)** - `src/value.rs:34`, `src/value.rs:67`
**Confidence**: 92%
- Problem: `Value::from_yaml(yaml: serde_yml::Value)` and `Value::from_json(json: serde_json::Value)` are `pub` methods that accept foreign types in their signatures. This forces downstream crates to depend on the exact same versions of `serde_yml` and `serde_json`, and any upgrade of those dependencies becomes a semver-breaking change for the library. The `serde_yml` dependency at `0.0.12` is itself pre-release (`0.0.x`), making this especially fragile.
- Fix: Either make these methods `pub(crate)` (they are only called from `resolver.rs`), or accept a `&str` instead and parse internally:
  ```rust
  // Option A: restrict visibility (recommended for v0.1)
  pub(crate) fn from_yaml(yaml: serde_yml::Value) -> Result<Value, MdsError> { ... }
  pub(crate) fn from_json(json: serde_json::Value) -> Result<Value, MdsError> { ... }

  // Option B: string-based public API
  pub fn from_yaml_str(s: &str) -> Result<Value, MdsError> { ... }
  pub fn from_json_str(s: &str) -> Result<Value, MdsError> { ... }
  ```

### MEDIUM

**`pub mod error` and `pub mod value` expose all `pub` items including constructor methods** - `src/lib.rs:41`, `src/lib.rs:48`
**Confidence**: 85%
- Problem: Both modules are `pub mod` (not `pub(crate) mod`), which means every `pub` method inside is part of the public API. In `error.rs`, this includes 28 constructor methods (`syntax()`, `syntax_at()`, `undefined_var()`, etc.) and all struct field names. In `value.rs`, this includes `from_yaml()`, `from_json()`, and all trait implementations. This creates a very large API surface for a v0.1 library, making future refactoring risky. Adding the `_at` constructors as public means error construction details become part of the contract.
- Fix: Consider re-exporting only what consumers need:
  ```rust
  // Instead of:
  pub mod error;
  pub mod value;

  // Narrower surface:
  pub(crate) mod error;
  pub(crate) mod value;
  pub use error::MdsError;
  pub use value::Value;
  ```
  Then selectively add `pub` to methods that are genuinely part of the intended API.

**Exit code contract (0/1/2/3) is undocumented in library API** - `src/main.rs:359`
**Confidence**: 82%
- Problem: The exit code categorization (0=success, 1=logic/syntax, 2=I/O, 3=resource limit) is documented only in a code comment on the `exit_code()` function. Scripts and CI pipelines that depend on these exit codes have no stable contract. Adding new `MdsError` variants could change which exit code bucket they fall into, since the catch-all `_ => 1` means new variants default to exit code 1 regardless of their nature.
- Fix: Document exit codes in the CLI help text and consider adding integration tests for each exit code category (partially done: `exit_code_success`, `exit_code_file_not_found`, `exit_code_syntax_error`, `exit_code_resource_limit` exist, which is good coverage).

**`ResolvedModule` struct fields are public but the module is `pub(crate)`** - `src/resolver.rs:36-41`
**Confidence**: 80%
- Problem: `ResolvedModule` has all fields `pub` (`functions`, `prompt_body`, `has_explicit_exports`, `explicit_exports`) and several `pub` methods (`get_export`, `get_all_exports`, `get_prompt_value`). While the `resolver` module itself is `pub(crate)`, this struct is returned from `pub(crate)` methods via `Arc<ResolvedModule>`. If the `resolver` module is ever made public (a natural evolution for embedding use cases), all these fields and methods become frozen API surface. The field names like `has_explicit_exports` and `explicit_exports` are implementation details of the export system.
- Fix: Make fields private with accessor methods, or ensure the struct remains unexposed. Current `pub(crate)` containment is adequate for now, but note the risk.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`serde_yml` at `0.0.12` is a pre-release dependency pinned by exact version** - `Cargo.toml:12`
**Confidence**: 88%
- Problem: `serde_yml = "0.0.12"` is a `0.0.x` pre-release crate. Per semver, `0.0.x` means every patch can be breaking. The Cargo.toml comment acknowledges this ("Pre-release (0.0.x); track for 0.1.x stability milestone"), but the current pin means `cargo update` could pull in `0.0.13` with breaking changes. If `serde_yml` changes its `Value` enum or parsing behavior, the MDS compiler's YAML frontmatter parsing could silently change behavior.
- Fix: Pin with `=0.0.12` for reproducible builds, or evaluate switching to a stable YAML parser (e.g., `serde_yaml` v0.9+ if API-compatible, or `yaml-rust2`).

**No `CHANGELOG.md` or version tracking mechanism** - project root
**Confidence**: 83%
- Problem: For a project preparing for public release at v0.1.0, there is no changelog. Early adopters have no way to know what changed between releases. Combined with the lack of `#[non_exhaustive]`, version bumps provide no warning about breaking changes.
- Fix: Create a `CHANGELOG.md` following Keep a Changelog format before publishing to crates.io.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`Value` enum lacks `Map`/`Object` variant but `from_yaml` and `from_json` reject maps with different error messages** - `src/value.rs:59-61`, `src/value.rs:91-93`
**Confidence**: 85%
- Problem: The spec explicitly defers object/map types to post-v0.1 (section 10). When that feature lands, adding a `Map(HashMap<String, Value>)` variant will be a breaking change for anyone exhaustively matching on `Value`. The current error messages differ slightly: YAML says "object/map types" while JSON says "object/map types" (consistent, good). But the rejection means frontmatter like `config: {key: value}` silently fails rather than providing a forward-compatible path.
- Impact: Future regression risk. The `#[non_exhaustive]` fix above addresses this.

### LOW

**`MdsError` does not implement `PartialEq`** - `src/error.rs:22`
**Confidence**: 80%
- Problem: `MdsError` derives `Clone` and `Debug` but not `PartialEq`. This is intentional (the `Arc<NamedSource>` field makes `PartialEq` non-trivial), but it means downstream tests cannot use `assert_eq!` on errors. The integration tests work around this by matching on string representations. This is a minor ergonomic issue, not a regression, but worth noting as a deliberate API choice.
- Impact: No regression risk, but adding `PartialEq` later would be a compatible addition.

## Suggestions (Lower Confidence)

- **Consider `#[doc(hidden)]` on `MdsError` constructor methods** - `src/error.rs:178-466` (Confidence: 70%) -- The 28 constructor methods (`syntax()`, `syntax_at()`, etc.) are implementation details that don't need to be part of the public API contract. Hiding them from docs reduces the perceived API surface while keeping them accessible.

- **`clean_output` strips `\r` globally which changes content inside code blocks** - `src/lib.rs:332-361` (Confidence: 65%) -- The `clean_output` function strips all `\r` characters and collapses excess newlines. This runs on the final output after evaluation, including content that was inside ```` ``` ```` code blocks. If a template intentionally produces `\r\n` line endings (e.g., for Windows batch scripts), the output will have `\r` stripped. This is likely intentional normalization, but could surprise users.

- **`compile_file` wrapper adds minimal value** - `src/lib.rs:375` (Confidence: 62%) -- `compile_file(path: &str)` is a thin wrapper that converts `&str` to `Path`. It adds a second entry point for the same operation. If both are maintained, any change to `compile` must be reflected in the wrapper, creating a regression surface.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 2 | 2 | - |
| Should Fix | - | - | 2 | - |
| Pre-existing | - | - | 1 | 1 |

**Regression Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The two HIGH issues -- missing `#[non_exhaustive]` on public enums and leaked transitive dependency types in the public API -- are the primary regression risks. For a v0.1 project, these are easy to fix now but extremely expensive to fix after publication (any fix becomes a semver-breaking change). The project has strong test coverage (286 tests, 171 integration) and well-structured error handling. Addressing the `#[non_exhaustive]` and visibility issues before the first crates.io publish would make the API significantly more resilient to future changes.

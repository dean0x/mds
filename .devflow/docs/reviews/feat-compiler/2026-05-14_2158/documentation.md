# Documentation Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Doc comment for `MAX_CONFIG_SIZE` merges into `load_config` doc comment** - `src/main.rs:33`
**Confidence**: 95%
- Problem: The doc comment `/// Maximum allowed size for ...` on line 33 runs directly below the last line of the `load_config` function doc comment (line 32: `/// resolve relative output_dir values.`) with no blank line separator. Rustdoc attaches all `///` lines preceding an item to that item. Since `const MAX_CONFIG_SIZE` is the next item, the constant gets the correct doc. However, `load_config` on line 36 now has NO doc comment at all -- the `///` block from lines 25-33 is entirely consumed by `MAX_CONFIG_SIZE` because there is no intervening blank line + separate `///` block for the function. This means `load_config` is a public-facing internal function with a 7-line doc comment that was severed from it by the insertion of the constant.
- Fix: Add a blank line between the `load_config` doc comment and the `MAX_CONFIG_SIZE` doc comment to clearly separate them:
```rust
/// The `config_dir` is the directory that *contains* `mds.json` — used to
/// resolve relative `output_dir` values.

/// Maximum allowed size for `mds.json` (1 MB) to prevent runaway memory use.
const MAX_CONFIG_SIZE: u64 = 1024 * 1024;

/// Walk up from `start` looking for `mds.json`.
///
/// Returns `Ok(Some((config, config_dir)))` when found, `Ok(None)` when no
/// `mds.json` exists in the hierarchy, or `Err(...)` when a file is found but
/// contains invalid JSON.
///
/// The `config_dir` is the directory that *contains* `mds.json` — used to
/// resolve relative `output_dir` values.
fn load_config(
```
Alternatively, move the doc comment block for `load_config` directly above `fn load_config` (after the const), restoring the original attachment.

### MEDIUM

**`ModuleCtx` struct has incomplete field documentation** - `src/resolver.rs:532-538`
**Confidence**: 82%
- Problem: This PR added a doc comment for `file_str` (`/// Canonical display path of the source file...`) but the other three fields (`source`, `base_dir`, `runtime_vars`) remain undocumented. Since the PR explicitly added one field's doc comment, the inconsistency is introduced by this change. Partial documentation can mislead -- a reader might assume the undocumented fields are self-evident, but `source` vs `file_str` distinction is subtle.
- Fix: Add brief doc comments for the remaining fields:
```rust
struct ModuleCtx<'a> {
    /// Canonical display path of the source file (e.g. the path shown in error messages).
    file_str: &'a str,
    /// Raw source text of the file being resolved.
    source: &'a str,
    /// Directory containing the source file, used to resolve relative import paths.
    base_dir: &'a Path,
    /// Runtime variable overrides passed from the CLI or API caller.
    runtime_vars: &'a HashMap<String, Value>,
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`resolve_import` method lacks a doc comment** - `src/resolver.rs:380`
**Confidence**: 80%
- Problem: Every other method in the `ModuleCache` impl block has a doc comment (`canonicalize_and_check`, `read_validated_file`, `resolve`, `resolve_source`, `process_module`, `collect_definitions_and_imports`). The `resolve_import` method, which handles all three import variants (alias, merge, selective), has none. While the PR did not add this method, it modified surrounding methods and the `collect_definitions_and_imports` that calls it.
- Fix: Add a brief doc comment:
```rust
/// Resolve a single `@import` directive, adding its exports into the caller's scope.
///
/// Dispatches to the appropriate handler for alias, merge, and selective imports.
fn resolve_import(
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`CollectedDefs` fields could use brief doc comments** - `src/resolver.rs:525-529` (Confidence: 65%) -- The struct was promoted from a type alias to a named struct in this PR. While the struct-level doc comment explains its purpose, individual field comments (especially `has_explicit_exports` vs `explicit_exports`) would clarify the distinction.

- **Inline comments in `evaluate_for` and `invoke_function` are duplicated verbatim** - `src/evaluator.rs:200-203,299-302` (Confidence: 70%) -- The double-fault error-preservation comment block is copy-pasted identically in both locations. This is intentional per the commit message ("Apply double-fault error-preservation pattern"), but a short shared comment referencing a single rationale location would reduce drift risk if the pattern's semantics evolve.

- **`to_namespace` visibility comment could reference export spec** - `src/resolver.rs:508-509` (Confidence: 62%) -- The inline comment explains the prompt_body export rule but does not reference where this rule is specified (e.g., spec.md). A brief cross-reference would help future maintainers verify correctness.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The documentation quality is generally strong -- the new code has good inline comments explaining "why" (double-fault preservation, LIFO invariants, closure capture semantics), doc comments on refactored methods are thorough, and the `serde_yml` pre-release tracking comment in Cargo.toml is useful. The blocking HIGH issue (severed doc comment on `load_config`) should be fixed before merge as it creates a misleading doc attachment for `MAX_CONFIG_SIZE`.

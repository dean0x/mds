# Documentation Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`CollectedDefs` type alias undocumented fields** - `src/resolver.rs:512`
**Confidence**: 85%
- Problem: `CollectedDefs` is a type alias for a 3-element tuple `(HashMap<String, Arc<FunctionDef>>, bool, HashSet<String>)`. While the type alias has a one-line doc comment, the tuple positions are not documented. Callers must read `collect_definitions_and_imports` to know that position 0 is `functions`, position 1 is `has_explicit_exports`, and position 2 is `explicit_exports`. This is a named-yet-opaque return type.
- Fix: Convert `CollectedDefs` to a named struct, or add field documentation to the type alias doc comment:
```rust
/// Collected output of the AST definition/import walk in `collect_definitions_and_imports`.
///
/// Fields (in tuple order):
/// 1. `HashMap<String, Arc<FunctionDef>>` — functions defined in this module (including re-exports)
/// 2. `bool` — `true` if any `@export` directive was encountered
/// 3. `HashSet<String>` — the explicitly listed export names
type CollectedDefs = (HashMap<String, Arc<FunctionDef>>, bool, HashSet<String>);
```
A named struct would be idiomatic Rust but the tuple approach is consistent with the project's "no comments by default, only comment WHY not WHAT" policy. The doc comment is the minimum fix.

**`ModuleCtx` struct fields lack doc comments** - `src/resolver.rs:515-519`
**Confidence**: 82%
- Problem: `ModuleCtx` is a new struct introduced in this PR to bundle four context fields. The struct has a one-line doc comment but none of the four fields (`file_str`, `source`, `base_dir`, `runtime_vars`) have documentation. While their names are descriptive, `file_str` is particularly opaque -- it is the canonical display string of the source file path, not a file string content. The project's KNOWLEDGE.md correctly documents this (`ModuleCtx struct bundles the borrowed per-module context`), but the code itself does not.
- Fix: Add a brief doc comment to at least the non-obvious field:
```rust
struct ModuleCtx<'a> {
    /// Canonical display path of the source file (e.g. `/abs/path/to/foo.mds`).
    file_str: &'a str,
    source: &'a str,
    base_dir: &'a Path,
    runtime_vars: &'a HashMap<String, Value>,
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**New `MdsError` constructor methods lack doc comments (5 occurrences)** - `src/error.rs:443-465`
**Confidence**: 83%
- `src/error.rs:443` (`io`), `src/error.rs:449` (`yaml_error`), `src/error.rs:455` (`json_error`), `src/error.rs:461` (`not_mds_file`), `src/error.rs:437` (`resource_limit`)
- Problem: Five new convenience constructors were added to `MdsError` without doc comments, while the existing `_at` constructors also lack doc comments. The feature knowledge specifies: "public API functions have `#[must_use]` attributes" and "error variants have `help(...)` diagnostic attributes". The constructors are `pub` methods on a public enum. No doc comments means `cargo doc` shows empty entries for these methods.
- Fix: Per the project's "no comments by default" policy, this is acceptable for internal constructors. However, `MdsError` is a public type (`pub mod error`). If the intent is for downstream callers to construct errors, brief doc comments would help. If they are internal-only, consider reducing visibility. This is a judgment call rather than a hard defect.

## Pre-existing Issues (Not Blocking)

_No CRITICAL pre-existing documentation issues found._

## Suggestions (Lower Confidence)

- **`exit_code` function could document the fallback for non-`MdsError` errors more explicitly** - `src/main.rs:329` (Confidence: 68%) -- The doc comment mentions `miette::miette!()` errors fall through to exit code 1, but the function is in a `main.rs` private scope, so external consumers cannot reach it. The KNOWLEDGE.md correctly documents this behavior. Low priority since the function is private and well-documented in KNOWLEDGE.md.

- **`check_str_collecting_warnings` doc comment references `check_str_with` but the unlike relationship is backward** - `src/lib.rs:302` (Confidence: 65%) -- The doc says "Unlike `check_str_with`", but `check_str_with` is the function that prints warnings. The `check_str_collecting_warnings` variant is the one that does NOT print. The phrasing is technically correct ("unlike X, this function does not print") but could be clearer since `check_str_with` itself does not have a collecting variant -- it delegates to `check_str_collecting_warnings` nowhere. The relationship is: `check` prints, `check_collecting_warnings` does not. The "Unlike" phrasing follows the existing `compile_collecting_warnings` doc pattern, so it is at least consistent.

- **No CHANGELOG or README in the repository** - (Confidence: 75%) -- The project has no README.md or CHANGELOG.md. This PR introduces significant behavioral changes (default file output instead of stdout, new `mds.json` config, new `--out-dir` flag, new exit codes). These are breaking changes for anyone piping `mds build` output. The CLI help text was updated, which is good, but there is no external documentation for the breaking change. The KNOWLEDGE.md gotcha at line 498 correctly notes this: "Existing scripts that pipe `mds build foo.mds` and expect stdout output must be updated to add `-o -`."

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

The documentation quality in this PR is strong overall. Key positive observations:

1. **KNOWLEDGE.md updated thoroughly**: The feature knowledge file was comprehensively updated to reflect all architectural changes -- `EvalContext`, `CapturedScope`, `IndexSet`, `Arc<FunctionDef>`, `CollectedDefs`, `ModuleCtx`, the `process_module` decomposition, new CLI output behavior, `mds.json` config, exit codes. Anti-patterns and gotchas sections were updated to match.

2. **New public API functions have proper doc comments**: `check_collecting_warnings` and `check_str_collecting_warnings` in `src/lib.rs` include doc comments with `# Examples`, `#[must_use]` attributes, and `rust,no_run` or `rust` doc-test markers. This follows the existing pattern established by `compile_collecting_warnings`.

3. **Inline code comments explain "why" not "what"**: The new code follows the project's no-comments-by-default policy. Where comments exist, they explain rationale (e.g., "captured.functions are owned FunctionDef (not Arc) -- wrap in Arc for scope insertion", "shift_remove preserves insertion order of remaining elements", "IndexSet provides both O(1) membership test and insertion-ordered iteration"). These are all "why" comments.

4. **CLI help text updated**: The `long_about` and `after_help` strings for `mds build` were updated to reflect the new default behavior (file output, `-o -` for stdout, `--out-dir`). The output arg description documents the precedence.

5. **Struct doc comments added for new types**: `EvalContext`, `Lexer`, `CapturedScope`, `FunctionDef.captured` all have doc comments explaining their purpose and key design decisions.

6. **Deleted stale file**: `autobeat-orchestrator-analysis.md` was removed -- it was an unrelated analysis report that should not have been in the repo.

The conditions for approval are minor: the `CollectedDefs` tuple alias and `ModuleCtx` fields would benefit from slightly more documentation, and the new `MdsError` constructors could use brief doc comments given that `error` is a public module.

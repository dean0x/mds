# Complexity Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**`run()` function in main.rs is 150 lines with 4 levels of nesting** - `src/main.rs:405`
**Confidence**: 90%
- Problem: The `run()` function spans lines 405-555 (150 lines). It handles three CLI subcommands (`Build`, `Check`, `Init`) in a single match expression, with the `Build` arm alone spanning ~70 lines and containing nested `match output_path { Some(path) => { if let Some(parent) ... } }` at 4 nesting levels. This exceeds the 50-line warning threshold and approaches the 5-level nesting concern.
- Fix: Extract each match arm into a dedicated handler function. For example:
  ```rust
  fn run_build(input: Option<PathBuf>, output: Option<String>, out_dir: Option<PathBuf>, vars: Option<PathBuf>, set_vars: Vec<(String, String)>, quiet: bool) -> Result<(), miette::Error> { ... }
  fn run_check(input: Option<PathBuf>, vars: Option<PathBuf>, set_vars: Vec<(String, String)>, quiet: bool) -> Result<(), miette::Error> { ... }
  fn run_init(filename: PathBuf, force: bool, quiet: bool) -> Result<(), miette::Error> { ... }
  ```
  The `run()` function becomes a thin dispatch with ~15 lines.

**`collect_definitions_and_imports` is 93 lines with deeply nested match arms** - `src/resolver.rs:272`
**Confidence**: 85%
- Problem: This function (lines 272-365) has a `for node in body { match node { ... } }` with three levels of nesting inside the `Node::Export` arm: `match export { ExportDirective::ReExport { ... } => { ... validate ... resolve ... insert ... } }`. The ReExport and Wildcard arms each contain multiple fallible operations, error construction, and mutation. At ~93 lines it exceeds the 50-line threshold.
- Fix: Extract the three `ExportDirective` handlers into helper functions or at minimum extract the `Node::Export` arm into a `process_export(export, functions, explicit_exports, scope, ctx, warnings)` method, keeping `collect_definitions_and_imports` as a loop-and-dispatch orchestrator.

### MEDIUM

**`validate_and_read_file` is 88 lines performing security, I/O, and validation** - `src/resolver.rs:71`
**Confidence**: 82%
- Problem: This function (lines 71-153) performs symlink detection, root directory initialization, import depth check, path traversal prevention, file reading, size validation, UTF-8 validation, and extension detection -- all in a single function. At 88 lines with 3 nesting levels, it is cognitively dense. Each concern is sequential but the function does not have natural "paragraph breaks" that aid scanning.
- Fix: The function's sequential nature makes it acceptable as-is, but consider splitting into `canonicalize_and_check_symlink(path)` and `read_and_validate_file(canonical)` to separate filesystem resolution from content reading. This would also make each function independently testable.

**`resolve_import` has 3-level nesting with repetitive resolve patterns** - `src/resolver.rs:367`
**Confidence**: 80%
- Problem: The function (lines 367-447, 80 lines) matches three `ImportDirective` variants, each with nearly identical setup: `validate_import_path` -> `resolve_path` -> `self.resolve` -> `.map_err(attach_import_span)`. The `Selective` arm adds extra complexity with a closure and a `for name in names` loop. While the total nesting depth (3) is borderline, the repetition across arms makes the function harder to scan.
- Fix: Extract the common resolve-with-error-rewrite pattern:
  ```rust
  fn resolve_import_path(&mut self, path: &str, ctx: &ModuleCtx, warnings: &mut Vec<String>, offset: usize) -> Result<Arc<ResolvedModule>, MdsError> {
      validate_import_path(path)?;
      let import_path = resolve_path(ctx.base_dir, path);
      self.resolve(&import_path, ctx.runtime_vars, warnings)
          .map_err(|e| attach_import_span(e, path, ctx.file_str, ctx.source, offset))
  }
  ```
  Each arm then calls this shared helper, reducing duplication and line count.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`error.rs` constructor pairs follow a repetitive 10-line boilerplate pattern (9 pairs)** - `src/error.rs:177-466`
**Confidence**: 85%
- Problem: The `MdsError` impl block contains 9 pairs of `foo()` / `foo_at()` constructors, each following the exact same pattern: `foo()` sets span/src to None, `foo_at()` calls `at()` then fills in span/src. This is 290 lines of constructors for 9 error variants. While individual constructors are simple, the aggregate duplication inflates the file to 639 lines and makes it tedious to maintain -- adding a new span-aware variant requires copying and adapting two functions.
- Fix: This is a known Rust pattern where macros can reduce boilerplate. Consider a declarative macro:
  ```rust
  macro_rules! error_constructors {
      ($fn_name:ident, $variant:ident, $field:ident) => {
          pub fn $fn_name($field: impl Into<String>) -> Self { ... }
          pub fn paste::paste!{[<$fn_name _at>]}(...) -> Self { ... }
      };
  }
  ```
  Alternatively, accept the boilerplate as the cost of explicit, greppable constructors -- it is a trade-off. If the current approach is intentional, no change needed.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parser.rs` is 1024 lines** - `src/parser.rs`
**Confidence**: 82%
- Problem: At 1024 lines, `parser.rs` exceeds the 500-line file-length critical threshold. This file was not significantly modified in this PR (only a small `dot_notation_error` extraction), so this is pre-existing.
- Fix: Consider splitting into `parser.rs` (directive/block parsing) and `parser/interpolation.rs` (expression/arg parsing) in a future PR.

## Suggestions (Lower Confidence)

- **`resolve_output_path` has 6 numbered precedence steps in a 60-line function** - `src/main.rs:97` (Confidence: 70%) -- The function is well-documented with comments mapping to each step, and uses early returns effectively, but at 60 lines with 3 nesting levels it sits at the boundary. Consider extracting steps 4-5 (directory-based output resolution) into a shared helper since they share the `create_dir_all` + `derive_output_filename` pattern.

- **`process_module` parameter list has 6 parameters** - `src/resolver.rs:229` (Confidence: 65%) -- The function already bundles context into `ModuleCtx` for downstream calls but still accepts 6 parameters itself. The `(source, file_str, base_dir)` triple could be part of a struct, though the function is only called from two sites so the impact is limited.

- **`parse_cli_value` uses cascading if-let with early returns** - `src/main.rs:278` (Confidence: 60%) -- At 38 lines this is within bounds, but the pattern of `match` then `if let Ok(n)` then `if let Ok(f)` then `if starts_with('[')` is a type-coercion cascade. Readable now, but could become complex if more types are added.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR is a significant net improvement for complexity: `process_module` was decomposed from a monolith to a ~25-line orchestrator, the lexer was refactored from a monolithic function into a `Lexer` struct with focused `scan_*` methods, `EvalContext` consolidates three threaded parameters, and `CapturedScope` bundles three closure capture fields. These are genuine complexity reductions that make the codebase more maintainable.

The two HIGH issues (`run()` at 150 lines and `collect_definitions_and_imports` at 93 lines with deep nesting) are the main concerns. The `run()` function is the most impactful to address since it is the CLI entry point and will grow as new commands are added. The resolver function is borderline but its nesting depth and mixed concerns (define registration, import resolution, export collection) warrant extraction.

The error constructor boilerplate is a known Rust ergonomics trade-off and is not blocking, but worth acknowledging as the file approaches 640 lines.

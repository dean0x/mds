# Complexity Review Report

**Branch**: feat-workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`run_build` has 6 parameters** - `crates/mds-cli/src/main.rs:483-489`
**Confidence**: 85%
- Problem: `run_build` accepts 6 individual parameters (`input`, `output`, `out_dir`, `vars`, `set_vars`, `quiet`). This exceeds the 5-parameter warning threshold and makes the call site harder to read. The parameter list is a 1:1 mirror of the `Commands::Build` variant fields, suggesting a missed opportunity to pass the struct directly or use a parameter object.
- Fix: Extract a `BuildArgs` struct or pass the destructured `Commands::Build` enum variant directly:
  ```rust
  struct BuildArgs {
      input: Option<PathBuf>,
      output: Option<String>,
      out_dir: Option<PathBuf>,
      vars: Option<PathBuf>,
      set_vars: Vec<(String, String)>,
  }

  fn run_build(args: BuildArgs, quiet: bool) -> Result<()> { ... }
  ```
  This keeps `quiet` separate since it comes from the top-level `Cli` struct, not the subcommand.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`resolve_output_path` has 4 `Option`-wrapped parameters and 6 exit paths** - `crates/mds-cli/src/main.rs:126-185`
**Confidence**: 82%
- Problem: This function accepts 4 parameters (all wrapped in `Option`/reference-to-Option), each influencing a different branch of a 6-step precedence chain. The cyclomatic complexity is approximately 8 (6 return paths + 2 nested conditions in step 5). While each step is clearly documented with numbered comments, the combination of `Option` unwrapping, `match`, `if let`, and early returns across 59 lines makes it one of the more complex functions in the file. The function is pre-existing but its signatures were touched in this PR (return type unification).
- Fix: No immediate action required since the numbered comments provide a clear reading guide. If this function grows further, consider splitting the `mds.json` resolution (step 5, lines 156-173) into its own `resolve_from_config` helper.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`load_config` has 4 levels of nesting** - `crates/mds-cli/src/main.rs:36-83`
**Confidence**: 80%
- Problem: The `for` loop body contains an `if candidate.is_file()` branch that nests 3 additional error-handling operations. The deepest nesting (line 60-65, size check error) reaches 4 levels. This is at the warning threshold but not critical.
- Fix: Extract the file-found body into a `parse_config_file(candidate: &Path) -> Result<(MdsConfig, PathBuf)>` helper to flatten the nesting.

**`main.rs` is 779 lines (622 code + 157 test)** - `crates/mds-cli/src/main.rs`
**Confidence**: 80%
- Problem: The CLI file contains config loading, output path resolution, CLI definition, value parsing, subcommand runners, and unit tests all in a single file. At 779 lines it exceeds the 500-line critical threshold. However, the file is well-organized with section comments and clear function boundaries, and the 157 test lines are appropriately co-located with unit tests.
- Fix: Consider extracting config loading (`load_config`, `MdsConfig`, `BuildConfig`, `MAX_CONFIG_SIZE`) and output path resolution (`derive_output_filename`, `prepare_output_dir`, `resolve_output_path`) into a `config.rs` or `output.rs` module within the CLI crate. This would bring `main.rs` to approximately 500 lines.

## Suggestions (Lower Confidence)

- **`run_check` stdin/file branches duplicate warning iteration** - `crates/mds-cli/src/main.rs:540-561` (Confidence: 65%) -- The if/else branches for stdin vs file both contain identical `for w in &warnings { eprintln!("{w}"); }` blocks. A small refactor could unify the warning-printing logic.

- **`parse_cli_value` uses sequential if-let chains for type coercion** - `crates/mds-cli/src/main.rs:306-344` (Confidence: 62%) -- The function uses 5 sequential return points (keywords, i64, f64, bracket-array, string fallback). This is readable but could be cleaner as a match on parse results. Current form is idiomatic Rust, so this is stylistic.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The workspace split itself is a complexity *reduction* -- it cleanly separates library code (mds-core) from CLI code (mds-cli) with zero behavioral changes. The actual code changes are minimal: import consolidation and return type unification (`std::result::Result<T, miette::Error>` to `Result<T>` alias). These are net improvements to readability. The one blocking finding (`run_build` parameter count) is a minor structural issue that could be addressed in a follow-up. The pre-existing file length concern is partially mitigated by the good internal organization with section comments. Overall, this is a clean, low-risk refactor.

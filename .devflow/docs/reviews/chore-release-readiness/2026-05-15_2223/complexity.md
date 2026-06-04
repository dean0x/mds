# Complexity Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### HIGH

**`run_build` exceeds recommended function length (70 lines)** - `src/main.rs:447-516`
**Confidence**: 85%
- Problem: `run_build` is 70 lines with 4 levels of nesting (the `match output_path` -> `Some(path)` -> `if let Some(parent)` -> `if !parent.as_os_str().is_empty()` block). The function handles input resolution, config loading, output path resolution, compilation, warning output, directory creation, and file writing — at least 4 distinct responsibilities. This is the longest function introduced in the PR and sits firmly in the "warning" zone (50-200 lines) for function length.
- Fix: Extract the output-writing block (lines 492-514) into a dedicated `write_output(path: Option<PathBuf>, compiled: &str, quiet: bool)` function. This reduces `run_build` to ~45 lines and eliminates one nesting level. The `run_check` function demonstrates the right size at 36 lines.

```rust
fn write_output(
    output_path: Option<PathBuf>,
    compiled: &str,
    quiet: bool,
) -> Result<(), miette::Error> {
    match output_path {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        miette::miette!("cannot create output directory {}: {e}", parent.display())
                    })?;
                }
            }
            std::fs::write(&path, compiled).map_err(|e| {
                miette::miette!("cannot write {}: {e}", path.display())
            })?;
            if !quiet {
                eprintln!("Compiled to {}", path.display());
            }
        }
        None => {
            print!("{compiled}");
        }
    }
    Ok(())
}
```

### MEDIUM

**`run_build` does not use `resolve_input` unlike `run_check`** - `src/main.rs:458-467`
**Confidence**: 82%
- Problem: `run_check` delegates input resolution to the extracted `resolve_input()` helper (line 527), but `run_build` duplicates the input resolution logic inline (lines 458-467) because it additionally needs to print the "Building ..." banner. This creates an asymmetry between two sibling functions that share the same input-resolution intent, making the code harder to maintain — a change to auto-detection behavior requires updating two places.
- Fix: Have `resolve_input` return a tuple or use a wrapper that also returns whether auto-detection was used, then let `run_build` conditionally print the banner.

```rust
fn resolve_input_verbose(
    input: Option<PathBuf>,
    quiet: bool,
    verb: &str,
) -> std::result::Result<PathBuf, miette::Error> {
    match input {
        Some(p) => Ok(p),
        None => {
            let detected = auto_detect_mds_file()?;
            if !quiet {
                eprintln!("{verb} {}", detected.display());
            }
            Ok(detected)
        }
    }
}
```

**Duplicate `validate_import_path` + `resolve_path` + `self.resolve` pattern across 3 import helpers** - `src/resolver.rs:378-467`
**Confidence**: 80%
- Problem: All three extracted import handlers (`resolve_alias_import`, `resolve_merge_import`, `resolve_selective_import`) start with the same 4-line preamble: `validate_import_path(path)?; let import_path = resolve_path(...); let resolved = self.resolve(...).map_err(...)?;`. While each handler has genuinely different post-resolution logic, the shared preamble is a maintenance risk — updating path validation would require touching all three functions.
- Fix: Extract a `resolve_import_module` helper that performs the shared validation-resolve-error-attachment sequence and returns `Arc<ResolvedModule>`. Each handler then calls the shared helper and only implements its unique scope-mutation logic. This is optional — the current form is still readable and each function is reasonably sized (17, 25, and 45 lines respectively).

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`resolve_output_path` has 6 control-flow branches across 60 lines** - `src/main.rs:129-188`
**Confidence**: 82%
- Problem: While this function was not introduced in this PR, the PR touched it via the mds.json integration. The function implements a 6-step precedence chain, each step with its own early return. The cyclomatic complexity is around 8 (6 return paths + 2 nested conditionals). The function is well-commented and each step is clearly labeled, which mitigates the raw complexity, but it approaches the warning threshold.
- Fix: No immediate change required. The comments documenting each precedence step effectively serve as section headers. If the function grows further (e.g., adding more precedence levels), consider extracting steps 4-5 (directory-based output) into a helper.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`invoke_function` has 57 lines with scope push/restore/pop pattern** - `src/evaluator.rs:165-221`
**Confidence**: 80%
- Problem: `invoke_function` handles recursion detection, arity checking, scope push, captured namespace restoration, captured function restoration, captured var restoration, param binding, evaluation, LIFO validation, scope pop, and error preference — 11 distinct concerns in one function. The double-fault error handling (lines 204-220) adds necessary but non-trivial control flow. This predates the PR but is in a file that was modified.
- Fix: The captured scope restoration (lines 187-198) could be extracted to `scope.restore_captured(&func.captured)` on the `Scope` type. This would reduce `invoke_function` to ~45 lines and move the loop-over-captures logic to where it belongs (the scope module).

**`collect_export` has 47 lines with 3-way match** - `src/resolver.rs:330-376`
**Confidence**: 80%
- Problem: The three export forms (Named, ReExport, Wildcard) each have distinct logic, but the function is borderline long at 47 lines. The Wildcard arm (lines 360-373) includes a collision check loop that adds nesting. This is pre-existing code that was not modified in this PR.
- Fix: Acceptable as-is given the inherent 3-variant dispatch. If export forms grow, consider extracting each arm into its own function (matching the pattern used for import handlers).

## Suggestions (Lower Confidence)

- **File size of `resolver.rs` at 782 lines** - `src/resolver.rs` (Confidence: 70%) — The file contains the `ModuleCache` implementation, `ResolvedModule` methods, 6 free helper functions, and `parse_frontmatter`. While logically cohesive (all module resolution), it is approaching the 800-line mark. Consider whether `ResolvedModule` methods and the free helper functions (`build_cycle_string`, `parse_frontmatter`, `validate_file_type`, etc.) could live in a separate `module.rs` or `resolve_helpers.rs`.

- **File size of `main.rs` at 770 lines (370 non-test)** - `src/main.rs` (Confidence: 65%) — Half the file is tests, which is fine, but the production code at 370 lines includes CLI structs, config structs, output resolution, auto-detection, value parsing, stdin reading, and 3 run functions. The run_build/run_check/run_init extraction was a good step; further extraction of config/output concerns into a separate module would improve navigability.

- **`resolve_selective_import` is the longest of the three import handlers at 45 lines** - `src/resolver.rs:423-467` (Confidence: 65%) — The `not_exported` closure creation (lines 440-448) and the `prompt` vs function branching inside the loop add cyclomatic complexity. This is still within reasonable bounds but is notably longer than its siblings (`resolve_alias_import` at 17 lines, `resolve_merge_import` at 25 lines).

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR's primary complexity contribution is positive: it decomposes a monolithic `run()` into `run_build`/`run_check`/`run_init`, extracts security checks from `canonicalize_and_check` into focused helpers, and splits `resolve_import` into per-variant handlers. The `canonicalize_and_check` orchestrator dropped from ~50 lines to 8 lines, and `resolve_import` from ~70 lines to a 6-line match dispatcher. The `run()` function went from ~150 lines to 18 lines of pure dispatch.

The one blocking HIGH is that `run_build` at 70 lines with 4 nesting levels partially undoes the extraction benefit — it should have its output-writing block extracted to match the clean style of the other decompositions. The MEDIUM findings are genuine but non-critical asymmetries and mild duplication that are typical of a first extraction pass.

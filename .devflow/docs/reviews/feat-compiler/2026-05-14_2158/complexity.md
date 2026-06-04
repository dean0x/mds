# Complexity Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**`collect_definitions_and_imports` is 93 lines with 5-level nesting** - `src/resolver.rs:285`
**Confidence**: 85%
- Problem: This function spans ~93 lines (285-377) and reaches 5 levels of nesting (`fn > for > match > match > for`). The outer `match node` dispatches to `Define`, `Import`, and `Export`, and the `Export` arm contains a second `match` over three export variants, each with its own multi-line body. While the PR improved readability by switching the return type from a tuple to `CollectedDefs`, the function body itself remains above the warning threshold (50 lines) and at the nesting ceiling (5 levels).
- Fix: Extract each top-level match arm into a dedicated helper. The `Node::Export` arm is the most complex (lines 330-371, ~41 lines with its own nested match); extracting it as `collect_export(export, &mut functions, &mut explicit_exports, ctx, warnings)` would drop both function length and max nesting by one level. The `Node::Define` arm (lines 298-326, ~28 lines) is similarly self-contained and benefits from extraction.

**`resolve_output_path` is 71 lines with duplicated directory-creation blocks** - `src/main.rs:111`
**Confidence**: 82%
- Problem: `resolve_output_path` spans 71 lines (111-182) with two near-identical blocks for "derive filename + create dir + join" (steps 4 and 5, lines 136-143 and 161-168). The PR moved `input_path` derivation earlier (good), but did not consolidate the two `create_dir_all` + `derive_output_filename` blocks that share identical structure.
- Fix: Extract a helper like `fn prepare_output_dir(dir: &Path, input_path: Option<&Path>) -> Result<PathBuf, miette::Error>` that handles `derive_output_filename`, `create_dir_all`, and the join. Both step 4 and step 5 would call this helper, reducing `resolve_output_path` to ~40 lines.

### MEDIUM

**Duplicated double-fault error-preservation pattern** - `src/evaluator.rs:200-208` and `src/evaluator.rs:299-307`
**Confidence**: 84%
- Problem: The double-fault `match (result, pop_result)` block is copy-pasted verbatim between `invoke_function` (lines 200-208) and `evaluate_for` (lines 299-307), including the identical 3-line comment. The PR description notes this was intentionally applied as a "double-fault error-preservation pattern", but duplicating 10 lines (comment + match) creates a maintenance hazard -- if the error prioritization logic changes, both sites must be updated in lockstep.
- Fix: Extract a helper function:
  ```rust
  /// Resolve a double-fault: prefer the render error over a scope-pop error.
  fn resolve_double_fault(
      render: Result<String, MdsError>,
      pop: Result<(), MdsError>,
  ) -> Result<String, MdsError> {
      match (render, pop) {
          (Err(render_err), _) => Err(render_err),
          (Ok(_), Err(pop_err)) => Err(pop_err),
          (Ok(s), Ok(())) => Ok(s),
      }
  }
  ```
  Then both call sites become: `resolve_double_fault(result, scope.pop())?`

**`run` function is 150 lines with deeply nested match arms** - `src/main.rs:430`
**Confidence**: 80%
- Problem: `run()` spans 150 lines (430-580) with its three `Commands` match arms. The `Build` arm alone is ~68 lines (433-501). While each arm follows a linear flow, the function exceeds the 50-line warning threshold by 3x. This is pre-existing code, but this PR added changes to the `Check` arm (the `--quiet` flag threading) within this function.
- Fix: Extract each match arm into its own function: `run_build(...)`, `run_check(...)`, `run_init(...)`. This is an incremental improvement that can be done in a follow-up.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`process_module` has 7 parameters** - `src/resolver.rs:242`
**Confidence**: 82%
- Problem: `process_module` takes 7 parameters (`&mut self, source, file_str, base_dir, is_md, runtime_vars, warnings`). The PR introduced `ModuleCtx` to bundle 4 of these (`file_str, source, base_dir, runtime_vars`) but only uses it in `collect_definitions_and_imports`, not in `process_module` itself. The 7-parameter count exceeds the warning threshold (5).
- Fix: Construct `ModuleCtx` at the top of `process_module` and thread it through, then change the `process_module` signature to accept `ModuleCtx` instead of the 4 individual fields. This reduces the parameter count to 4 (`&mut self, module_ctx, is_md, warnings`).

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`resolve_import` has 3-way match with repetitive path validation** - `src/resolver.rs:380`
**Confidence**: 80%
- Problem: All three `ImportDirective` arms start with `validate_import_path(path)?; let import_path = resolve_path(ctx.base_dir, path); let resolved = self.resolve(...)`. This common preamble is repeated 3 times across 78 lines. The `Selective` arm also reaches 5-level nesting with the inner `if name == "prompt"` branch.
- Fix: Hoist the common preamble above the match (extract path + alias from the variant first), and consider extracting the `Selective` import handling into a helper.

## Suggestions (Lower Confidence)

- **`canonicalize_and_check` comment density** - `src/resolver.rs:73` (Confidence: 65%) -- The 15-line strategy comment (lines 77-90) is longer than the code it describes. The comment is helpful for posterity but could be condensed to ~5 lines without losing clarity.

- **`invoke_function` linear complexity** - `src/evaluator.rs:152` (Confidence: 70%) -- At 57 lines, this function is at the warning threshold. The four sequential `for` loops restoring captured scope (lines 174-187) are straightforward but could be extracted into a `restore_captured_scope(func, scope)` helper to make the function's control flow (guards -> setup -> call -> teardown) more scannable.

- **`exit_code_resource_limit` test builds 2002-element YAML in a loop** - `tests/integration.rs:3027` (Confidence: 62%) -- The test constructs frontmatter with two 1001-element arrays via a loop, producing ~15KB of YAML. A `vec!["x"; 1001].join("\n")` pattern or a helper function would be more concise, though this is a test so readability standards are relaxed.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR is a net improvement for complexity: it decomposes `validate_and_read_file` into two focused functions (`canonicalize_and_check` + `read_validated_file`), replaces a bare tuple return with a named `CollectedDefs` struct, introduces `ModuleCtx` to bundle context parameters, and uses `IndexSet::pop()` instead of `shift_remove()` for clearer LIFO semantics. The two HIGH findings (function length in `collect_definitions_and_imports` and duplication in `resolve_output_path`) are real complexity concerns but not merge-blocking on their own -- they represent opportunities for follow-up decomposition rather than defects.

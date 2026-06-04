# Consistency Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**Diff**: `git diff 97b478f...HEAD` (15 files, +2475/-887 lines)

## Issues in Your Changes (BLOCKING)

### HIGH

**Error message version references inconsistently removed** - `src/value.rs:60`, `src/value.rs:92`
**Confidence**: 82%
- Problem: The YAML and JSON "object/map types are not supported" error messages had their `"in MDS v0.1"` suffix removed, while the parser retains `"in v0.1"` in two other user-facing error messages (`src/parser.rs:217`, `src/parser.rs:474`). Error messages that reference version scope should be consistent across the codebase -- either all include the version qualifier or none do.
- Fix: Either restore `"in MDS v0.1"` in `value.rs`, or remove the `v0.1` references from `parser.rs` to match. Given that these are user-facing error strings and the parser messages explicitly reference version-specific limitations (dot notation, negation in @if), keeping the version qualifier everywhere is the safer choice.

## Issues in Code You Touched (Should Fix)

_No issues meeting the 80% confidence threshold._

## Pre-existing Issues (Not Blocking)

_No critical pre-existing issues identified._

## Suggestions (Lower Confidence)

- **`load_config` has no file size guard** - `src/main.rs:51` (Confidence: 65%) -- `load_config` calls `read_to_string` on `mds.json` without a file size check. The resolver enforces `MAX_FILE_SIZE` on `.mds` files, and `load_vars_file` checks `MAX_FILE_SIZE` on vars JSON files. A malicious or accidental multi-MB `mds.json` would be read into memory unchecked. The risk is low because `mds.json` is a local project config file, but the pattern diverges from the rest of the codebase's file-reading discipline.

- **`load_config` does not check for symlinks** - `src/main.rs:50` (Confidence: 62%) -- `validate_and_read_file` in the resolver rejects symlinks in the final path component. `load_config` uses `is_file()` which follows symlinks without any symlink check. This is arguably fine since `mds.json` is a project config and not user-supplied import paths, but it is a pattern deviation from the import path's security posture.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Consistency Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Consistency Observations

This PR demonstrates strong consistency discipline across a large refactoring:

1. **Error constructor migration**: All `MdsError` struct-literal construction (`MdsError::Io { message: ... }`) has been consistently migrated to constructor methods (`MdsError::io(...)`) across all source files. No mixed patterns remain.

2. **`serde_yaml` to `serde_yml` migration**: Complete and clean. Zero leftover `serde_yaml` references in source, tests, or Cargo.toml.

3. **`Arc<FunctionDef>` adoption**: Consistently applied across all storage layers (`Frame::functions`, `NamespaceScope::functions`, `ResolvedModule::functions`) with owned `FunctionDef` in `CapturedScope::functions` to break reference cycles. The feature knowledge pattern is followed precisely.

4. **`EvalContext` parameter bundling**: All evaluator internal functions consistently take `ctx: &mut EvalContext` rather than the previous three separate parameters. No mixed patterns remain.

5. **`IndexSet` for cycle detection**: Clean replacement of the previous `HashSet + Vec` pair. `shift_remove` preserves insertion order as documented.

6. **`CapturedScope` struct adoption**: All access to captured closure state consistently uses `func.captured.namespaces` / `func.captured.functions` / `func.captured.vars` instead of the old flat fields. No leftover `captured_namespaces` references.

7. **`mds_bin()` helper in tests**: All CLI integration tests use the `mds_bin()` helper consistently; no raw `Command::new(env!(...))` calls remain outside the helper definition.

8. **`mds::Value` path migration**: All integration tests consistently use `mds::Value` instead of the previous `mds::value::Value` path.

9. **`#[must_use]` on new public API**: The new `check_collecting_warnings` and `check_str_collecting_warnings` functions carry `#[must_use = "warnings should be used"]`, matching the pattern of existing collecting variants.

10. **scope.pop() returns Result consistently**: All callers use `scope.pop()?` with no bare `.unwrap()` calls on pop in non-test code.

11. **`debug_assert!` vs `expect` for invariants**: The `scope.rs` methods use `expect("BUG: ...")` for the frames-nonempty invariant (appropriate since it is load-bearing), while `evaluator.rs` uses `debug_assert!` for the LIFO call-stack invariant (appropriate since it is a sanity check in a hot path). Both patterns are deliberately chosen and documented.

12. **CLI error pattern**: `main.rs` functions (`load_config`, `resolve_output_path`, `auto_detect_mds_file`) consistently use `miette::miette!()` for CLI-layer errors, while library code uses `MdsError`. This separation is intentional and well-documented.

13. **`-o -` for stdout**: CLI tests that previously relied on default stdout behavior have been consistently updated to pass `-o -` where stdout output is expected, matching the new default-to-file behavior.

14. **Resolver decomposition**: The `process_module` refactoring into `build_scope_from_frontmatter`, `collect_definitions_and_imports`, `validate_exports`, and `validate_and_read_file` follows the existing naming conventions and responsibility boundaries.

### Condition for Approval

Address the HIGH-severity error message version reference inconsistency (either add `"in MDS v0.1"` back to `value.rs` or remove it from `parser.rs`).

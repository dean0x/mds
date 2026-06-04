# Consistency Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Remaining evaluator limits not consolidated into limits.rs** - `crates/mds-core/src/evaluator.rs:11-24`
**Confidence**: 82%
- Problem: The PR consolidates 5 cross-module constants into `limits.rs` as a single source of truth. However, 5 additional `MAX_*` constants remain in `evaluator.rs` (`MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`, `MAX_WARNINGS`), plus 1 in `value.rs` (`MAX_VALUE_DEPTH`), 1 in `fs.rs` (`MAX_PATH_SEGMENTS`), and 1 in `resolver.rs` (`MAX_IMPORT_DEPTH`). These are currently module-private and only used locally, so consolidation is not strictly necessary, but the SECURITY.md table references them all as a unified set of defense-in-depth limits. Having some limits in `limits.rs` and others scattered creates an inconsistency in the "single source of truth" concept. The PR description accurately says "5 cross-module constants" which is correct -- only these 5 were cross-module. This is informational only.

## Suggestions (Lower Confidence)

- **Unused pub(crate) re-export** - `crates/mds-core/src/parser.rs:13` (Confidence: 65%) -- `is_valid_identifier` is re-exported as `pub(crate)` from the parser module but is not currently consumed by any module outside `parser`. The export may be anticipating future usage, but if not, it could be reduced to module-private scope.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Consistency Score**: 9/10
**Recommendation**: APPROVED

### Rationale

This is a well-executed structural refactoring with strong consistency properties:

1. **Constant consolidation** -- The 5 cross-module constants (`MAX_NESTING_DEPTH`, `MAX_ELSEIF_BRANCHES`, `MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`, `MAX_DOT_SEGMENTS`) are correctly consolidated into `limits.rs`. All import paths across `parser.rs`, `evaluator.rs`, `fs.rs`, `validator.rs`, and `lib.rs` are updated consistently. No stale references remain.

2. **Module extraction pattern** -- The `#[path = "parser_helpers.rs"]` and `#[cfg(test)] #[path = "parser_tests.rs"]` pattern is consistent with the existing `error.rs` / `error_tests.rs` precedent in this codebase.

3. **Visibility modifiers** -- All extracted helpers use `pub(super)` consistently, except `is_valid_identifier` which uses `pub(crate)` and is explicitly re-exported from `parser.rs` -- appropriate for a function intended for crate-wide use.

4. **Section headers** -- The `// -- Section --` style added to `limits.rs` matches the existing convention used in `error.rs`, `error_tests.rs`, and `options.rs`.

5. **Doc comments** -- All doc comments from the original `parser.rs` are preserved verbatim in the extracted files.

6. **SECURITY.md updates** -- Location references for `MAX_FILE_SIZE` and `MAX_NESTING_DEPTH` correctly updated from their old modules to `limits.rs`.

7. **Signature cleanup** -- The removal of the unused `_offset` parameter from `parse_export_directive` is a valid cleanup (the `ExportDirective` AST type has no `offset` field, unlike `ImportDirective`). This does not create an inconsistency -- it removes one.

8. **Test extraction** -- All 668 lines of tests moved to `parser_tests.rs` with correct `use super::helpers::*` and `use super::*` imports, matching the `error_tests.rs` pattern.

Applies ADR-002: The PR claims to close #35 (limits consolidation) and #36 (parser split). The diff confirms both are substantively addressed -- constants are consolidated and parser.rs is reduced from ~1820 to ~423 lines.

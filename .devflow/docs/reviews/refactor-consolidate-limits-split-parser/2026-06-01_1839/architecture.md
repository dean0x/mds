# Architecture Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01
**PR**: #52

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Evaluator limits could eventually join limits.rs** - `evaluator.rs:11-24` (Confidence: 60%) -- Five evaluator-private constants (`MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`, `MAX_WARNINGS`) remain module-local. They are not cross-module today, so leaving them is reasonable, but a future follow-up could consolidate them to make `limits.rs` the exhaustive single source of truth for all runtime limits. Same applies to `MAX_IMPORT_DEPTH` in `resolver.rs`, `MAX_VALUE_DEPTH` in `value.rs`, and `MAX_PATH_SEGMENTS` in `fs.rs`.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 9/10
**Recommendation**: APPROVED

## Analysis

This PR executes two well-scoped refactoring tasks cleanly:

### 1. Limits Consolidation (Closes #35)

Five cross-module constants (`MAX_DOT_SEGMENTS`, `MAX_NESTING_DEPTH`, `MAX_ELSEIF_BRANCHES`, `MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`) were scattered across `ast.rs`, `parser.rs`, and `resolver.rs`. They are now centralized in `limits.rs` as a single source of truth. All consumers (`evaluator.rs`, `fs.rs`, `validator.rs`, `parser.rs`, `lib.rs`) import from `limits` instead.

**Architectural strengths:**
- Correct use of `pub(crate)` visibility -- constants are accessible within the crate but not leaked to external consumers. `lib.rs` re-exports the two constants (`MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`) that the CLI binary needs via explicit `pub const` wrappers with documentation.
- Module-private constants (e.g., `MAX_CALL_DEPTH` in `evaluator.rs`) were correctly left in place since they have only one consumer. This avoids over-centralization.
- Pinning tests in `limits.rs` guard against accidental value changes.

### 2. Parser Split (Closes #36)

`parser.rs` was reduced from ~1820 lines to ~423 lines by extracting:
- `parser_helpers.rs` (~733 lines) -- stateless helper functions (condition parsing, argument parsing, import/export directive parsing, string unescaping, identifier validation, newline stripping).
- `parser_tests.rs` (~668 lines) -- all parser tests.

**Architectural strengths:**
- Clean separation of concerns: `parser.rs` retains the `Parser` struct and its stateful methods (the recursive-descent parser with `self.pos`, `self.depth`). `parser_helpers.rs` contains only stateless free functions that take inputs and return results, with no access to parser state. This aligns with the Single Responsibility Principle (applies ADR-001 -- the split keeps each file focused on a single concern).
- The `#[path = "..."]` attribute makes helpers and tests submodules of `parser`, preserving `pub(super)` visibility for helper functions. This prevents helpers from leaking into the broader crate namespace while remaining accessible to the parser and its tests.
- `is_valid_identifier` is correctly re-exported as `pub(crate)` via `pub(crate) use helpers::is_valid_identifier` since it is used by the validator module.
- The `parse_export_directive` signature was cleaned up to drop the unused `offset` parameter, which is correct since `ExportDirective` variants have no `offset` field.
- No behavioral changes: all 591 tests pass, clippy is clean, formatting is clean.

### Dependency Direction

All dependency arrows are correct and point inward:
- `parser.rs` -> `limits` (for constants)
- `parser_helpers.rs` -> `limits` (for `MAX_DOT_SEGMENTS`, `MAX_NESTING_DEPTH`)
- `evaluator.rs` -> `limits` (for `MAX_DOT_SEGMENTS`, `MAX_ELSEIF_BRANCHES`)
- `fs.rs` -> `limits` (for `MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`)
- `validator.rs` -> `limits` (for `MAX_NESTING_DEPTH`)
- `lib.rs` -> `limits` (for public re-exports)

No circular dependencies. No layering violations.

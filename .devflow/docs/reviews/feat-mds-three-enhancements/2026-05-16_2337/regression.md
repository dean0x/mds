# Regression Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16T23:37

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

- **Error message format change** - `src/evaluator.rs:118` (Confidence: 65%) — The error message changed from `"field '{field}' not found on object '{root}'"` to `"field '{field}' not found on '{path_so_far}'"`. While no tests match on this string and the new message is strictly more informative (showing the full traversed path), any downstream consumer parsing error messages by text would break. This is low-risk given the library is young and errors are structured types.

- **strip_type_mds broadened matching** - `src/lib.rs:353-355` (Confidence: 60%) — Previously only `type: mds` (unquoted) was stripped from output frontmatter. Now `type: "mds"` and `type: 'mds'` are also stripped. This is an intentional fix, but a user who relied on quoted `type: "mds"` surviving into output would see different behavior. Extremely unlikely in practice since the whole purpose of this key is compiler detection.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

### Changes Reviewed

1. **evaluator.rs** — Replaced `assert!()` panics with `Result` returns for empty condition/iterable paths. Extracted `run_loop_body()` and `evaluate_for_key_value()` helpers. Added `MAX_DOT_SEGMENTS` depth guard. Improved error diagnostics with path tracking.

2. **parser.rs** — Added `MAX_DOT_SEGMENTS` constant (32). Added segment count guards to all dot-path parsing locations. Extracted `parse_dot_expr` helper from `parse_interpolation_expr`.

3. **lib.rs** — Extended `strip_type_mds` to handle quoted YAML variants (`"mds"`, `'mds'`).

4. **tests/integration.rs** — Strengthened assertions from `assert!(contains(...))` to `assert_eq!`. Added new tests for runtime-supplied objects and key-value dot-path iteration.

### Regression Checklist

- [x] No exports removed — public API unchanged
- [x] Return types backward compatible — all public functions retain same signatures
- [x] Default values unchanged — no behavioral defaults modified
- [x] Side effects preserved — no events/logging removed
- [x] All consumers of changed code updated — internal refactoring only
- [x] Migration complete across codebase — no old API patterns remaining
- [x] Commit messages match implementation — assert-to-Result, dot-segment guard, quoted YAML stripping all verified in code
- [x] All 349 tests pass

### Key Observations

The refactoring from `assert!()` to `Result` is a reliability improvement — the old code would panic in release builds if the parser invariant was ever violated. The new code returns a proper error. This is not a regression; it is strictly safer behavior for the same logical condition.

The `run_loop_body` and `evaluate_for_key_value` extractions are semantically equivalent to the inlined code they replace. The iteration order, scope management, error preference logic, and bounds checking are all preserved identically.

The test changes from `assert!(contains(...))` to `assert_eq!(...)` are stricter and would surface any regressions in output format that the previous weaker assertions would have missed.

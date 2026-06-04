# Regression Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**`debug_assert` in resolver vs `assert` in evaluator for LIFO invariants** - `src/resolver.rs:204`, `src/evaluator.rs:196`
**Confidence**: 82%
- Problem: The evaluator promotes its call-stack LIFO check from `debug_assert!` to `assert!` (lines 196-198) with a documented rationale ("cost is negligible at MAX_CALL_DEPTH = 128"). The resolver uses `debug_assert_eq!` (line 204) for a structurally identical LIFO invariant on `self.resolving`. Both invariants guard against silent corruption (recursion detection in evaluator, cycle detection in resolver). If the evaluator's invariant is safety-critical enough for release-mode enforcement, the resolver's is too -- a non-LIFO pop in the resolver would corrupt the cycle-detection set, potentially allowing circular imports to pass silently.
- Fix: Promote `debug_assert_eq!` on line 204 to `assert_eq!` for consistency with the evaluator pattern, or document why the resolver invariant is less critical.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`resolve_output_path` input=None + config.output_dir path is now unreachable** - `src/main.rs:131` (Confidence: 65%) -- The refactoring changed the early-return condition from `is_stdin && out_dir.is_none()` to `input_path.is_none() && out_dir.is_none()`. This broadens the guard to also cover `input=None` (not just stdin). When `input=None`, `out_dir=None`, but `config` has an `output_dir`, the old code would fall through to step 5 and write to the config's output directory; the new code returns stdout at step 3. In practice the call site always passes `Some(input)`, so this path is unreachable today. However, if `resolve_output_path` is ever called with `input=None` by a future caller (e.g. a library API), the behavior would differ from the old code. Consider adding a comment or a unit test documenting this narrowed precondition.

- **Error message string change could break external tooling that matches on exact text** - `src/value.rs:60`, `src/value.rs:92` (Confidence: 62%) -- The error messages changed from `"object/map types are not supported"` to `"object/map types are not supported in MDS v0.1"`. Internal tests use substring matching and pass fine, but any external tool or script that matches on the exact old string would break. Low risk given the project's maturity stage.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The changes are well-structured with no lost exports, no deleted files, no removed public APIs, and no signature changes. All 280 tests pass. The core refactorings -- resolver decomposition (`validate_and_read_file` split into `canonicalize_and_check` + `read_validated_file`), `shift_remove` to `pop`, `debug_assert` to `assert` promotion in evaluator, double-fault error priority change, and `CollectedDefs` type alias to struct -- are all safe transformations with correct semantics. The intentional double-fault behavior change (preferring render errors over pop errors) is well-documented and is an improvement over the old behavior. The one condition for approval: consider promoting the resolver's `debug_assert_eq` to `assert_eq` for consistency with the evaluator's LIFO enforcement pattern, or document the rationale for the difference.

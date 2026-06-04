# Regression Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

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

- **`length()` returns byte count, not character count for strings** - `crates/mds-core/src/builtins.rs:341` (Confidence: 65%) -- `s.len()` returns byte length, which differs from character count on multi-byte strings (e.g., `length("café")` returns 6, not 5). The spec documents this as "String byte length" so it is intentional, but users familiar with JavaScript/Python may expect character count. Not a regression since this is new functionality, but worth considering for a future `char_count` builtin or a doc note.

- **`sort()` NaN ordering is non-deterministic** - `crates/mds-core/src/builtins.rs:414-415` (Confidence: 60%) -- `partial_cmp` returns `None` for NaN comparisons, mapped to `Ordering::Equal`. If an array contains NaN values, sort order is implementation-defined. Not a regression (new code), and the parser rejects NaN literals, but NaN could theoretically appear from arithmetic in future versions.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Analysis Details

### Regression Checklist

- [x] **No exports removed** -- No `pub` exports were removed. New exports added: `BuiltinError` variant, `Param` struct, `builtins` module, `condvalue_to_value`, `required_param_count`.
- [x] **Return types backward compatible** -- `call_function` changed from `Result<String>` to `Result<Value>` (internal `fn`, not public API). All call sites properly updated: `evaluate_expr` calls `.to_string()`, `resolve_args` preserves the `Value` (improvement: nested builtin calls now preserve type rather than coercing to string).
- [x] **Default values unchanged or documented** -- `ArityMismatch.expected` changed to `expected_min`/`expected_max` (breaking internal struct change). All 5 call sites updated. Error messages now show range format (e.g., "expected 1-3 arguments") which is a superset of the old single-value format.
- [x] **Side effects preserved** -- No logging, events, or side effects removed.
- [x] **All consumers of changed code updated** -- `FunctionDef.params` changed from `Vec<String>` to `Vec<Param>`. All 7 construction sites updated to `Param::required()`. All access sites updated to use `param.name` or `required_param_count()`.
- [x] **Migration complete across codebase** -- grep confirms zero remaining `Vec<String>` param patterns. All test files, scope.rs, evaluator.rs, validator.rs, and api_surface.rs updated.
- [x] **Commit message matches implementation** -- 4 commits each accurately describe their changes. PR closes #53, #54, #55 as stated (applies ADR-002, applies ADR-008).
- [x] **Breaking changes documented** -- Spec updated to v0.2 with new condition forms in section 4.3, default args and built-in functions in section 4.5.
- [x] **No files deleted** -- No files removed from the repository.
- [x] **690 tests pass** -- All existing tests continue to pass. ~100 new tests added for the three features.

### Key Structural Changes Verified

1. **`Condition` enum extended** with `And(Vec<Condition>)` and `Or(Vec<Condition>)`. The `path()` method returns `&[]` for compound variants; `root()` returns `Err`. Both `evaluate_condition` (evaluator) and `validate_condition` (validator) handle compound variants by recursing into operands before reaching leaf conditions, so `path()`/`root()` are never called on compound variants in practice.

2. **`Arg` enum extended** with `NumberLiteral(f64)`, `BooleanLiteral(bool)`, `NullLiteral`. All match arms in evaluator (`resolve_args`) and validator (`validate_var_args`) updated to handle new variants.

3. **`parse_condition` refactored** to a three-layer parser: `parse_condition` -> `parse_and_level` -> `parse_simple_condition`. Existing single-condition parsing delegates through to `parse_simple_condition` unchanged. Quote-aware splitting ensures `||`/`&&` inside strings are not treated as operators (verified by test).

4. **`parse_define_block` refactored** to use `parse_define_params` with quote-aware comma splitting. The old inline parsing logic (comma split, identifier validation, duplicate check) is preserved in the new function with added default-value support.

5. **Built-in function resolution** follows user-defined-first precedence in both evaluator and validator, ensuring shadowing works correctly (verified by `builtin_shadowed_by_user_function` test).

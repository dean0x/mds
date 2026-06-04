# Consistency Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Inconsistent error message wording for MAX_DOT_SEGMENTS limit** - `src/evaluator.rs:105`, `src/parser.rs:226,286,551,728`
**Confidence**: 85%
- Problem: The evaluator uses the phrasing "dot path depth exceeds maximum of {N} segments" while the parser uses "exceeds maximum segment count of {N}" (four occurrences). Within the parser itself, consistency is maintained, but across the evaluator/parser boundary the same logical guard uses different wording.
- Fix: Align all five messages to one phrasing. The parser's "exceeds maximum segment count of {N}" reads more naturally:
```rust
// evaluator.rs:104-106
return Err(MdsError::syntax(format!(
    "dot path exceeds maximum segment count of {MAX_DOT_SEGMENTS}"
)));
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Parser `@if`/`@for` dot-segment errors could use `syntax_at`** - `src/parser.rs:225,285` (Confidence: 65%) -- The `parse_if_block` and `parse_for_block` methods have access to `self.file`, `self.source`, and `offset`, yet use the bare `MdsError::syntax` for the new dot-segment limit errors. `parse_dot_expr` (which handles interpolation) uses `syntax_at` for the same guard. However, the pre-existing pattern in all `Parser` struct methods is to use bare `syntax`, so this is defensible as-is.

- **`run_loop_body` takes `bindings: &[(&str, Value)]` which requires `Value` cloning for key-value pairs** - `src/evaluator.rs:353` (Confidence: 62%) -- The `Value::String(key)` is constructed and passed by value into the bindings slice, then immediately `clone()`-ed inside `run_loop_body`. For the key-value path this is optimal (keys are already owned), but for the array path, `item` is already an owned `Value` that gets re-cloned. A `Vec<(&str, Value)>` taken by value (consuming the bindings) could avoid one clone per iteration. Minor optimization concern, not a consistency issue per se.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The branch demonstrates strong consistency overall:
- The `.first().ok_or_else()` pattern replacing `assert!()` is applied uniformly in both `evaluate_if` and `evaluate_for`, matching the documented codebase idiom.
- `MAX_DOT_SEGMENTS` is defined once in `parser.rs` as `pub(crate)` and imported in `evaluator.rs`, following the single-definition pattern used by `MAX_NESTING_DEPTH` and `MAX_TRAVERSAL_DEPTH`.
- `run_loop_body` properly deduplicates the push/set/evaluate/pop/prefer-first-error sequence that was previously repeated.
- `parse_dot_expr` extraction follows the pattern of other standalone parser helpers (`parse_for_vars`, `parse_interpolation_expr`, `parse_import_directive`).
- `strip_type_mds` handles all three YAML quoting variants consistently.
- Test style (doc comments, assertion messages, naming) matches existing tests.
- The one MEDIUM finding is a minor cross-module error message wording inconsistency that does not affect behavior.

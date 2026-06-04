# Reliability Review Report

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

(none -- no reliability concerns at any confidence level)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Reliability Score**: 10
**Recommendation**: APPROVED

## Rationale

This PR is a pure structural refactoring with no behavioral changes, and it is clean from a reliability perspective. Specific findings:

### Bounded Iteration -- All loops terminate

- `parse_body` (parser.rs:125): bounded by `self.tokens.len()` with every branch advancing `self.pos` by at least 1.
- `collect_elseif_branches` (parser.rs:273): bounded by `MAX_ELSEIF_BRANCHES` (256) with explicit limit check before parse work.
- `find_unquoted_operator` (parser_helpers.rs:116): bounded by `bytes.len()` with `i` advancing by at least 1 per iteration.
- `parse_args_inner` (parser_helpers.rs:559): iterates over `args_str.chars()` (finite). Recursion depth bounded by `MAX_NESTING_DEPTH` (64).
- `unescape_string` (parser_helpers.rs:666): iterates over `s.chars()` (finite).

### Assertion Density -- Invariants checked

- `enter_block` enforces `MAX_NESTING_DEPTH` before recursive parse.
- `parse_args_inner` enforces `MAX_NESTING_DEPTH` before recursive call dispatch.
- `validate_dot_path_parts` enforces `MAX_DOT_SEGMENTS` on all dot-path inputs.
- `parse_dot_path` enforces `MAX_DOT_SEGMENTS` segment-by-segment.
- Evaluator retains `debug_assert!` on `MAX_ELSEIF_BRANCHES` as defense-in-depth (evaluator.rs:381).
- Pinning tests in `limits.rs` assert all 5 constants have their expected values, preventing silent drift.

### Allocation Discipline -- No concerns

- `Vec::with_capacity(4)` used for `collect_elseif_branches` and `parse_dot_path` -- appropriate small pre-allocations.
- `String::with_capacity(s.len())` used in `unescape_string` -- exact pre-sizing.
- No unbounded allocations in hot paths.

### Indirection Limits -- Clean

- No nested Box, no pointer-to-pointer patterns. All data is flat structs and Vecs.

### Metaprogramming Restraint -- Clean

- No macros, no recursive generics, no reflection. All types are concrete and bounded.

### Constants Consolidation (applies ADR-001)

The move of 5 cross-module constants (`MAX_NESTING_DEPTH`, `MAX_ELSEIF_BRANCHES`, `MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`, `MAX_DOT_SEGMENTS`) into `limits.rs` as single source of truth is a reliability improvement. Previously these constants were scattered across `ast.rs`, `parser.rs`, and `resolver.rs`, creating drift risk. Module-local constants (`MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`, `MAX_WARNINGS`, `MAX_PATH_SEGMENTS`, `MAX_IMPORT_DEPTH`, `MAX_VALUE_DEPTH`) correctly remain in their defining modules since they are not shared.

### Parser Split Preserves All Bounds

The extraction of helpers into `parser_helpers.rs` and tests into `parser_tests.rs` preserves all resource limit checks identically. Every `MAX_*` guard that existed in the original monolithic `parser.rs` is present in the refactored code at the same logical locations. The `#[path = "..."]` module attribute correctly scopes `pub(super)` visibility to the parent `parser` module.

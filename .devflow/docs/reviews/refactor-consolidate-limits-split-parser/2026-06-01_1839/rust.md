# Rust Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01
**PR**: #52

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`strip_leading_newline` / `strip_trailing_newline` mutate `Vec` parameter** - `parser_helpers.rs:703,720` (Confidence: 65%) -- These functions accept `mut nodes: Vec<Node>` and mutate in place (`.remove(0)`, `.pop()`). While idiomatic Rust for ownership transfer, the `remove(0)` in `strip_leading_newline` is O(n). For small template bodies this is negligible, but a `VecDeque` or draining iterator would be more efficient for large bodies. Low practical impact given typical template sizes.

- **`parse_args_inner` uses char-by-char iteration for argument splitting** - `parser_helpers.rs:559` (Confidence: 60%) -- The function iterates character-by-character to split arguments while tracking string and paren state. This is correct and necessary for the grammar, but the `String` accumulation via `push(ch)` could be replaced with byte-range tracking for zero-copy slicing, reducing allocations. Low priority given this is a parser for human-authored templates.

- **`#[path = ...]` module attribute for co-located files** - `parser.rs:11,17` (Confidence: 70%) -- Using `#[path = "parser_helpers.rs"]` and `#[path = "parser_tests.rs"]` works correctly but is unconventional when the files are already in the same directory. Standard Rust module structure would use `mod helpers;` with `helpers.rs` or `helpers/mod.rs`. The `#[path]` attribute is typically reserved for non-standard layouts. This is a style choice rather than a correctness issue, and the PR description indicates this was a deliberate decision for the split.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

This is a clean structural refactoring (applies ADR-001 -- squash merge with pre-merge gate). The PR achieves its stated goals:

1. **Constants consolidation**: Five cross-module constants (`MAX_NESTING_DEPTH`, `MAX_ELSEIF_BRANCHES`, `MAX_FILE_SIZE`, `MAX_TRAVERSAL_DEPTH`, `MAX_DOT_SEGMENTS`) are now centralized in `limits.rs` as a single source of truth. Import paths across `evaluator.rs`, `fs.rs`, `lib.rs`, `parser.rs`, `resolver.rs`, and `validator.rs` are updated consistently.

2. **Parser split**: `parser.rs` shrinks from ~1820 lines to 423 lines by extracting free functions to `parser_helpers.rs` (733 lines) and all tests to `parser_tests.rs` (668 lines). The `Parser` struct and its methods stay in `parser.rs`, while standalone parsing utilities move to helpers.

3. **No behavioral changes**: All 591 tests pass. Clippy reports zero warnings. Formatting is clean. The `parse_export_directive` signature correctly removes the previously unused `_offset` parameter (confirmed: `ExportDirective` variants do not store offset).

4. **Visibility is well-scoped**: Extracted helpers use `pub(super)` (visible only to `parser.rs`), with `is_valid_identifier` re-exported as `pub(crate)` for cross-module use. Constants use `pub(crate)` consistently.

5. **Pinning tests in `limits.rs`**: The `limits_have_expected_values` test pins all constant values, providing regression protection against accidental changes to safety limits.

Ownership and borrowing patterns are correct throughout the new files. Error handling consistently uses `Result` with `MdsError` -- no panics in non-test code (the single `.expect()` in `collect_elseif_branches` is guarded by a loop condition that guarantees the prefix exists). No `unsafe` code. No `.unwrap()` outside tests.

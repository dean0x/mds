# Rust Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**Files reviewed**: 7 Rust files, +1040 -76 lines
**Prior resolutions**: Cycle 2 resolved 19 issues (19 fixed, 0 FP, 0 deferred). collect_elseif_branches extracted, #[must_use] added on Condition::root(), CondValue::Bool->Boolean rename, stale comments fixed.

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`PartialEq` derive on `CondValue` includes `f64` -- NaN comparison inconsistency** - `crates/mds-core/src/ast.rs:17`
**Confidence**: 82%
- Problem: `CondValue` derives `PartialEq`, which means `CondValue::Number(f64::NAN) != CondValue::Number(f64::NAN)` at the type level (Rust's `f64` `PartialEq` returns false for NaN). This is consistent with the runtime `values_equal` function, but `CondValue` is used in the `Condition::Eq` and `Condition::NotEq` variants. If `CondValue` is ever compared structurally (e.g., deduplication, caching, or test assertions using `assert_eq!` on AST nodes), NaN-bearing values will behave unexpectedly. The parser already rejects NaN/Infinity literals, so this cannot currently be triggered by user input -- but the `PartialEq` derive on an f64-containing type is a latent hazard if the invariant weakens.
- Fix: This is acceptable given the parser's `is_finite()` guard (line 493), but consider adding a doc comment on the `Number(f64)` variant noting that `PartialEq` assumes the parser has excluded non-finite values:
```rust
/// A numeric literal: `42`, `3.14`, `-5`
///
/// Invariant: the parser rejects NaN and Infinity, so this always holds a
/// finite f64. `PartialEq` correctness depends on this invariant.
Number(f64),
```

**`find_unquoted_operator` scans raw bytes -- safe for ASCII operators only** - `crates/mds-core/src/parser.rs:515-562`
**Confidence**: 80%
- Problem: `find_unquoted_operator` operates on `s.as_bytes()` directly, scanning byte-by-byte. This is safe because the operators (`==`, `!=`) and quote characters (`"`, `'`, `\\`) are all single-byte ASCII, and the function only compares against ASCII byte values. However, if multi-byte UTF-8 content appears inside string literals, the `i += 2` escape-skip (line 530) could theoretically land mid-codepoint. In practice this is safe because: (a) the backslash `\` is always a single byte, and (b) the next byte after a backslash in a multi-byte sequence cannot be `"`, `'`, `!`, or `=` since those are all ASCII-range bytes that cannot appear as continuation bytes in valid UTF-8. The code is correct but the reasoning is non-obvious.
- Fix: Add a brief safety comment explaining why byte-level scanning is sound for UTF-8 input:
```rust
// SAFETY: All operators and delimiters are ASCII single-byte characters.
// UTF-8 continuation bytes (0x80..0xBF) cannot collide with any of
// '=', '!', '"', '\'', or '\\', so byte-level scanning is sound.
fn find_unquoted_operator(s: &str) -> Option<(usize, &'static str)> {
```

## Issues in Code You Touched (Should Fix)

_(none)_

## Pre-existing Issues (Not Blocking)

_(none -- no critical pre-existing issues in reviewed Rust files)_

## Suggestions (Lower Confidence)

- **`parse_cond_value` single-quote string matching edge case** - `crates/mds-core/src/parser.rs:464-465` (Confidence: 65%) -- A string like `"'"` (length 1, starts and ends with `'`) would match both the double-quote and single-quote branches due to the `||` structure, but the double-quote branch handles it correctly since it checks `starts_with('"')` first. The `s.len() >= 2` guard prevents a panic on single-char input. This is correct but the overlapping structure could be clearer with an early `match s.as_bytes()[0]` dispatch.

- **`unreachable!()` in `parse_condition` match arm** - `crates/mds-core/src/parser.rs:612` (Confidence: 70%) -- The `_ => unreachable!()` arm in the operator match could theoretically panic if `find_unquoted_operator` is extended to return new operators in the future. Consider using an explicit error return instead for defense-in-depth, though as long as `find_unquoted_operator` only returns `"=="` and `"!="` this is fine.

- **`Condition` does not derive `PartialEq`** - `crates/mds-core/src/ast.rs:30` (Confidence: 60%) -- `Condition` derives `Debug, Clone` but not `PartialEq`, while `CondValue` does derive `PartialEq`. This means AST-level comparison/testing of `Condition` variants requires pattern matching rather than `==`. If `PartialEq` were added, `Condition::Eq` and `Condition::NotEq` variants containing `CondValue` with `f64` would inherit the NaN pitfall. Not deriving it is actually safer, but worth noting the intentional asymmetry.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This is a well-structured PR that adds significant language features (negation, equality operators, @elseif) to the MDS template engine. The Rust code quality is high:

- **Ownership and borrowing**: Proper use of references throughout. `parse_condition`, `parse_cond_value`, `find_unquoted_operator` all accept `&str` rather than `String`. The evaluator borrows `&Condition` and `&CondValue` without unnecessary cloning.
- **Error handling**: Consistent use of `Result` with `MdsError` propagation via `?`. No `.unwrap()` in library code. Error messages are specific and actionable (e.g., "expected variable name after '!'", "use '==' for comparison, not '='").
- **Type-driven design**: The `Condition` enum with four variants (Truthy, Not, Eq, NotEq) makes illegal states unrepresentable. The `CondValue` enum restricts RHS literals to four concrete types. The `#[must_use]` on `Condition::root()` prevents silently ignoring errors.
- **Resource limits**: `MAX_ELSEIF_BRANCHES` (256) prevents unbounded branch parsing. The limit check in `collect_elseif_branches` fires before parsing branch bodies, preventing adversarial input from forcing unbounded work.
- **Test coverage**: Comprehensive positive/negative tests for all new features, including edge cases (NaN, empty strings, cross-type comparisons, nested @if in @elseif body, escaped quotes in string literals).

The two MEDIUM findings are documentation improvements (add safety/invariant comments), not correctness bugs. The code is ready to merge after addressing those optional improvements.

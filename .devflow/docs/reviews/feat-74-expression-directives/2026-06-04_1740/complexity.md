# Complexity Review Report

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04T17:40

## Issues in Your Changes (BLOCKING)

### HIGH

**`parse_simple_condition` remains at ~94 lines with embedded inline byte-scanner (CC ~12)** - `parser_helpers.rs:585-678`
**Confidence**: 82%
- Problem: The prior review cycle (Cycle 1) deferred this function's complexity (~94 lines, CC ~12). This PR adds a 39-line inline byte-scanning block (lines 623-661) to detect bare `=` outside quotes/parens. The function now has four distinct responsibilities: negation dispatch, operator comparison, bare-equals detection, and truthy/literal rejection. The inline scanner at lines 623-661 duplicates the same quote/paren-tracking state machine found in `find_unquoted_operator`, `strip_trailing_directive_colon`, and `split_on_unquoted_op`. Maximum nesting depth reaches 4 levels inside the scanner block.
- Fix: Extract the bare-equals check into a dedicated helper like `find_bare_equals(s: &str) -> bool` that reuses the same scanning pattern. This would reduce `parse_simple_condition` from ~94 to ~55 lines and CC from ~12 to ~8, bringing it under the warning threshold.

```rust
/// Returns true if `s` contains a bare `=` (not `==` or `!=`) outside
/// quotes and parentheses.
fn has_bare_equals(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut in_string = false;
    let mut string_char = b'"';
    let mut paren_depth: usize = 0;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            if ch == b'\\' && i + 1 < bytes.len() { i += 2; continue; }
            if ch == string_char { in_string = false; }
            i += 1; continue;
        }
        match ch {
            b'"' | b'\'' => { in_string = true; string_char = ch; }
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'=' if paren_depth == 0 => {
                let after = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
                let before_bang = i > 0 && bytes[i - 1] == b'!';
                if after != b'=' && !before_bang { return true; }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

// Then in parse_simple_condition:
if has_bare_equals(s) {
    return Err(MdsError::syntax("use '==' for comparison, not '='"));
}
```

### MEDIUM

**`parse_expr_inner` at 120 lines is above the warning threshold** - `parser_helpers.rs:139-258`
**Confidence**: 83%
- Problem: This new function spans 120 lines with a CC of approximately 14 (counting each early-return branch as a decision point). It handles 8 distinct expression types in a linear if-chain. The function-call/qualified-call section (lines 189-235) reaches 4 levels of nesting with interleaved `if let` and conditional logic. While the early-return style keeps individual branches readable, the overall length makes it harder to verify that all expression types are covered and that no case falls through incorrectly.
- Fix: Consider extracting the function-call parsing section (lines 189-235) into a helper like `parse_call_or_qualified_call(s, first_dot, first_paren) -> Option<Result<Expr, MdsError>>`. This would reduce `parse_expr_inner` to ~75 lines and keep the call-parsing logic testable independently.

**Duplicated quote/paren-aware byte-scanning pattern across 5 functions** - `parser_helpers.rs` (multiple locations)
**Confidence**: 85%
- Problem: The same `in_string`/`string_char`/`paren_depth` state machine is implemented inline in 5 separate functions: `strip_trailing_directive_colon` (line 39-77), `find_unquoted_operator` (line 344-397), `split_on_unquoted_op` (line 454-498), and the inline block within `parse_simple_condition` (line 624-660). A fifth instance without paren tracking exists in `has_unterminated_string` (line 106-125). Each reimplements escape handling, quote tracking, and paren depth tracking with minor variations in the "action" taken at each byte.
- Fix: Extract a generic `scan_unquoted` iterator or callback-based scanner that handles the state machine once and lets callers provide the "what to do outside quotes/parens" logic. This eliminates ~150 lines of near-identical code and ensures all scanners stay consistent when the grammar evolves (e.g., if bracket support is added later).

```rust
/// Advance through `s` byte-by-byte, calling `on_bare(i, ch)` for each
/// byte that is outside string literals and at paren_depth == 0.
/// Returns early with Some(T) if `on_bare` returns Some.
fn scan_bare_bytes<T>(
    s: &str,
    mut on_bare: impl FnMut(usize, u8, &[u8]) -> Option<T>,
) -> Option<T> {
    let bytes = s.as_bytes();
    let mut in_string = false;
    let mut string_char = b'"';
    let mut paren_depth: usize = 0;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            if ch == b'\\' && i + 1 < bytes.len() { i += 2; continue; }
            if ch == string_char { in_string = false; }
            i += 1; continue;
        }
        match ch {
            b'"' | b'\'' => { in_string = true; string_char = ch; }
            b'(' => { paren_depth += 1; }
            b')' => { paren_depth = paren_depth.saturating_sub(1); }
            _ if paren_depth == 0 => {
                if let Some(result) = on_bare(i, ch, bytes) {
                    return Some(result);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`validate_for_node` grew to 85 lines with nested match arms** - `validator.rs:89-173`
**Confidence**: 80%
- Problem: This function went from ~40 lines (simple path-based validation) to 85 lines with a top-level `match` on 7 `Expr` variants, several containing nested match arms and multiple error construction sites. The `Expr::Var` arm alone is 30 lines with 3 nesting levels. While each arm is individually clear, the function now handles validation dispatch, scope lookup, type checking, and error formatting all in one place.
- Fix: The `Expr::Call` and `Expr::QualifiedCall` arms are identical except for the `name` extraction -- they could be merged into a single arm with `Expr::Call { name, .. } | Expr::QualifiedCall { name, .. }` (though this requires the field name to match, which it does). Alternatively, extract the `Expr::Var` arm's type-checking logic into a small helper.

## Pre-existing Issues (Not Blocking)

(No CRITICAL pre-existing issues found.)

## Suggestions (Lower Confidence)

- **`parser_helpers.rs` file approaching 1389 lines** (Confidence: 65%) -- The file is approaching the 500-line "warning" threshold for maintainability by a factor of ~2.8x. With the addition of `strip_trailing_directive_colon`, `has_unterminated_string`, and `parse_expr_inner`, it now contains 30+ functions. Consider splitting into submodules by concern (e.g., `parser_helpers/scanning.rs`, `parser_helpers/conditions.rs`, `parser_helpers/expressions.rs`).

- **`looks_like_number` could be replaced by a standard check** - `parser_helpers.rs:1072` (Confidence: 62%) -- The function is a custom heuristic; `s.parse::<f64>().is_ok()` is already called immediately after. The `looks_like_number` pre-check could potentially be simplified or removed if the cost of attempting the parse directly is acceptable.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The PR introduces well-structured expression parsing with clear early-return patterns (applies ADR-008 -- bundling related language features). The main complexity concerns are: (1) `parse_simple_condition` remains above the complexity threshold after being deferred from Cycle 1 and gains a new inline scanner, (2) the new `parse_expr_inner` exceeds 100 lines, and (3) the duplicated byte-scanning state machine across 5 functions is a maintainability risk that will compound as the grammar grows. Extracting a shared scanner and the bare-equals check would address all three issues while keeping the linear-scan performance characteristics.

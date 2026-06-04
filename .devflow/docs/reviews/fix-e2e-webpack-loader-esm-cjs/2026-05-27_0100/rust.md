# Rust Review Report

**Branch**: feat/if-equality-negation-elseif -> main
**Date**: 2026-05-27
**PR**: #34
**Commit**: 34a3126 feat(mds-core): add negation, equality (==, !=), and @elseif to MDS template language

## Issues in Your Changes (BLOCKING)

### HIGH

**`find_unquoted_operator` escape handling: closing-quote check runs before escape skip** - `crates/mds-core/src/parser.rs:493-501`
**Confidence**: 82%
- Problem: In `find_unquoted_operator`, when inside a string, the code first checks if `ch == string_char` (line 494, setting `in_string = false`), then checks if `ch == b'\\'` (line 498). This means if the closing-quote character happens to be checked before the escape logic, the order is: (1) close string, (2) check escape. The correct order should be: check escape first, then check closing quote. Currently, this works for `\"` (backslash is not the quote char, so line 494 is false) and `\\"` (backslash skips to `"`, which correctly closes). However, if a string ends with `\'` inside a single-quoted string like `'test\''`, the `'` at position of `\'` would first set `in_string = false` at line 494, then the escape check at 498 never fires because `'` is not `\\`. The backslash before it was already processed by the previous iteration (skip 2 positions), so the `'` here is actually the real closing quote. After tracing all cases, the logic happens to produce correct results for all reachable inputs because the escape skip (i+=2) always jumps past the escaped character. The ordering is still confusing and fragile — a refactor to check escapes first would be clearer and more defensive.
- Fix: Reorder the checks so escape handling is checked before the closing-quote check:
```rust
if in_string {
    // Skip escaped characters inside strings FIRST
    if ch == b'\\' && i + 1 < len {
        i += 2;
        continue;
    }
    if ch == string_char {
        in_string = false;
    }
    i += 1;
    continue;
}
```

### MEDIUM

**`parse_cond_value` accepts `NaN`, `inf`, `-inf` as numeric literals** - `crates/mds-core/src/parser.rs:464-468`
**Confidence**: 88%
- Problem: `s.parse::<f64>()` accepts `NaN`, `inf`, `infinity`, and `-inf` as valid inputs. A user writing `@if val == NaN:` would get a condition that never matches (IEEE 754: NaN != NaN), which is silently surprising. Similarly, `inf` is unlikely to be an intentional condition value. These should be rejected at parse time with a clear error rather than silently accepted.
- Fix: Add a guard before the `parse::<f64>()` call to reject non-finite special values:
```rust
// Numeric (integer or float, including negative)
if let Ok(n) = s.parse::<f64>() {
    if !n.is_finite() {
        return Err(MdsError::syntax(
            "NaN and infinity are not valid condition values",
        ));
    }
    return Ok(CondValue::Number(n));
}
```

**Unknown directive error message does not mention `@elseif`** - `crates/mds-core/src/parser.rs:211-213`
**Confidence**: 90%
- Problem: When a user writes an orphan `@elseif` outside of an `@if` block, the error says: "unknown directive: @elseif flag:. Valid directives: @if, @else:, @end, @for, @define, @import, @export, @include". Since `@elseif` is now a supported directive, the error should either list it or provide a targeted hint that `@elseif` must appear inside an `@if` block. The current message is misleading — it implies `@elseif` is not a valid directive at all.
- Fix: Add a targeted check in `parse_directive` before the generic "unknown directive" error:
```rust
if trimmed.starts_with("@elseif ") || trimmed == "@elseif" {
    return Err(MdsError::syntax(
        "@elseif can only appear inside an @if block, after the then-body",
    ));
}
```

**`parse_cond_value` does not process escape sequences in string literals** - `crates/mds-core/src/parser.rs:436-442`
**Confidence**: 80%
- Problem: The `parse_cond_value` function extracts the inner content of quoted strings verbatim (`&s[1..s.len() - 1]`) without processing escape sequences like `\"` or `\\`. Meanwhile, `find_unquoted_operator` does track escapes when scanning for operators. This means `@if var == "say \"hi\""` would set the RHS value to `say \"hi\"` (with literal backslashes) rather than `say "hi"`. Since the YAML frontmatter value for `var` would be `say "hi"` (without backslashes), the comparison would fail unexpectedly. This is only an issue if users attempt to use escaped quotes in condition values, which may be rare in practice.
- Fix: Either process escape sequences in `parse_cond_value` (unescape `\"` to `"` and `\\` to `\`), or document that escape sequences are not supported in condition string literals and reject backslashes with a clear error.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Stale comment block in `parse_condition`** - `crates/mds-core/src/parser.rs:539-543`
**Confidence**: 92%
- Problem: Lines 539-543 contain a comment block that says "Check for bare `=` (not `==`) ... This is handled below after the equality check." This is a planning/draft comment that describes the code structure but adds no value — the actual bare-`=` check at lines 588-601 is self-documenting. The stale comment at the top of the function is confusing because it appears before the negation prefix check, not near the code it describes.
- Fix: Remove the stale comment block at lines 539-543. The actual implementation at lines 588-601 has its own clear comment.

## Pre-existing Issues (Not Blocking)

No critical pre-existing issues identified in the reviewed files.

## Suggestions (Lower Confidence)

- **`evaluate_condition` could extract path resolution into a shared helper** - `crates/mds-core/src/evaluator.rs:355-362` (Confidence: 70%) -- The file already has `resolve_condition_path` helper, but the diff originally showed duplicated `path.first()` + `resolve_dot_path()` in each match arm. Verify the refactored version is what shipped.

- **`CondValue` and `Condition` could benefit from `#[non_exhaustive]`** - `crates/mds-core/src/ast.rs:16-39` (Confidence: 65%) -- If future comparison operators are planned (e.g., `<`, `>`, `>=`, `<=`), marking these enums `#[non_exhaustive]` would prevent downstream match exhaustiveness assumptions. However, since everything is `pub(crate)`, this is optional.

- **Deep nesting tests use `thread::Builder::new().stack_size(8MB)`** - `crates/mds-core/src/parser.rs:1560`, `crates/mds-cli/tests/security.rs:243` (Confidence: 65%) -- The 8MB stack for nesting-depth tests is a pragmatic workaround, but it signals that `parse_body`/`parse_if_block` recursion depth is bounded only by `MAX_NESTING_DEPTH` (256), which still creates deep call stacks. Consider whether iterative parsing is warranted long-term.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The new feature is well-structured overall: the `Condition` enum follows the Rust pattern of making illegal states unrepresentable, `values_equal` correctly enforces strict typing, and the parser cleanly separates operator scanning from condition parsing. The evaluator's short-circuit logic for `@elseif` is straightforward and correct. Test coverage is thorough with good edge cases (cross-type comparisons, operator-in-string, nested @if inside @elseif).

The blocking HIGH issue (`find_unquoted_operator` escape ordering) is a code clarity and defensiveness concern rather than a currently-exploitable bug. The MEDIUM issues around `NaN`/`inf` acceptance and the missing `@elseif` in the unknown-directive error are user-facing quality gaps that should be addressed before merge.

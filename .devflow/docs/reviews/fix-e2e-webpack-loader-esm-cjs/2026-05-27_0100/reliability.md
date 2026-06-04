# Reliability Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27T01:00:00Z

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`find_unquoted_operator` escape handling has a control-flow ordering fragility** - `crates/mds-core/src/parser.rs:493-503`
**Confidence**: 82%
- Problem: Inside the `in_string` branch, the closing-quote check (line 494) sets `in_string = false` BEFORE the escape check (line 498). If a closing quote character is encountered, `in_string` is set to false, but execution falls through to the escape check and then to `i += 1; continue`. While this produces correct results today because quote characters are never backslashes, the ordering is fragile: if the close-quote check were to `continue` after toggling state, or if future maintenance changed the character set, the fallthrough could cause a misparse. More importantly, for input like `"\\\"` (backslash-backslash-quote), the sequence is: `\\` triggers escape skip at i+=2, then `\"` triggers escape skip at i+=2 (skipping the quote) -- which is correct. However, the lack of an early `continue` after the close-quote match at line 495 means the closing-quote detection and escape detection are not mutually exclusive branches, relying on the implicit fact that `string_char != b'\\'`.
- Fix: Add an early `continue` after the closing-quote toggle to make the control flow explicitly mutually exclusive:
```rust
if in_string {
    // Skip escaped characters inside strings (must check BEFORE close-quote)
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
This also reorders escape-before-close-quote, which is the canonical ordering for string scanners and prevents `\"` from being misinterpreted as a closing quote. The current code handles this correctly because the `\\` in the prior iteration consumes the backslash, but making escape-first is more robust.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Deeply recursive `parse_body`/`parse_if_block` call chain requires explicit stack size for 256-level nesting** - `crates/mds-core/src/parser.rs:107-168`, `crates/mds-core/src/parser.rs:216-282`
**Confidence**: 85%
- Problem: The parser uses mutual recursion between `parse_body` and `parse_if_block` (and `parse_for_block`, `parse_define_block`). With `MAX_NESTING_DEPTH = 256`, the call chain can be ~256 frames deep, each frame carrying local state. Tests for 256-deep nesting already require spawning a thread with 8 MB stack (see `security.rs:243` and `parser.rs:1560`). This means production callers invoking `compile_str` on untrusted input with deep nesting could hit a stack overflow on the default thread stack (typically 2 MB on Linux, 8 MB on macOS). The depth limit error fires correctly in the test, but only because the test gives it enough stack to reach the check. On the default stack, the overflow may occur before the depth check triggers.
- Fix: Consider one of: (a) lowering `MAX_NESTING_DEPTH` to a value that fits in a default 2 MB stack (e.g., 64), (b) converting the recursive descent parser to an iterative approach with an explicit stack for block nesting, or (c) documenting that callers processing untrusted input must ensure adequate stack size. Option (a) is the simplest and aligns with practical use cases (64 levels of nesting is already extreme for a template language).

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`_esmImport` via `new Function` bypasses module resolution guarantees** - `packages/webpack-loader/src/index.ts:10-13` (Confidence: 65%) -- The `new Function('id', 'return import(id)')` pattern is a known workaround for TypeScript CJS downcompilation, but it creates an opaque dynamic import that tools (bundlers, linters, security scanners) cannot statically analyze. The comment documents the rationale well. Monitor for CSP restrictions in environments that disallow `eval`/`new Function`.

- **`parse_cond_value` accepts `Infinity` and `-Infinity` as valid numbers** - `crates/mds-core/src/parser.rs:465` (Confidence: 70%) -- `s.parse::<f64>()` succeeds for `"Infinity"` and `"-Infinity"`, which would create `CondValue::Number(f64::INFINITY)`. While not a crash risk, it may produce surprising equality semantics (e.g., `@if x == Infinity:` would match if x is also infinity). Consider whether these should be rejected as invalid condition values.

- **Off-by-one in elseif limit check allows parsing one extra branch body before rejecting** - `crates/mds-core/src/parser.rs:253-259` (Confidence: 62%) -- The limit check at line 255 runs after parsing the branch body at line 253. This means the parser fully parses the body of the 257th `@elseif` branch before rejecting it. While the limit is still enforced (correctness is preserved), a malicious input with 257 branches each containing expensive bodies forces unnecessary work. Moving the check before `parse_body` would reject earlier.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The changes introduce well-bounded `@elseif` chains (MAX_ELSEIF_BRANCHES = 256), proper short-circuit evaluation, and resource limits that prevent pathological inputs from causing unbounded iteration. The `find_unquoted_operator` scanner terminates deterministically (bounded by input length). All loops in the new code are bounded: the `@elseif` collection loop is capped by the branch limit, the `while i < len` scanner is bounded by string length, and the evaluator's `for (cond, body) in &block.elseif_branches` is bounded by the parsed branches. The main reliability concern is the recursive parser's stack consumption at the 256-level nesting limit, which already requires 8 MB stack in tests. Consider lowering `MAX_NESTING_DEPTH` or documenting stack requirements for untrusted input.

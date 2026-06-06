# Rust Review Report

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`parse_expr_inner` string literal check does not account for trailing escaped quotes** - `crates/mds-core/src/parser_helpers.rs:146-151`
**Confidence**: 82%
- Problem: The string literal detection uses `s.starts_with('"') && s.ends_with('"') && s.len() >= 2` without verifying that the final quote is not itself escaped. An input like `"\\"` (backslash followed by quote, 3 bytes) would be classified as a complete string literal `StringLiteral("\\")` rather than an unterminated string. The `unescape_string` function processes the interior, but the fundamental issue is that the boundary detection does not walk escape sequences.
- Impact: In practice this is mitigated because upstream callers (`strip_trailing_directive_colon`, `find_unquoted_operator`) use proper escape-aware scanning before splitting the condition string. A pathological input would need to bypass those layers. However, `parse_expr_inner` is a `pub(super)` function and could be called from future code paths without those guards.
- Fix: Add an escape-aware check before the simple `starts_with`/`ends_with` test:
  ```rust
  // Quoted string literal — verify the closing quote is not escaped
  if (s.starts_with('"') || s.starts_with('\'')) && s.len() >= 2 {
      let quote = s.as_bytes()[0];
      let bytes = s.as_bytes();
      // Walk from position 1 to find the unescaped closing quote
      let mut i = 1;
      while i < bytes.len() {
          if bytes[i] == b'\\' && i + 1 < bytes.len() {
              i += 2;
              continue;
          }
          if bytes[i] == quote && i == bytes.len() - 1 {
              let inner = &s[1..s.len() - 1];
              return Ok(Expr::StringLiteral(unescape_string(inner)));
          }
          i += 1;
      }
      return Err(MdsError::syntax(
          "unterminated string literal in directive expression",
      ));
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Duplicated string/paren scanning logic across 4 functions** - `crates/mds-core/src/parser_helpers.rs:27-97,318-401,438-501,620-661`
**Confidence**: 85%
- Problem: Four functions (`strip_trailing_directive_colon`, `find_unquoted_operator`, `split_on_unquoted_op`, and the bare `=` scan in `parse_simple_condition`) each independently implement the same byte-level quote-tracking + paren-depth logic with minor variations. This creates a maintenance risk: a fix to one (e.g., adding bracket support) must be replicated to all four, and divergence has already occurred (the bare `=` scanner in `parse_simple_condition` is a separate inline loop).
- Fix: Extract a common `ScanState` struct or a `for_each_unquoted_byte` iterator that tracks `in_string`, `string_char`, `paren_depth`, and escape handling in one place. Each caller would consume it with a closure for its specific logic. This is not blocking because all four implementations are currently correct and tested, but it is technical debt that grows with each new scanning variant. (applies ADR-008 -- bundling related changes reduces maintenance surface.)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Qualified call limited to single-level namespace** - `crates/mds-core/src/parser_helpers.rs:204-234` (Confidence: 65%) -- `parse_expr_inner` only supports `ns.func(args)` qualified calls (one dot before the paren). Multi-level namespaces like `a.b.func(args)` would fail `is_valid_identifier` on `"b.func"`. This is likely by design for the current grammar but could be documented with a comment to prevent future confusion.

- **`CondValue` type retained alongside new literal Expr variants** (Confidence: 60%) -- The prior resolution cycle deferred the CondValue/Expr type duplication. The new `Expr::StringLiteral/NumberLiteral/BooleanLiteral/NullLiteral` variants now overlap with `CondValue::String/Number/Boolean/Null`. Since the prior cycle explicitly deferred this, not re-raising as blocking.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The refactoring from `Vec<String>` dot-paths to `Expr` in `Condition` and `ForBlock` is well-executed. The evaluate/render split (`evaluate_expr` -> `Result<Value>` + `render_expr` wrapper) is a clean separation of concerns. Resource limit hardening in `split()` and `join()` follows best practices (applies ADR-008 -- bundled with the expression directive feature). Error messages are specific and actionable throughout. The one HIGH finding is a defensive concern about escape-aware string boundary detection in `parse_expr_inner`; it is mitigated by upstream scanners in practice but should be hardened to prevent future callers from encountering the gap.

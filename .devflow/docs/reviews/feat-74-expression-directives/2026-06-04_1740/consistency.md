# Consistency Review Report

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent colon-stripping strategy: `@for` does not use `directive_colon_error` for unterminated-string diagnostics** - `crates/mds-core/src/parser.rs:334`
**Confidence**: 90%
- Problem: `@if` and `@elseif` use `directive_colon_error()` to produce a targeted "unterminated string literal" message when `strip_trailing_directive_colon` returns `None` and the input contains an unclosed quote. `@for` calls the same `strip_trailing_directive_colon` but falls back to a generic `"@for directive must end with ':'"` message. Now that `@for` iterables accept full expressions (including function calls with string args like `split(s, ":")`), users can hit unterminated-string scenarios in `@for` too, and the generic error message is unhelpful.
- Fix:
```rust
// parser.rs:333-334 — replace:
let rest = strip_trailing_directive_colon(rest.trim())
    .ok_or_else(|| MdsError::syntax("@for directive must end with ':'"))?;
// with:
let trimmed = rest.trim();
let rest = strip_trailing_directive_colon(trimmed)
    .ok_or_else(|| directive_colon_error("@for", trimmed))?;
```

### MEDIUM

**`@define` still uses naive `strip_suffix(':')` while `@if`/`@elseif`/`@for` use quote+paren-aware `strip_trailing_directive_colon`** - `crates/mds-core/src/parser.rs:382`
**Confidence**: 82%
- Problem: `@define` blocks parse `@define name(params):` using `.strip_suffix(':')`. Today this is safe because `@define` params only allow `CondValue` literals (no function calls with colons in string args). However, the inconsistency creates a latent risk: if `@define` params ever support richer expressions, the naive strip will break on colons inside quoted defaults. The other three directive parsers in this PR were consistently migrated; `@define` was not.
- Fix: Either migrate `@define` to `strip_trailing_directive_colon` for uniformity, or add an explicit comment documenting why the simple `strip_suffix(':')` is deliberately retained (e.g., `@define` body is `name(params)` where parens fully contain any colons).

**Duplicate CondValue/Expr literal variants: two parallel type hierarchies for the same concept** - `crates/mds-core/src/ast.rs:12` and `crates/mds-core/src/ast.rs:116-123`
**Confidence**: 80%
- Problem: `CondValue` (String, Number, Boolean, Null) and `Expr` literal variants (StringLiteral, NumberLiteral, BooleanLiteral, NullLiteral) are structurally identical. `CondValue` was the RHS literal for condition comparisons; that role is now fulfilled by `Expr` literal variants. `CondValue` survives only for `@define` parameter defaults. The codebase now has two parallel type hierarchies for "compile-time literal value", plus a `condvalue_to_value` bridge function. This is a consistency concern (applies ADR-008 -- related features touching the same layers should be unified).
- Fix: This is a refactor opportunity for a follow-up: replace `Param.default: Option<CondValue>` with `Param.default: Option<Expr>` and remove `CondValue` + `condvalue_to_value` entirely. The `parse_cond_value` call in `parse_define_params` would become `parse_expr_inner`, and the evaluator would use `evaluate_expr` for defaults. Mark with a TODO if not addressed in this PR.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Inconsistent error message style for literal rejection across three directive contexts** - `crates/mds-core/src/parser_helpers.rs:430`, `crates/mds-core/src/parser_helpers.rs:672`, `crates/mds-core/src/parser.rs:359`
**Confidence**: 82%
- Problem: The three literal-rejection error messages use different sentence structures:
  - Negation: `"use a variable or function call, not a bare literal, after '!'"`
  - Truthy: `"use a variable or function call, not a bare literal, in @if condition"`
  - For iterable: `"cannot iterate over a literal value: '{iterable_str}'"`
  The first two share a pattern but differ in suffix; the third uses a completely different structure and includes the offending value. This inconsistency makes it harder for users to understand error messages and for tests to match patterns uniformly.
- Fix: Align the three messages to a consistent template. For example:
  - Negation: `"cannot negate a literal value — use a variable or function call after '!'"`
  - Truthy: `"cannot use a literal value as an @if condition — use a variable or function call"`
  - For iterable: `"cannot iterate over a literal value — use a variable or function call"`
  Or include the value in all three for debuggability.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Repeated quote+paren-aware byte scanner pattern (8 instances, 3 new in this PR)** - `crates/mds-core/src/parser_helpers.rs` (multiple locations)
**Confidence**: 85%
- Problem: The same `in_string`/`string_char`/`paren_depth` byte-scanning loop appears in `strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op`, `parse_simple_condition` bare-`=` check, and others. This PR added 3 new instances (the functions `strip_trailing_directive_colon`, `has_unterminated_string`, and the inline bare-`=` scanner). Each instance reimplements escape handling, quote tracking, and paren depth independently. A divergence in any one copy (e.g., forgetting escape handling) would be a subtle bug.
- Fix: Extract a reusable `QuoteParenScanner` iterator or closure that yields `(position, char, is_bare)` tuples, then build the specific scanning functions on top. This is a pre-existing pattern that this PR expanded; not blocking, but worth a follow-up refactor.

## Suggestions (Lower Confidence)

- **`CondValue` naming is now misleading** - `crates/mds-core/src/ast.rs:12` (Confidence: 70%) -- The type is named `CondValue` (condition value) but is no longer used in conditions at all -- only for `@define` parameter defaults. A rename to `LiteralValue` or `DefaultValue` would better reflect its current role.

- **`parse_expr_inner` public visibility scope** - `crates/mds-core/src/parser_helpers.rs:139` (Confidence: 65%) -- `parse_expr_inner` is `pub(super)` but is called from both `parser.rs` (for `@for` iterable parsing) and `parser_helpers.rs` (for condition parsing). If any future module outside `parser` needs expression parsing, the visibility would need widening. The naming with `_inner` suffix is also inconsistent with other public helpers like `parse_condition` which lack the suffix.

- **`parse_simple_condition` inline bare-`=` scanner could reuse `find_unquoted_operator`** - `crates/mds-core/src/parser_helpers.rs:623-661` (Confidence: 65%) -- The inline byte scanner for detecting bare `=` duplicates the quote+paren-aware scanning that `find_unquoted_operator` already performs. The check could potentially be folded into `find_unquoted_operator` by having it also report single `=` as a distinct operator variant.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The core refactoring from `Vec<String>` paths to `Expr` is applied consistently across the AST, evaluator, parser, and validator. The Condition variants, evaluate_condition, and validate_condition all correctly handle the new Expr-based types. However, the colon-stripping strategy diverges between `@for` (no unterminated-string diagnostic) and `@if`/`@elseif` (targeted diagnostic via `directive_colon_error`), and the parallel `CondValue`/`Expr` literal type hierarchies should be acknowledged with a TODO. The error message style for literal rejection varies across the three rejection sites.

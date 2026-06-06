# Consistency Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### HIGH

**Module docstring references removed function `parse_dot_path`** - `crates/mds-core/src/parser_helpers.rs:7`
**Confidence**: 95%
- Problem: The module-level docstring still lists `parse_dot_path` under "Condition parsing" responsibilities, but this function was removed and replaced by `parse_expr_inner`. The docstring also omits the new functions: `strip_trailing_directive_colon`, `has_unterminated_string`, and `parse_expr_inner`.
- Fix: Update the module docstring to reflect the current API surface:
```rust
//! - **Condition parsing** — `parse_condition`, `parse_negation_condition`,
//!   `find_unquoted_operator`, `parse_cond_value`, `parse_expr_inner`,
//!   `strip_trailing_directive_colon`, `has_unterminated_string`
```

**`parse_condition` docstring not updated for expression support** - `crates/mds-core/src/parser_helpers.rs:520-522`
**Confidence**: 92%
- Problem: The `parse_condition` docstring still documents only `var == "value"` / `var != 42` forms for Eq/NotEq, but the implementation now accepts arbitrary expressions on both sides (e.g., `func(a) == func(b)`). The child function `parse_simple_condition` was correctly updated (line 565-568) but the parent was not.
- Fix: Update the docstring to match:
```rust
/// - `var` / `config.debug` / `func(args)` → `Condition::Truthy`
/// - `!var` / `!func(args)` → `Condition::Not`
/// - `expr == expr` / `expr != expr` → `Condition::Eq` / `Condition::NotEq`
```

### MEDIUM

**`ForBlock` import style inconsistent in validator.rs** - `crates/mds-core/src/validator.rs:1,90`
**Confidence**: 88%
- Problem: The PR removed `ForBlock` from the top-level import (line 1) but uses it as `crate::ast::ForBlock` inline at line 90. Every other AST type used in this file (`IfBlock`, `Condition`, `Expr`, `Node`) is imported at the top. This is a gratuitous style inconsistency within the same file.
- Fix: Add `ForBlock` back to the import line:
```rust
use crate::ast::{required_param_count, Arg, Condition, Expr, ForBlock, IfBlock, Node};
```
Then change line 90 from `block: &crate::ast::ForBlock` to `block: &ForBlock`.

**Inconsistent unterminated-string error handling between `@if` and `@elseif`** - `crates/mds-core/src/parser.rs:246-253,311-312`
**Confidence**: 85%
- Problem: `parse_if_block` (line 246-253) provides a targeted "unterminated string literal in @if condition" error when `strip_trailing_directive_colon` returns `None` and the input has an unterminated string. But `collect_elseif_branches` (line 311-312) only gives the generic "@elseif directive must end with ':'" error. Since the PR introduced the enhanced error specifically for expression-containing directives, both `@if` and `@elseif` should benefit.
- Fix: Apply the same `has_unterminated_string` check in `collect_elseif_branches`:
```rust
let elseif_cond_str = strip_trailing_directive_colon(elseif_rest)
    .ok_or_else(|| {
        if has_unterminated_string(elseif_rest) {
            MdsError::syntax("unterminated string literal in @elseif condition")
        } else {
            MdsError::syntax("@elseif directive must end with ':'")
        }
    })?;
```

**`@define` still uses `strip_suffix(':')` while `@if` and `@for` use `strip_trailing_directive_colon`** - `crates/mds-core/src/parser.rs:373-376`
**Confidence**: 80%
- Problem: The PR introduced `strip_trailing_directive_colon` for `@if` and `@for` to handle colons inside string arguments, but `@define` still uses the old `strip_suffix(':')` pattern. While `@define` is less likely to be affected (the colon in default values is inside parentheses), the inconsistency means the directive colon-stripping strategy varies within the same file. If a future change adds expression support to `@define` body syntax, this will be a latent bug.
- Fix: Consider using `strip_trailing_directive_colon` for `@define` as well for uniform directive parsing. This is lower priority since `@define` defaults are inside parens.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`CondValue` docstring says "RHS of an equality condition" but conditions no longer use it** - `crates/mds-core/src/ast.rs:8`
**Confidence**: 85%
- Problem: The `CondValue` enum docstring was partially updated in this PR (line 8 now says "or as a default parameter value") but the opening line still says "on the RHS of an equality condition." Conditions now use `Expr` variants, not `CondValue`. `CondValue` is now only used for `@define` parameter defaults. The stale reference will confuse readers.
- Fix: Update the docstring:
```rust
/// A literal value for a default parameter in `@define` blocks.
```

**Trivial wrapper `evaluate_condition_value` adds indirection without value** - `crates/mds-core/src/evaluator.rs:405-411`
**Confidence**: 82%
- Problem: `evaluate_condition_value` is a one-line function that just calls `evaluate_expr`. The call sites (`evaluate_condition`) could call `evaluate_expr` directly, which would be consistent with how `evaluate_for` directly calls `evaluate_expr(&block.iterable, scope, ctx)` at line 617. Having a wrapper for conditions but not for iterables is inconsistent.
- Fix: Inline `evaluate_condition_value` and call `evaluate_expr` directly in `evaluate_condition`. Alternatively, if the intent is to add condition-specific validation later, add a comment documenting that purpose.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Duplicated string/paren-depth scanning pattern across 5+ functions** - `crates/mds-core/src/parser_helpers.rs`
**Confidence**: 85%
- Problem: The `in_string` / `string_char` / `paren_depth` byte-level scanning pattern is now duplicated across `strip_trailing_directive_colon` (line 37-75), `has_unterminated_string` (line 95-114), `find_unquoted_operator` (line 332-390), `split_on_unquoted_op` (line 442-489), and the bare-`=` check in `parse_simple_condition` (line 609-647). The PR added 2 new instances (`strip_trailing_directive_colon`, `has_unterminated_string`) and enhanced 2 existing ones (adding `paren_depth`). A shared `ScanState` or callback-based scanner would reduce the surface area for bugs. This is pre-existing architectural debt amplified by the PR.

## Suggestions (Lower Confidence)

- **`CondValue` and `Expr` literal variants represent the same concept** - `crates/mds-core/src/ast.rs` (Confidence: 70%) -- `CondValue::String/Number/Boolean/Null` and `Expr::StringLiteral/NumberLiteral/BooleanLiteral/NullLiteral` are isomorphic. Now that conditions use `Expr`, `CondValue` exists only for `@define` defaults. A future cleanup could replace `Param.default: Option<CondValue>` with `Param.default: Option<Expr>` and remove `CondValue` entirely, along with `condvalue_to_value`.

- **`looks_like_number` guard in `parse_expr_inner` vs direct parse in `parse_cond_value`** - `crates/mds-core/src/parser_helpers.rs:163,292` (Confidence: 65%) -- `parse_expr_inner` uses a `looks_like_number` guard before `s.parse::<f64>()`, while `parse_cond_value` goes straight to parse. The guard is necessary in `parse_expr_inner` (to distinguish from identifiers), but the inconsistency could confuse future contributors.

- **Test comments still reference `condvalue_to_value` for coverage tracking** - `crates/mds-core/src/evaluator.rs:1016,1026,1039,1052` (Confidence: 65%) -- Comments like `// condvalue_to_value: CondValue::Number -> Value::Number` describe internal plumbing that is only tangentially related to what the tests actually verify. These comments are pre-existing but could be confusing now that the condition path uses `Expr` instead of `CondValue`.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR introduces expression support in directives with generally good internal consistency. The core pattern -- replacing `Vec<String>` paths and `CondValue` with `Expr` in `Condition` and `ForBlock` -- is applied uniformly across AST, parser, evaluator, and validator. The new `strip_trailing_directive_colon` function correctly handles the colon-in-string-args edge case that expressions introduce. However, several documentation references were not updated to reflect the new API surface, the `ForBlock` import was inconsistently removed in validator.rs, and the unterminated-string error message was added for `@if` but not `@elseif`. These are all straightforward fixes. Applies ADR-008 (bundled language features in single PR).

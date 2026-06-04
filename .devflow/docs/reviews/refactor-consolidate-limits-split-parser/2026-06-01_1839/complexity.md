# Complexity Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01T18:39

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`parser_helpers.rs` exceeds file length threshold (733 lines)** - `parser_helpers.rs`
**Confidence**: 85%
- Problem: The helpers file is 733 lines, well above the 500-line critical threshold and more than double the 300-line warning threshold. While the split from parser.rs (1820 lines) is a clear improvement, the helpers file has accumulated 20 standalone functions with no internal organization. It is a flat bag of functions rather than a cohesive module with logical sub-groupings.
- Fix: Consider further splitting `parser_helpers.rs` into cohesive sub-modules by concern:
  - `condition.rs`: `parse_condition`, `parse_negation_condition`, `parse_cond_value`, `find_unquoted_operator`, `parse_dot_path` (~175 lines)
  - `interpolation.rs`: `parse_interpolation_expr`, `parse_dot_expr`, `parse_args`, `parse_args_inner`, `parse_single_arg_inner` (~175 lines)
  - `directive.rs`: `parse_import_directive`, `parse_export_directive`, `parse_for_vars`, `parse_quoted_path` (~130 lines)
  - `util.rs`: `unescape_string`, `is_valid_identifier`, `is_directive_token`, `validate_dot_path_parts`, `strip_leading_newline`, `strip_trailing_newline` (~100 lines)

  This is a "should we go further" question rather than a blocking defect -- the current state is strictly better than before. Downgrading to Should-Fix.

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### HIGH

**`parse_dot_expr` has 6 parameters** - `parser_helpers.rs:401`
**Confidence**: 85%
- Problem: `parse_dot_expr(content, dot_pos, offset, len, file, source)` takes 6 parameters, above the 5-parameter warning threshold. The `(offset, len, file, source)` group is a repeated pattern across multiple functions -- it represents source-location context for error reporting.
- Fix: Introduce a `SourceCtx` struct to bundle error-reporting context:
  ```rust
  struct SourceCtx<'a> {
      offset: usize,
      len: usize,
      file: &'a str,
      source: &'a str,
  }
  ```
  Then `parse_dot_expr(content: &str, dot_pos: usize, ctx: SourceCtx)` drops to 3 parameters. This would also simplify `parse_interpolation_expr` (4 params) and any future functions that need source context.

### MEDIUM

**`parse_import_directive` is 61 lines with 3 nesting levels** - `parser_helpers.rs:233`
**Confidence**: 82%
- Problem: At 61 lines the function is in the warning zone (50-200 lines). It handles three distinct import forms (selective, alias, merge) in a single function with multiple early returns. The selective import branch (L237-267) alone is 30 lines with 3 levels of nesting.
- Fix: Extract the selective import branch into a dedicated `parse_selective_import(rest, offset)` helper. This would bring `parse_import_directive` to ~35 lines and make each import form independently testable.

**`parse_args_inner` is 67 lines with state-machine complexity** - `parser_helpers.rs:541`
**Confidence**: 80%
- Problem: The function uses 5 mutable state variables (`args`, `current`, `in_string`, `string_char`, `escaped`, `paren_depth`) creating a manual state machine. While the logic is correct and bounded, the interleaving of string tracking, paren tracking, and comma splitting makes it harder to verify at a glance.
- Fix: This is acceptable complexity for a tokenizer-like function. Consider adding a brief comment block at the top summarizing the state machine states (Outside, InString, Escaped) for the next reader. No structural change needed.

**`parse_define_block` is 60 lines** - `parser.rs:363`
**Confidence**: 80%
- Problem: At 60 lines the function is in the warning zone. It handles parameter parsing, duplicate detection, body parsing, and newline trimming. The parameter validation loop (L394-406) with `HashSet` tracking could be extracted.
- Fix: Extract parameter parsing to `parse_define_params(params_str) -> Result<Vec<String>, MdsError>` to bring the function to ~40 lines.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parse_body` is 56 lines with 8-arm match** - `parser.rs:118`
**Confidence**: 82%
- Problem: The match on Token variants has 8 arms spanning 40+ lines. Each arm is simple, but the function's visual footprint makes it harder to scan. This existed in the original parser.rs on main.
- Fix: Not blocking. Could extract text-producing arms into a `token_to_node()` helper, but the current form is idiomatic for Rust parsers.

**`parse_directive` is 57 lines with 9 conditional branches** - `parser.rs:175`
**Confidence**: 80%
- Problem: Linear chain of 9 `if`/`if let` checks for directive dispatch. Cyclomatic complexity ~10 (at the warning threshold). This is pre-existing from main.
- Fix: Not blocking. A dispatch table or macro could reduce the visual complexity, but the early-return pattern keeps each branch isolated and readable.

## Suggestions (Lower Confidence)

- **Repeated dot-path validation pattern** - `parser_helpers.rs` (Confidence: 70%) -- `parse_dot_path` and `validate_dot_path_parts` do nearly the same thing (validate segments + check MAX_DOT_SEGMENTS) but return different types. The `parse_for_block` in parser.rs also inlines this validation (L332-347). Consider unifying around a single validation path to reduce the surface area for inconsistency.

- **`parse_single_arg_inner` has 4-way if/else-if chain** - `parser_helpers.rs:614` (Confidence: 65%) -- The function dispatches across StringLiteral, Call, MemberAccess, and Var using a chain of `if`/`else if` with string inspection. This is typical for recursive-descent parsers and works, but a brief doc comment explaining the dispatch priority would help future readers.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 1 | 3 | - |
| Pre-existing | - | - | 2 | - |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR achieves its stated goal: splitting a 1820-line monolithic `parser.rs` into a 423-line parser core, 733-line helpers file, and 668-line test file. The `limits.rs` consolidation (48 lines) cleanly centralizes 5 cross-module constants with pinning tests. All individual functions are under the 200-line critical threshold. The longest functions (`parse_dot_expr` at 69 lines, `parse_args_inner` at 67 lines, `parse_import_directive` at 61 lines) are in the warning zone but have clear single-purpose logic with early returns. No behavioral changes were introduced. The main condition for approval is addressing the `parse_dot_expr` 6-parameter signature (applies ADR-001 -- pre-merge quality gate) and considering whether `parser_helpers.rs` at 733 lines warrants further decomposition in a follow-up.

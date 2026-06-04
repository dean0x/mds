# Complexity Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### HIGH

**`parse_if_block` function length approaching warning threshold** - `crates/mds-core/src/parser.rs:234-300`
**Confidence**: 85%
- Problem: `parse_if_block` is now 66 lines (threshold: 50 = HIGH). The addition of the @elseif loop (lines 249-280) brings the function from a clean 30-line @if/@else parser to a 66-line function with three sequential parsing phases (condition, elseif loop, else). The while-loop that collects @elseif branches adds a nesting level inside the method, increasing cyclomatic complexity to approximately 8 (the `while` + `if !starts_with` break + `strip_prefix` check + `strip_suffix` check + `len >= MAX` guard + the else_body `if matches!`).
- Fix: Extract the @elseif collection loop into a dedicated method `collect_elseif_branches`:
```rust
fn collect_elseif_branches(&mut self) -> Result<Vec<(Condition, Vec<Node>)>, MdsError> {
    let mut branches: Vec<(Condition, Vec<Node>)> = Vec::new();
    while let Some(Token::Directive(d, _)) = self.peek() {
        if !d.trim().starts_with("@elseif ") {
            break;
        }
        let elseif_dir = d.clone();
        self.pos += 1;

        let elseif_cond_str = elseif_dir
            .trim()
            .strip_prefix("@elseif ")
            .ok_or_else(|| MdsError::syntax("internal error: expected @elseif prefix"))?
            .trim()
            .strip_suffix(':')
            .ok_or_else(|| MdsError::syntax("@elseif directive must end with ':'"))?
            .trim();

        let elseif_cond = parse_condition(elseif_cond_str)?;
        let elseif_body = self.parse_body(&["@else:", "@end"], &["@elseif "])?;

        if branches.len() >= MAX_ELSEIF_BRANCHES {
            return Err(MdsError::syntax(format!(
                "@if block has more than {MAX_ELSEIF_BRANCHES} @elseif branches"
            )));
        }
        branches.push((elseif_cond, elseif_body));
    }
    Ok(branches)
}
```
This would bring `parse_if_block` back down to ~35 lines and keep the @elseif loop's complexity isolated and independently testable.

### MEDIUM

**`parse_condition` function has 5 distinct exit paths** - `crates/mds-core/src/parser.rs:561-623`
**Confidence**: 82%
- Problem: `parse_condition` handles negation prefix (with three sub-checks: double negation, negation+comparison, empty after `!`), equality/inequality operators, bare `=` detection, and the default truthy path. This gives it a cyclomatic complexity of approximately 9 (negation `if` + double negation + negation+operator + empty rest + operator `if` + rhs empty + bare `=` + `!after.starts_with('=')` + `!before.ends_with('!')` + truthy default). While each branch is small and well-commented, the function is doing condition dispatch, validation, AND parsing in one place.
- Fix: The function is 62 lines -- borderline, not critical. If it grows further (e.g., adding `&&`/`||` operators), extract the negation prefix handling into `parse_negated_condition(rest: &str)` and the operator handling into `parse_comparison_condition(s: &str, op_pos, op)`. For now, the early-return style keeps it readable; flag this for extraction if another condition type is added.

**`find_unquoted_operator` manual byte-scanning loop** - `crates/mds-core/src/parser.rs:506-553`
**Confidence**: 80%
- Problem: This is a 47-line manual scanner with mutable state tracking (`in_string`, `string_char`, byte index `i`). The while loop with manual `i += 2` for escape sequences and multiple `if` branches checking different byte values has cyclomatic complexity of approximately 7. Manual index arithmetic with `i + 1 < len` bounds checks is a common source of off-by-one bugs, though the current implementation appears correct.
- Fix: The byte-level scanning is justified here -- this needs to be aware of string quoting context, and Rust's standard library does not provide a quote-aware operator finder. The function is well-commented with clear state transitions. No refactor needed now, but if additional operators are added (e.g., `<`, `>`, `<=`, `>=`), consider a small state machine enum rather than boolean flags.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`parser.rs` file length at 1753 lines** - `crates/mds-core/src/parser.rs`
**Confidence**: 82%
- Problem: The file is 1753 lines total (threshold: >500 = CRITICAL by the metrics, but ~700 lines are `#[cfg(test)]` module content). The production code is approximately 1050 lines, which is above the 500-line warning threshold. The new additions (`parse_dot_path`, `parse_cond_value`, `find_unquoted_operator`, `parse_condition`, `unescape_string`) add approximately 200 new lines of production code. Each function is focused and well-scoped, but the file is accumulating responsibility for all parsing concerns.
- Fix: The file is a parser module and all functions share a common concern (token-to-AST conversion). The test section is appropriately co-located. No split needed now, but if the condition language grows (adding `&&`/`||`, comparisons like `<`/`>`), consider extracting `condition.rs` containing `parse_condition`, `parse_cond_value`, `find_unquoted_operator`, `parse_dot_path`, and `unescape_string` (approximately 200 lines that form a self-contained sub-parser).

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parse_args_inner` character-by-character loop with nested state** - `crates/mds-core/src/parser.rs:934-1000`
**Confidence**: 82%
- Problem: This 66-line function uses a char-by-char loop with `escaped`, `in_string`, `string_char`, and `paren_depth` mutable state variables. This is structurally similar to `find_unquoted_operator` but more complex due to paren tracking. Both functions implement hand-rolled string scanners with escape awareness, representing a pattern that could eventually be unified.
- Fix: Not blocking. If a third quote-aware scanner is needed, consider a shared `QuoteAwareScanner` iterator that yields tokens (string-literal, operator, delimiter, text) and can be consumed by both argument parsing and condition parsing.

## Suggestions (Lower Confidence)

- **`parse_single_arg_inner` has 4 branches with different parse strategies** - `crates/mds-core/src/parser.rs:1007-1049` (Confidence: 70%) -- The function dispatches across string literal, nested call, member access, and variable in a chain of `if`/`else if`. Each branch is short, but the function handles four distinct grammar productions. Consider if a small enum of arg-token-types could make this a match.

- **`validate_node` growing match arms** - `crates/mds-core/src/validator.rs:22-125` (Confidence: 65%) -- The `Node::For` arm is 53 lines with a 3-condition guard plus nested `if`. The new @elseif validation loop is clean, but the overall match body is 103 lines. Consider extracting `validate_if`, `validate_for`, `validate_define` helper functions to keep each arm under 10 lines.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new code is well-structured with clear early-return patterns, proper bounds enforcement (MAX_ELSEIF_BRANCHES, MAX_NESTING_DEPTH reduced from 256 to 64), and good separation of concerns (the `Condition::root()` centralisation and `resolve_condition_value` extraction reduce duplication). The primary actionable finding is extracting the @elseif collection loop from `parse_if_block` to keep function length under the 50-line threshold. The condition sub-parser functions (`parse_condition`, `parse_cond_value`, `find_unquoted_operator`) are individually focused and readable despite the manual byte-scanning, which is justified by the quote-awareness requirement. The evaluator changes (`evaluate_condition`, `values_equal`) are clean and concise -- each function is under 15 lines with straightforward match dispatch.

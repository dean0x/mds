# Complexity Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**`evaluate_for` remains at 64 lines with moderate cyclomatic complexity** - `src/evaluator.rs:405-468`
**Confidence**: 82%
- Problem: After extracting `evaluate_for_key_value` and `run_loop_body`, the `evaluate_for` function is still 64 lines with 5 distinct early-return paths and a match statement. The function handles two completely separate iteration strategies (key-value and array) in one body. This exceeds the 50-line warning threshold.
- Fix: The extraction already completed most of the simplification. The remaining body is largely guard clauses and a single loop. The key-value path is already delegated. Consider extracting the array iteration path (lines 430-467) into a parallel `evaluate_for_array` helper for symmetry:
  ```rust
  fn evaluate_for(block: &ForBlock, scope: &mut Scope, ctx: &mut EvalContext) -> Result<String, MdsError> {
      let root = block.iterable.first()
          .ok_or_else(|| MdsError::syntax("internal error: @for block has empty iterable path"))?;
      let iterable = resolve_dot_path(root, &block.iterable[1..], scope)?;

      if let Some(ref key_var) = block.key_var {
          let map = match iterable { /* ... */ };
          return evaluate_for_key_value(key_var, &block.var, map, &block.body, scope, ctx);
      }
      evaluate_for_array(iterable, &block.var, &block.body, scope, ctx)
  }
  ```

### MEDIUM

**`parse_args_inner` at 59 lines with 4 nesting levels** - `src/parser.rs:634-692`
**Confidence**: 83%
- Problem: The character-by-character parser loop in `parse_args_inner` has a 4-level nesting depth (`for` > `if/else if/else if/else if/else if` chain). While each individual branch is simple, the function's visual complexity makes it harder to follow than necessary. At 59 lines it's in the warning zone.
- Fix: This is a pre-existing function that was not substantially modified in this PR. However, since the `parse_single_arg_inner` function was extended with new `MemberAccess` parsing (a change in this PR), note that `parse_args_inner` feeds into it. No immediate action required since this is a character-state-machine pattern that is inherently sequential, but if it grows further, consider extracting the string-parsing state into a helper.

**`parse_single_arg_inner` at 53 lines with if/else-if chain (5 branches)** - `src/parser.rs:699-751`
**Confidence**: 81%
- Problem: The function dispatches across 5 mutually exclusive cases in a flat if/else-if chain: string literal, nested function call, member access (new in this PR), variable reference, and error. At 53 lines it is in the warning zone. The new `MemberAccess` branch (lines 723-741) adds a dot-path validation loop that duplicates the same pattern seen in `parse_dot_expr`.
- Fix: Extract a shared `validate_dot_path_parts` helper to deduplicate the validation pattern:
  ```rust
  fn validate_dot_path_parts(parts: &[&str], context: &str) -> Result<(), MdsError> {
      if parts.len() > MAX_DOT_SEGMENTS {
          return Err(MdsError::syntax(format!(
              "dot path in {context} exceeds maximum segment count of {MAX_DOT_SEGMENTS}"
          )));
      }
      for part in parts {
          let part = part.trim();
          if !is_valid_identifier(part) {
              return Err(MdsError::syntax(format!(
                  "invalid dot-path in {context}: each segment must be a valid identifier"
              )));
          }
      }
      Ok(())
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`invoke_function` at 57 lines with scope manipulation complexity** - `src/evaluator.rs:232-288`
**Confidence**: 80%
- Problem: This function handles parameter binding, lexical scope restoration (namespaces, functions, captured vars), call-stack LIFO tracking, and double-fault error preservation. While each section is well-commented, the function mixes setup concerns (scope restoration) with execution concerns (call-stack tracking, error handling). At 57 lines it is in the warning zone.
- Fix: The function is coherent in purpose (invoke a function with proper scope/stack management) and the comments make it readable within the 5-minute rule. This is a low-priority improvement. If it grows further, the scope-restoration block (lines 254-267) could become `restore_captured_scope(scope, &func.captured)`.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parser.rs` total file length at 1321 lines** - `src/parser.rs`
**Confidence**: 85%
- Problem: The parser file exceeds the 500-line critical threshold for file length at 1321 lines (including ~490 lines of tests). The production code is approximately 830 lines. While the tests belong with the parser module, the file is becoming large enough to consider splitting.
- Fix: Not blocking. When the module next grows, consider moving tests to a `tests/parser_tests.rs` or `parser/tests.rs` submodule, or splitting the parser into `parser/mod.rs` + `parser/args.rs` + `parser/directives.rs`.

## Suggestions (Lower Confidence)

- **Repeated MAX_DOT_SEGMENTS guard pattern** - `src/parser.rs:224`, `src/parser.rs:284`, `src/parser.rs:549`, `src/parser.rs:726`, `src/evaluator.rs:103` (Confidence: 72%) — The same `if parts.len() > MAX_DOT_SEGMENTS { return Err(...) }` pattern appears 5 times with slightly different error messages. A shared validator function would reduce this to one definition and 5 call sites.

- **`parse_dot_expr` has 6 parameters** - `src/parser.rs:521-528` (Confidence: 65%) — The function takes 6 parameters (`content`, `dot_pos`, `offset`, `len`, `file`, `source`). This is at the high parameter count threshold. However, 4 of these (`offset`, `len`, `file`, `source`) are context-for-error-reporting that flows through many parser functions, so this is consistent with the existing pattern.

- **`evaluate_for` duplication of total_iterations check** - `src/evaluator.rs:388,458` (Confidence: 62%) — The `ctx.total_iterations > MAX_TOTAL_ITERATIONS` guard appears identically in both `evaluate_for_key_value` and the array path in `evaluate_for`. Could be consolidated into `run_loop_body` itself, but this would change the semantics slightly (the check currently runs before the body, not after).

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR demonstrates strong complexity discipline: extracting `run_loop_body`, `evaluate_for_key_value`, and `parse_dot_expr` are all positive refactoring moves that reduce nesting and bring previously-oversized functions under control. The remaining issues are moderate — function lengths hovering just above the 50-line warning threshold — and do not pose a readability risk given the clear naming and thorough documentation. The one HIGH finding (`evaluate_for` at 64 lines) is a soft concern given that the function is already significantly improved from its previous state. Approve with the suggestion to extract a shared dot-path validation helper in a follow-up.

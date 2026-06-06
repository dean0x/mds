# Code Review Summary

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04_1740
**Cycle**: 2

## Merge Recommendation: BLOCK

This PR has **6 HIGH blocking issues** and **3 MEDIUM blocking issues** that prevent merge. The issues fall into two categories:

1. **Correctness & Safety** (Rust, Testing): Escape-aware quote detection gap in `parse_expr_inner` + missing test assertions
2. **Code Quality** (Complexity, Consistency, Performance): Deferred items from Cycle 1 are still present and grew worse with new duplicate code

The PR made good progress on expression directives but the review identified issues that must be resolved before merging.

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 6 | 3 | 0 | 9 |
| Should Fix | 0 | 0 | 5 | 0 | 5 |
| Pre-existing | 0 | 0 | 1 | 0 | 1 |

## Blocking Issues (Must Fix Before Merge)

### HIGH Severity

**1. `parse_expr_inner` escape-aware quote detection gap** — `crates/mds-core/src/parser_helpers.rs:146-151` (82% confidence, RUST)
- Problem: String literal detection uses `starts_with('"') && ends_with('"')` without verifying the closing quote is not escaped. Input like `"\\"` (escaped backslash before quote) would be incorrectly classified as a complete string literal.
- Fix: Add escape-aware boundary detection before accepting the string as terminated.
- Impact: Correctness issue — malformed input could parse when it should fail with unterminated string error.

**2. @for directive uses wrong error helper for unterminated strings** — `crates/mds-core/src/parser.rs:334` (90% confidence, CONSISTENCY)
- Problem: `@for` calls `strip_trailing_directive_colon` but falls back to generic error. `@if`/`@elseif` use `directive_colon_error()` to produce targeted error messages. Now that `@for` iterables accept expressions with colons in string args, the inconsistency is unhelpful to users.
- Fix: Use `directive_colon_error("@for", trimmed)` instead of generic message.
- Impact: User experience — less helpful error messages when conditions contain unterminated strings.

**3. split() Vec without pre-allocation hint** — `crates/mds-core/src/builtins.rs:264` (82% confidence, PERFORMANCE)
- Problem: `Vec::new()` starts with zero capacity. For large inputs, this causes ~17 reallocations (log2 scaling). The incremental limit check happens after each reallocation.
- Fix: Use `Vec::with_capacity(64)` to avoid first 6 reallocations for free.
- Impact: Performance — unnecessary allocation overhead in common operation.

**4. parse_simple_condition remains at ~94 lines with CC ~12** — `crates/mds-core/src/parser_helpers.rs:585-678` (82% confidence, COMPLEXITY)
- Problem: This function was flagged as high complexity in Cycle 1 and deferred. This PR adds a NEW 39-line inline byte scanner (lines 623-661) for bare `=` detection, making the problem worse. The function now has 4 distinct responsibilities with max nesting depth of 4 levels.
- Fix: Extract bare-equals check into `has_bare_equals(s)` helper function, reducing `parse_simple_condition` from ~94 to ~55 lines and CC from ~12 to ~8.
- Impact: Maintainability — deferred issue grew worse; function is still above complexity threshold.

**5. Nested call argument structure not verified in parse test** — `crates/mds-core/src/parser_tests.rs:1206` (82% confidence, TESTING)
- Problem: `parse_for_nested_call_iterable` asserts the outer call is `sort` but does not verify the inner argument is a `unique` call. Parser-level AST verification is missing.
- Fix: Destructure and assert inner argument is `Arg::NestedCall(inner_name, inner_args)` with expected structure.
- Impact: Test quality — test would pass even if parser silently corrupted inner AST structure.

**6. Sort order not asserted in evaluate_for_sort_unique_iterable** — `crates/mds-core/src/parser_tests.rs:1487` (80% confidence, TESTING)
- Problem: Test checks that `a` and `b` are present and count is 2, but does not assert `a` appears before `b`. A broken sort returning `[b, a]` would still pass.
- Fix: Add assertion that first item is "a" and second is "b".
- Impact: Test quality — test does not verify the behavior its name claims to test.

### MEDIUM Severity (Blocking)

**7. parse_expr_inner 120 lines with CC ~14** — `crates/mds-core/src/parser_helpers.rs:139-258` (83% confidence, COMPLEXITY)
- Problem: New function exceeds 100-line threshold. The function-call section (lines 189-235) reaches 4 nesting levels. While individual branches are clear, the overall length makes it hard to verify all expression types are covered.
- Fix: Extract function-call/qualified-call parsing (lines 189-235) into helper `parse_call_or_qualified_call()`, reducing to ~75 lines.
- Impact: Code quality — difficult to verify and extend; combined with `parse_simple_condition`, parser helpers are growing unwieldy.

**8. @define inconsistency: naive strip_suffix vs strip_trailing_directive_colon** — `crates/mds-core/src/parser.rs:382` (82% confidence, CONSISTENCY)
- Problem: `@define` still uses `.strip_suffix(':')` while `@if`/`@elseif`/`@for` use quote+paren-aware stripping. If `@define` params ever support richer expressions, this will break.
- Fix: Either migrate `@define` to `strip_trailing_directive_colon` or add explicit comment documenting why simple strip is safe (i.e., body is `name(params)` where parens fully contain colons).
- Impact: Future-proofing — latent risk if grammar evolves.

**9. CondValue/Expr literal duplication** — `crates/mds-core/src/ast.rs:12, 116-123` (80% confidence, CONSISTENCY)
- Problem: `CondValue` (String, Number, Boolean, Null) and `Expr` literal variants (StringLiteral, etc.) are structurally identical. Two parallel type hierarchies for the same concept. Deferred from Cycle 1 but still unresolved.
- Fix: Replace `Param.default: Option<CondValue>` with `Option<Expr>` and remove `CondValue` + `condvalue_to_value` bridge. Mark with TODO if not addressed in this PR.
- Impact: Type safety — maintenance multiplier; adding a new literal requires changes in two places.

## Should Fix Issues (Category 2: In Code You Touched)

### MEDIUM Severity

**10. Literal rejection error messages are inconsistent** — Multiple locations (82% confidence, CONSISTENCY)
- Problem: Three different error message styles:
  - Negation: `"use a variable or function call, not a bare literal, after '!'"`
  - Truthy: `"use a variable or function call, not a bare literal, in @if condition"`
  - For iterable: `"cannot iterate over a literal value: '{value}'"`
- Fix: Align to consistent template (e.g., all include the bad value, or none).
- Impact: UX — inconsistent error messages confuse users about what went wrong.

**11. evaluate_expr unconditional clones in Var lookups** — `crates/mds-core/src/evaluator.rs:147-150` (80% confidence, PERFORMANCE)
- Problem: Every variable lookup clones the entire `Value`, including large objects/arrays. For `@for x in large_array:`, the iterable is cloned twice (once in `evaluate_expr`, once in `evaluate_for_array`).
- Fix: Consider `evaluate_expr_ref` variant returning `&Value`, or match iterable inline to avoid intermediate clone.
- Impact: Performance — double-clone of large arrays in loop contexts.

**12. join() output String without capacity estimate** — `crates/mds-core/src/builtins.rs:385` (80% confidence, PERFORMANCE)
- Problem: `String::new()` starts at zero capacity. For large arrays, output grows through repeated reallocations.
- Fix: Pre-estimate capacity as `arr.len() * (avg_element_len + sep.len())` capped by `MAX_OUTPUT_SIZE`.
- Impact: Performance — reallocation churn for large array joins.

**13. Duplicated quote/paren-aware byte scanning across 4 functions** — Multiple locations (85% confidence, RUST)
- Problem: `strip_trailing_directive_colon`, `find_unquoted_operator`, `split_on_unquoted_op`, and inline bare `=` scanner in `parse_simple_condition` each independently implement `in_string`/`string_char`/`paren_depth` state machine. This PR added 3 new functions with this pattern (was 2 in Cycle 1).
- Fix: Extract `scan_bare_bytes()` iterator or callback-based scanner that handles state machine once.
- Impact: Maintainability — divergence risk grows with each new scanner. Deferred from Cycle 1, now worse.

**14. Error messages not checked in two test assertions** — `crates/mds-core/src/parser_tests.rs:1515, 1506` (83% confidence each, TESTING)
- Problem: `evaluate_if_undefined_function_is_error` and `evaluate_for_non_array_result_is_error` assert `result.is_err()` without verifying error message content. Other tests in this PR check error messages; these are inconsistent.
- Fix: Add assertions checking that error message mentions the relevant issue (undefined function, type mismatch).
- Impact: Test quality — would not catch subtle error message regressions.

## Pre-existing Issues (Not Blocking)

**Duplicated quote/paren scanner pattern (8 instances, 3 new in this PR)** — `crates/mds-core/src/parser_helpers.rs` (85% confidence, CONSISTENCY)
- Note: This was deferred in Cycle 1. This PR added 3 new instances, growing from 2 to 4. The pattern is now more of a maintenance concern than in the prior cycle. Not blocking, but flagged for follow-up work.

## Convergence Status

**Cycle**: 2
**Prior Resolution**: Available (13 fixed, 2 false positives, 4 deferred in Cycle 1)
**Prior FP Ratio**: 2/19 = 10.5%
**Assessment**: Mixed convergence — 6 HIGH + 3 MEDIUM blocking issues exceed Cycle 1 expectation. However, only 1 issue is truly new (escape-aware quote detection); most others are recurring deferred items that grew worse with additional code additions.

### Convergence Pattern

- **Recurring from Cycle 1 (still not addressed)**:
  - Duplicated scanner state machines (2 instances → 4 instances, worsened)
  - CondValue/Expr type duplication (unresolved)
  - parse_simple_condition complexity (now + 39 lines for bare `=` scanner)

- **New in Cycle 2**:
  - parse_expr_inner escape-aware quote detection (security concern)
  - @for/@define directive inconsistency (missed refactoring)
  - Test AST structure verification gaps (incomplete testing)

**High FP ratio analysis (10.5%)**: The 2 false positives in Cycle 1 were likely the deferred items that seemed worse than they were in isolation. This cycle's finding rate (9/14 total issues, 64%) is higher, suggesting the review identified genuine issues rather than false alarms.

## Action Plan

**Before Merge:**

1. **Fix parse_expr_inner escape-aware quote detection** (HIGH, Rust) — Add proper boundary check for trailing quote.
2. **Fix split() pre-allocation** (HIGH, Performance) — Use `Vec::with_capacity(64)`.
3. **Fix @for directive_colon_error usage** (HIGH, Consistency) — Use helper for consistent error messages.
4. **Extract has_bare_equals() and reduce parse_simple_condition** (HIGH, Complexity) — Break out scanner into helper.
5. **Add nested call AST verification to test** (HIGH, Testing) — Destructure and assert inner structure.
6. **Add sort order assertion to test** (HIGH, Testing) — Assert `[a, b]` not `[b, a]`.
7. **Extract parse_call_or_qualified_call() and reduce parse_expr_inner** (MEDIUM, Complexity) — Bring function under 100 lines.
8. **Migrate @define to strip_trailing_directive_colon or document exception** (MEDIUM, Consistency).
9. **Replace CondValue with Expr or mark TODO** (MEDIUM, Consistency).

**Recommended Quick Fix:**

Start with the HIGH items (fixes 1-6 above), which are straightforward and unblock the PR. Then tackle MEDIUM items 7-9, which are slightly larger refactors but necessary for code quality.

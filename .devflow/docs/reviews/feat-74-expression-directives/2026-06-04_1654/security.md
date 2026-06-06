# Security Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**split() allocates full array before enforcing MAX_ARRAY_ELEMENTS limit** - `crates/mds-core/src/builtins.rs:260-269`
**Confidence**: 85%
- Problem: `builtin_split` calls `.collect()` on the entire split iterator, materializing the full `Vec<Value>` in memory, and only then checks whether the element count exceeds `MAX_ARRAY_ELEMENTS` (100,000). For adversarial input -- a 10 MB string (the MAX_FILE_SIZE limit) split on a 1-byte separator -- this produces up to ~10 million `Value::String` allocations before the guard fires. Each `Value::String` is at least 24+ bytes overhead, so this could transiently allocate ~240+ MB before the error path discards it.
- Fix: Use a bounded iterator that counts elements and bails early:
  ```rust
  let mut parts: Vec<Value> = Vec::new();
  for p in s.split(sep) {
      if parts.len() >= MAX_ARRAY_ELEMENTS {
          return Err(MdsError::resource_limit(format!(
              "split() produced more than {} elements",
              MAX_ARRAY_ELEMENTS
          )));
      }
      parts.push(Value::String(p.to_string()));
  }
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`parse_expr_inner` string literal matching may accept mismatched quotes** - `crates/mds-core/src/parser_helpers.rs:135-136` (Confidence: 65%) -- The check `s.starts_with('"') && s.ends_with('"')` and the single-quote variant are on separate branches, so a string starting with `"` and ending with `'` is handled correctly (falls through to the unterminated-string error). However, a string like `"hello\"` (escaped trailing quote) would match the first branch and strip the backslash-escaped quote as the closing delimiter, yielding `hello\` as the literal value. In practice this is low risk because `unescape_string` handles the escape, and the input comes from directive content bounded by file size limits.

- **`strip_trailing_directive_colon` does not limit paren_depth** - `crates/mds-core/src/parser_helpers.rs:39` (Confidence: 60%) -- The `paren_depth` counter in `strip_trailing_directive_colon` has no upper bound. While practically harmless because input is bounded by `MAX_FILE_SIZE`, adversarial input with thousands of unclosed `(` characters could drive `paren_depth` to a large value. The `saturating_sub` on `)` prevents underflow correctly. No real exploit path given the file size limit.

- **`evaluate_condition` signature change broadens side-effect scope** - `crates/mds-core/src/evaluator.rs:423-427` (Confidence: 70%) -- `evaluate_condition` now takes `&mut Scope` and `&mut EvalContext` (previously `&Scope`), because expressions in conditions can invoke functions. This is correct and necessary, but it means `@if` conditions can now have side effects (function calls that modify scope or produce output). This is by design per the PR, but worth noting as a deliberate expansion of the condition evaluation model's power.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Security Observations

1. **Resource limits are well-placed**: New `MAX_ARRAY_ELEMENTS` (100K) for `split()` and `MAX_OUTPUT_SIZE` (50MB) check in `join()` are good defense-in-depth additions that directly address the amplification risk from expression-powered iterables.

2. **Parser input validation is thorough**: `parse_expr_inner` validates function names, rejects bare literals in `@if`/`@for` positions, rejects `NaN`/`Infinity`, and checks for unterminated strings. The `is_valid_identifier` check on function names prevents injection of non-identifier characters.

3. **Colon-aware directive parsing**: `strip_trailing_directive_colon` correctly handles colons inside string literals and parenthesized expressions, preventing directive-boundary confusion attacks where embedded colons could truncate conditions.

4. **Recursion/depth limits carry forward**: `MAX_CALL_DEPTH` (128), `MAX_NESTING_DEPTH` (64), `MAX_DOT_SEGMENTS` (32), and `MAX_LOGICAL_OPERANDS` (16) all remain enforced on the new code paths. The argument parser's depth tracking (`parse_args_inner`) prevents stack overflow from deeply nested call expressions.

5. **NaN equality semantics preserved**: `values_equal_runtime` correctly follows IEEE 754 (`NaN != NaN`), preventing type confusion in condition comparisons.

6. **Empty separator rejection**: `builtin_split` rejects empty separators, preventing O(n) character-level splitting that could amplify element count.

### Condition for Approval

Fix the `split()` collect-then-check pattern (the single MEDIUM finding) to avoid transient over-allocation. This is a one-line refactor from `.collect()` to a bounded loop.

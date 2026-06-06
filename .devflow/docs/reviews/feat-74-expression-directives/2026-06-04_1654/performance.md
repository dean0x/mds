# Performance Review Report

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04
**PR**: #76

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**split() allocates full Vec before checking MAX_ARRAY_ELEMENTS limit** - `crates/mds-core/src/builtins.rs:260`
**Confidence**: 85%
- Problem: `builtin_split()` calls `.collect()` to build the entire `Vec<Value>` before checking whether the element count exceeds `MAX_ARRAY_ELEMENTS` (100,000). Since input strings can be up to `MAX_FILE_SIZE` (10 MB), a single-character separator could produce ~10 million `Value::String` elements (each heap-allocated), peaking at hundreds of MB before the guard on line 263 rejects the result and drops the allocation.
- Impact: Transient memory spike on adversarial input. The limit does prevent downstream damage (the Vec is dropped on error), but the allocation itself is the problem. In a WASM environment with limited memory, this could OOM before the guard fires.
- Fix: Use an iterator with an early-exit count check to avoid allocating beyond the limit:
  ```rust
  let mut parts: Vec<Value> = Vec::new();
  for p in s.split(sep) {
      parts.push(Value::String(p.to_string()));
      if parts.len() > MAX_ARRAY_ELEMENTS {
          return Err(MdsError::resource_limit(format!(
              "split() produced more than {} elements",
              MAX_ARRAY_ELEMENTS
          )));
      }
  }
  ```
  This bounds peak allocation to `MAX_ARRAY_ELEMENTS + 1` elements regardless of input size.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Repeated byte-level scanning in parse_simple_condition** - `crates/mds-core/src/parser_helpers.rs:569` (Confidence: 60%) -- The function makes up to 3 passes over the condition string (`find_unquoted_operator`, bare-`=` scan, then `parse_expr_inner`). Each pass tracks the same `in_string`/`paren_depth` state. A single-pass scanner could return all three results at once. However, condition strings are typically under 100 characters, so the practical impact is negligible for a template compiler.

- **String literal clone per condition evaluation** - `crates/mds-core/src/evaluator.rs:173` (Confidence: 65%) -- `Expr::StringLiteral(s) => Ok(Value::String(s.clone()))` allocates a new `String` every time a literal is evaluated. When an `@if` with a string comparison sits inside a large `@for` loop, the literal is cloned per iteration. A `Cow<'_, str>` or pre-interned approach could avoid this, but the strings are typically short and the codebase consistently clones `Value`, making this a codebase-wide pattern rather than a PR-specific issue.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Performance Observations

1. **join() incremental output guard** (`builtins.rs:392`): The output size check runs inside the fold loop, catching oversized output early rather than after full allocation. Well-designed. (applies ADR-008 -- this defensive measure complements the split() limit as part of the bundled expression-directive feature set.)

2. **@for iterable evaluated once** (`evaluator.rs:617`): The expression iterable (`evaluate_expr(&block.iterable, ...)`) runs before the loop body executes, avoiding redundant evaluation per iteration.

3. **Short-circuit &&/|| preserved** (`evaluator.rs:439-468`): Logical conditions continue to short-circuit correctly with the new expression-based operands.

4. **MAX_ARRAY_ELEMENTS limit added** (`limits.rs:53`): The new 100,000-element cap prevents unbounded array creation from `split()`. The limit value is well-chosen for a template compiler.

5. **strip_trailing_directive_colon forward scan** (`parser_helpers.rs:30-86`): Correctly scans once from left to right tracking quotes and parens, avoiding backtracking. The `has_unterminated_string` helper is only called on the error path, so it never adds overhead to the happy path.

### Condition for Approval

The single MEDIUM issue (split collect-then-check) should be addressed to prevent transient memory spikes in constrained environments (WASM). The fix is a straightforward iterator refactor with no behavioral change.

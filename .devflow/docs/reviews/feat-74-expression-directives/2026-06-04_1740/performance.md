# Performance Review Report

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**split() Vec grows without pre-allocation hint** - `builtins.rs:264`
**Confidence**: 82%
- Problem: `Vec::new()` starts with zero capacity. For typical inputs that produce dozens to thousands of parts, this causes repeated reallocations (each doubling the buffer). The prior code used `.collect()` on an iterator which Rust optimizes via `size_hint()` from `str::split()` (the lower bound is always 1, but the iterator cannot predict count). However, the new incremental loop lost even that minimal hint. For inputs near `MAX_ARRAY_ELEMENTS` (100,000), the Vec will reallocate approximately 17 times (log2(100000)).
- Fix: Use `Vec::with_capacity(min(s.matches(sep).count(), MAX_ARRAY_ELEMENTS))` or a reasonable initial capacity estimate. A simpler approach that avoids the O(n) pre-scan:
  ```rust
  let mut parts: Vec<Value> = Vec::with_capacity(64);
  ```
  64 is a reasonable default that avoids the first 6 reallocations for free and covers the vast majority of real-world split() calls. The incremental limit check still protects against adversarial inputs.

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### HIGH

(none)

### MEDIUM

**evaluate_expr clones Value unconditionally for Var lookups** - `evaluator.rs:147-150`
**Confidence**: 80%
- Problem: `evaluate_expr` now returns `Result<Value, MdsError>` (previously returned `Result<String, MdsError>`). For `Expr::Var`, it calls `scope.get_var(name).cloned()` which deep-clones the entire Value. For simple scalars (strings, numbers, booleans) this is cheap. For `Value::Object(HashMap)` or `Value::Array(Vec)` used in conditions (e.g., `@if items:` where items is a large array), this clones the entire structure just to call `.is_truthy()` on it.
  
  This was not an issue before because `evaluate_condition` previously used `resolve_condition_value` which also cloned, but only for the specific condition path -- not for every expression evaluation. Now that `evaluate_expr` is the unified entry point for both interpolation and conditions, every condition evaluation pays the clone cost.
  
  In `evaluate_for` (line 624), `evaluate_expr(&block.iterable, scope, ctx)?` clones the entire iterable (potentially a large array or object) and then `evaluate_for_array` clones it *again* at line 602 (`array.to_vec()`). This is a double-clone of the iterable array.
- Fix: For the double-clone in `evaluate_for`, the second clone at line 602 is necessary for borrow reasons, but the first clone at line 624 could be avoided by having a `evaluate_expr_owned` variant or by matching the iterable expression inline to avoid the intermediate Value. This is a moderate refactor -- the simplest mitigation is to document the known double-clone as accepted overhead. For `is_truthy()` checks in conditions, consider a `evaluate_expr_ref` that returns `&Value` for `Var` lookups, falling back to owned for Call/QualifiedCall.

**join() String grows without capacity estimate** - `builtins.rs:385`
**Confidence**: 80%
- Problem: `String::new()` starts at zero capacity. For arrays with known length, the output size can be estimated as `sum_of_element_lengths + (arr.len() - 1) * sep.len()`. Without a hint, String reallocates repeatedly as elements are appended. For large arrays near the 100K element limit, this causes significant reallocation churn.
- Fix: Pre-estimate capacity based on array length and separator:
  ```rust
  // Estimate: average element ~10 chars + separator between each pair
  let estimated = arr.len().saturating_mul(10 + sep.len());
  let mut out = String::with_capacity(estimated.min(MAX_OUTPUT_SIZE));
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**resolve_dot_path builds error-path strings with Vec allocation per field traversal** - `evaluator.rs:114-119`
**Confidence**: 70% (moved to Suggestions since below 80%)

## Suggestions (Lower Confidence)

- **Duplicated scanner state machines across 4 functions** - `parser_helpers.rs` (multiple locations) (Confidence: 70%) -- `strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op`, and the bare-`=` scanner in `parse_simple_condition` all implement near-identical byte-scanning loops with `in_string`/`string_char`/`paren_depth` tracking. This is noted in PRIOR_RESOLUTIONS as a deferred item. No new performance concern beyond the prior cycle's observation, but consolidation would reduce instruction-cache pressure in the parser hot path.

- **resolve_dot_path error-path string allocation** - `evaluator.rs:114-119` (Confidence: 70%) -- The `traversed_path` closure allocates a Vec and joins it into a String on every field traversal iteration, but only to produce error messages. Since it is a closure, it is only called on error. This is fine for correctness, but the closure is *created* on each iteration even if never called. The optimizer likely elides this, but it is worth verifying under debug builds.

- **Expr::StringLiteral(s.clone()) in evaluate_expr** - `evaluator.rs:173` (Confidence: 65%) -- When evaluating `Expr::StringLiteral(s)` in a condition (e.g., `@if var == "admin":`), the string is cloned to create a `Value::String`. For short literal strings this is negligible. For conditions evaluated inside `@for` loops with many iterations, the same literal is cloned on every iteration. An `Rc<str>` or interning approach could avoid this, but the impact is marginal for realistic template sizes.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 2 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR demonstrates strong performance awareness: incremental element-count limits in `split()`, output-size guards in `join()`, short-circuit evaluation in `&&`/`||`, and reuse of existing bounded iteration limits. The resource-limit hardening (MAX_ARRAY_ELEMENTS, MAX_OUTPUT_SIZE checks in builtins) is a meaningful improvement over the prior code.

The blocking HIGH finding (split() Vec pre-allocation) is a straightforward improvement that avoids unnecessary reallocations. The should-fix items (evaluate_expr clone cost, join() capacity hint) are moderate optimizations that become relevant at scale. The duplicated scanner state machines were already identified and deferred in the prior review cycle -- no new action required this cycle.

Conditions for approval: Address the split() pre-allocation (HIGH blocking). The should-fix items are recommended but not blocking given the project's early stage (applies ADR-008 -- bundling related improvements is preferred over incremental changes).

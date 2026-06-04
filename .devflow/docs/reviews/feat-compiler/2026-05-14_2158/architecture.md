# Architecture Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**Commits reviewed**: 1ac9848...7c49fc5 (4 commits)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Duplicated export-visibility filter logic (3 occurrences)** - Confidence: 85%
- `src/resolver.rs:467-471` (`get_export`), `src/resolver.rs:480` (`get_all_exports`), `src/resolver.rs:503-506` (`to_namespace`)
- Problem: The predicate `!self.has_explicit_exports || self.explicit_exports.contains(name)` is duplicated verbatim in three methods: `get_export`, `get_all_exports`, and `to_namespace`. This is a Single Responsibility / DRY concern -- if the export visibility rule ever changes (e.g. adding glob patterns, default exports), all three sites must be updated in lockstep, which is error-prone. The `to_namespace` method introduced in this PR adds a third copy of the same logic.
- Fix: Extract a private `is_exported(&self, name: &str) -> bool` helper and call it from all three methods:
  ```rust
  fn is_exported(&self, name: &str) -> bool {
      !self.has_explicit_exports || self.explicit_exports.contains(name)
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Duplicated double-fault error-preservation pattern** - `src/evaluator.rs:200-208` and `src/evaluator.rs:299-306` - Confidence: 82%
- Problem: The identical `match (result, pop_result)` block for double-fault error preservation appears in both `invoke_function` and `evaluate_for`. The PR description explicitly calls this a "pattern" applied consistently, which is good, but the identical 5-line match block is a candidate for extraction. If the precedence rule changes (e.g. to chain errors instead of dropping one), both sites must be updated.
- Fix: Extract a helper function:
  ```rust
  fn prefer_render_error<T>(render: Result<T, MdsError>, pop: Result<(), MdsError>) -> Result<T, MdsError> {
      match (render, pop) {
          (Err(render_err), _) => Err(render_err),
          (Ok(_), Err(pop_err)) => Err(pop_err),
          (Ok(val), Ok(())) => Ok(val),
      }
  }
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`ModuleCache` accumulates multiple responsibilities** - `src/resolver.rs` - Confidence: 80%
- Problem: `ModuleCache` owns security validation (`canonicalize_and_check`), file I/O (`read_validated_file`), caching (`modules` HashMap), cycle detection (`resolving` IndexSet), root directory tracking, module processing orchestration (`process_module`), AST walking (`collect_definitions_and_imports`), and import resolution (`resolve_import`). This is trending toward a god-struct pattern -- eight distinct concerns in one type. The changes in this PR (splitting `validate_and_read_file` into two methods) are a step in the right direction, but the overall surface area remains wide.
- Fix: Consider extracting `SecurityChecker` (canonicalize, symlink, path traversal, file size) and leaving `ModuleCache` responsible only for caching, cycle detection, and orchestration. Not urgent for a v0.1, but worth tracking.

**`prompt_body` export visibility check duplicated between `get_prompt_value` and `to_namespace`** - `src/resolver.rs:487-493` and `src/resolver.rs:510-516` - Confidence: 80%
- Problem: The `prompt_is_exported` boolean is computed identically in `get_prompt_value` and `to_namespace`. If the proposed `is_exported` helper is added (see Blocking section), these would naturally use `self.is_exported("prompt")`.

## Suggestions (Lower Confidence)

- **`CapturedScope` cloning overhead** - `src/evaluator.rs:178-179` (Confidence: 65%) -- Each captured function is cloned from owned `FunctionDef` then wrapped in a new `Arc` on every function invocation. With large closure captures in hot loops this could be expensive, but likely not an issue at v0.1 scale.

- **`CollectedDefs` could derive `Default`** - `src/resolver.rs:525-529` (Confidence: 60%) -- The new struct has all defaultable fields; a `Default` derive would allow simpler initialization in tests if needed, but the current explicit construction in `collect_definitions_and_imports` is clear enough.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The changes in this PR demonstrate strong architectural judgment overall:

1. **Resolver decomposition** -- Splitting `validate_and_read_file` into `canonicalize_and_check` + `read_validated_file` correctly separates the cheap security-check path (cache hits) from the expensive I/O path (cache misses). This follows the deep-modules principle -- the public `resolve()` method's interface stays simple while the internal decomposition improves efficiency.

2. **`CollectedDefs` struct** -- Replacing the 3-tuple `(HashMap, bool, HashSet)` with a named struct significantly improves readability at both the return site and the destructuring call site. Good application of self-documenting types.

3. **`IndexSet` pop() vs shift_remove()** -- Using `pop()` instead of `shift_remove()` for the LIFO resolving stack is a correct O(1) optimization that also better communicates the LIFO intent.

4. **Double-fault error-preservation pattern** -- The consistent `match (result, pop_result)` pattern in both `evaluate_for` and `invoke_function` is a well-reasoned approach to error precedence. The render error carries user-actionable diagnostics while the pop error signals a compiler bug -- preserving the render error is the right call.

5. **`assert!` promotion from `debug_assert!`** -- Promoting the LIFO invariant check to a release-mode `assert!` in `invoke_function` is architecturally sound -- the cost is negligible at MAX_CALL_DEPTH=128 and the invariant is safety-critical for recursion detection.

The one condition for approval: extract the export-visibility predicate into a shared helper to avoid the 3-way duplication that this PR introduced (via `to_namespace`). The remaining suggestions are minor and can be addressed in follow-up work.

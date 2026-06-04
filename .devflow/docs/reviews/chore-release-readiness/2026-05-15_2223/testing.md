# Testing Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**output_size_limit test allocates 50 MB in-process** - `src/evaluator.rs:622`
**Confidence**: 85%
- Problem: The `output_size_limit_rejects_oversized_output` test creates a 50 MB string with `"x".repeat(MAX_OUTPUT_SIZE + 1)`. While the comment says "to avoid allocating 50 MB of actual text," it actually does allocate exactly that much. This makes the test suite memory-hungry and slow. In CI environments with constrained memory or when running with `cargo test` (which parallelizes by default), this can cause OOM issues or significant slowdowns.
- Fix: Use a loop of smaller text nodes that accumulate past the limit, or use a `@for` loop with a large array to test the limit incrementally rather than a single 50 MB allocation:
  ```rust
  // Strategy: two text nodes that together exceed the limit
  let half = MAX_OUTPUT_SIZE / 2 + 1;
  let nodes = vec![text(&"x".repeat(half)), text(&"x".repeat(half))];
  ```
  This uses ~25 MB per allocation (two allocations) instead of one contiguous 50 MB block, and more closely tests the check-after-each-node behavior.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**warning_cap test asserts cap but not exact boundary** - `tests/integration.rs:3186-3194`
**Confidence**: 82%
- Problem: The `warning_cap_at_max_warnings` test generates 1010 `@include` warnings and asserts `warnings.len() <= 1000`. It does not verify that warnings are collected up to the cap -- it would pass even if the cap were 500 or if warnings stopped at 10. The test name implies it validates the boundary at exactly 1000, but the assertion is one-sided.
- Fix: Add a tighter lower-bound assertion to confirm warnings are collected up to the cap:
  ```rust
  assert_eq!(
      warnings.len(), 1000,
      "warnings must be capped at exactly 1000, got {}",
      warnings.len()
  );
  ```
  Or if the exact count depends on evaluation behavior, at minimum assert `warnings.len() >= 100` as a floor to prove collection is working.

**call_depth_limit test error assertion is loose** - `src/evaluator.rs:606-609`
**Confidence**: 80%
- Problem: The assertion checks for "call depth" OR "recursion" OR "128" in the error message. The test constructs a chain of MAX_CALL_DEPTH + 2 non-recursive functions, but because each function calls a different function (not itself), the first thing that fires is the `ctx.call_stack.len() >= MAX_CALL_DEPTH` check in `invoke_function` (line 175-178), which produces an error containing "call depth exceeds 128". The disjunction `|| err.contains("recursion")` is misleading since this is not a recursion detection test -- it is a call-depth limit test. The "128" check would also match unrelated numbers.
- Fix: Tighten to the expected message:
  ```rust
  assert!(
      err.contains("call depth"),
      "error should mention call depth limit, got: {err}"
  );
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No unit tests for resolver's extracted security helpers** - `src/resolver.rs:74,99,109`
**Confidence**: 82%
- Problem: The extracted `check_symlink`, `check_import_depth`, and `check_path_traversal` methods are covered only indirectly through integration tests. Since these are now independent methods with clear contracts, they would benefit from focused unit tests that document their boundary behavior (e.g., `check_import_depth` at exactly MAX_IMPORT_DEPTH, `check_path_traversal` with a path exactly at root boundary).
- Note: Integration tests do cover these paths, so this is not a correctness gap, but the refactoring into standalone methods creates a natural opportunity for targeted unit tests.

**No unit tests for run_build / run_check / run_init** - `src/main.rs:447,518,555`
**Confidence**: 80%
- Problem: The CLI logic was extracted from `run()` into `run_build`, `run_check`, and `run_init` but these functions have no direct unit tests. They are tested indirectly via CLI integration tests using `mds_bin()`, but the extraction creates testable units that could be covered more efficiently with unit tests (especially error paths like directory rejection and path traversal in `run_init`).
- Note: The CLI integration tests provide adequate coverage for release. Unit tests would improve feedback speed.

### LOW

**Validator tests do not cover push/pop behavior change** - `src/validator.rs:59-63,66-75`
**Confidence**: 80%
- Problem: The validator was changed from `scope.clone()` to `scope.push()`/`scope.pop()` for `@for` and `@define` blocks. The existing two unit tests verify basic define-body validation but do not verify that the push/pop does not leak scope (e.g., a variable defined inside a `@for` body should not be visible after the loop). This behavioral invariant was previously guaranteed by clone semantics and is now maintained by explicit pop.
- Note: Integration tests exercise this behavior through full compilation paths. A focused unit test would make the push/pop contract explicit.

## Suggestions (Lower Confidence)

- **YAML/JSON depth tests relocated without boundary verification** - `src/value.rs:261-303` (Confidence: 72%) -- The moved tests check depth 65 (one over the 64 limit) but do not verify that depth 64 succeeds. A boundary test at exactly the limit would strengthen confidence in the off-by-one correctness.

- **Integration test nesting depth test does not verify boundary** - `tests/integration.rs:3137-3156` (Confidence: 68%) -- `parser_nesting_depth_limit_rejects_deep_nesting` tests 257 levels (one past 256) but does not verify 256 levels succeeds. The one-sided test would pass even if the limit were 200.

- **MAX_OUTPUT_SIZE test comment contradicts behavior** - `src/evaluator.rs:616-618` (Confidence: 75%) -- The comment says "We use a pre-sized string just over the limit to avoid allocating 50 MB" but the code allocates exactly `MAX_OUTPUT_SIZE + 1` bytes (50 MB + 1 byte), which does allocate 50 MB. The misleading comment could confuse future maintainers.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 2 | 1 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new resource limit tests (call depth, output size, nesting depth, warning cap, directory rejection, path traversal) represent a meaningful improvement to test coverage for security and robustness boundaries. The relocation of YAML/JSON depth tests from integration to unit tests is the correct response to the `pub(crate)` visibility change. All 292 tests pass.

Conditions:
1. Fix the misleading comment in the output_size_limit test (or restructure the test to avoid the 50 MB single allocation)
2. Tighten the warning_cap assertion to verify the boundary is actually at 1000 (not just <= 1000)

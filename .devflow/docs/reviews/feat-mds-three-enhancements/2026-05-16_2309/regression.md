# Regression Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Evaluator uses panic-inducing `assert!` instead of Result for parser invariants** - `src/evaluator.rs:321,339`
**Confidence**: 82%
- Problem: The evaluator uses `assert!(!block.condition.is_empty())` and `assert!(!block.iterable.is_empty())` which will panic in production if the invariant is violated. Meanwhile, the validator (which runs before the evaluator) already uses the safe pattern `.first().ok_or_else(|| MdsError::syntax(...))` for the same check. This creates an inconsistency: if the validator is bypassed (e.g., direct AST construction in tests or future internal use), the evaluator would panic rather than returning a recoverable error.
- Fix: Replace `assert!` with the same `.first().ok_or_else()` pattern used in the validator:
  ```rust
  // In evaluate_if (line 321):
  let root = block.condition.first().ok_or_else(|| {
      MdsError::syntax("internal error: @if block has empty condition path")
  })?;
  let value = resolve_dot_path(root, &block.condition[1..], scope)?;

  // In evaluate_for (line 339):
  let root = block.iterable.first().ok_or_else(|| {
      MdsError::syntax("internal error: @for block has empty iterable path")
  })?;
  let iterable = resolve_dot_path(root, &block.iterable[1..], scope)?;
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing integration test for namespace-vs-object disambiguation error** - `src/evaluator.rs:158-161`
**Confidence**: 85%
- Problem: The old test `dot_notation_variable_access_gives_clear_error` was removed (it tested that `{u.greet}` on a namespace produced a clear error). The replacement test `dot_notation_object_access_works` only tests the happy path (object field access). The evaluator code at line 158-161 handles the namespace case with a targeted error message, but no integration test exercises this path. If this code path regresses, no test will catch it.
- Fix: Add an integration test verifying the namespace-vs-object error:
  ```rust
  #[test]
  fn member_access_on_namespace_gives_clear_error() {
      let dir = tempfile::tempdir().unwrap();
      let lib = dir.path().join("lib.mds");
      let main = dir.path().join("main.mds");
      std::fs::write(&lib, "@define greet(n):\nHi {n}!\n@end\n").unwrap();
      std::fs::write(&main, "@import \"./lib.mds\" as u\n{u.greet}\n").unwrap();
      let result = mds::compile(&main, None);
      assert!(result.is_err());
      let err = format!("{}", result.unwrap_err());
      assert!(
          err.contains("imported module") || err.contains("not a variable"),
          "should explain namespace vs object distinction, got: {err}"
      );
  }
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing regression issues identified.

## Suggestions (Lower Confidence)

- **Validator error span narrows from full condition to root only** - `src/validator.rs:36` (Confidence: 65%) — The validator's undefined-variable error span changed from `block.condition.len()` (full string length like "config.debug") to `root.len()` (just "config"). For simple variables this is identical, but for dot-paths the error underline is narrower. This is likely intentional (only the root is the undefined var), but could confuse users if the full path is shown in the message but only the first segment is underlined.

- **Output format change may surprise library consumers** - `src/lib.rs:254,278` (Confidence: 62%) — Templates with frontmatter now include the frontmatter in compiled output (previously only the body was emitted). While v0.1.0 semver permits breaking changes, any downstream tools parsing the output may need updating. The PR description mentions this as an intentional feature (E2: frontmatter preservation).

- **No deprecation path for removed error behaviors** - `tests/integration.rs:1537-1542,1701-1707` (Confidence: 60%) — Two tests that previously asserted errors (YAML maps rejected, dot-notation without parens was error) now assert success. Any documentation or tooling that relied on these being errors has no migration path beyond reading the PR description.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The AST type migration (String -> Vec<String>) is complete across all consumers. All 336 tests pass with zero warnings. The main regression concern is the evaluator's use of `assert!` (panicking) rather than Result-based error handling for the same invariant that the validator protects with a graceful error. While the parser guarantees the invariant holds, using `assert!` in non-test production code violates the "never panic in business logic" principle and creates an inconsistency with the validator's approach. The missing test for namespace disambiguation is a coverage gap from removing the old test without adding an equivalent for the error path.

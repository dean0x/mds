# Complexity Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**`resolve_by_key` is 76 lines with high cyclomatic complexity** - `resolver.rs:122-197`
**Confidence**: 85%
- Problem: This function has 7 decision points (cache hit check, cycle detection check, depth guard, 4 error-handling branches in the LIFO double-fault match) and spans 76 lines including comments. While the comments are excellent and each step is well-labeled, the function does cache lookup, cycle detection, depth guarding, file reading, type validation, context construction, module processing, LIFO invariant checking with double-fault handling, caching, and return -- at least 6 distinct responsibilities in one function body. The LIFO double-fault match (lines 185-189) adds a second layer of error handling on top of the normal flow, increasing cognitive load.
- Fix: Consider extracting the LIFO-check-and-unwrap into a helper method:
  ```rust
  /// Pop the resolving stack and verify LIFO invariant.
  /// Returns the process_module result if both succeed, or the
  /// appropriate error (preferring module errors on double-fault).
  fn pop_and_unwrap(
      &mut self,
      key: &str,
      result: Result<ResolvedModule, MdsError>,
  ) -> Result<ResolvedModule, MdsError> {
      let popped = self.resolving.pop();
      if popped.as_deref() != Some(key) {
          // On double-fault, prefer the module error
          if let Err(e) = result { return Err(e); }
          return Err(MdsError::syntax(format!(
              "internal error: resolving stack LIFO invariant violated ..."
          )));
      }
      result
  }
  ```
  This would reduce `resolve_by_key` to ~45 lines and isolate the LIFO concern.

**`collect_export` has 5 parameters plus high nesting** - `resolver.rs:348-404`
**Confidence**: 82%
- Problem: This method takes 5 parameters (`&mut self`, `export`, `defs`, `ctx`, `warnings`) and has a 3-arm match with nesting depth 4 in the `Wildcard` arm (fn -> match -> for -> if). While each arm is individually clear, the combination of parameter count and nesting pushes toward the boundary of comfortable readability.
- Fix: The 5-parameter count is borderline (threshold is 5+). Since `ctx` and `warnings` are threaded through every method, this is more of a structural observation than a refactoring target. Acceptable if other complexity metrics stay clean.

**`resolve_selective_import` has 7 parameters** - `resolver.rs:447-489`
**Confidence**: 90%
- Problem: This function takes 7 parameters: `&mut self`, `names`, `path`, `offset`, `scope`, `ctx`, `warnings`. The parameter list itself is 8 lines long (lines 448-455). With `ctx` already bundling 4 fields (`file_str`, `source`, `base_key`, `runtime_vars`), having `path`, `offset`, and `scope` as additional parameters suggests these could be bundled too. The same 7-parameter pattern appears in `resolve_alias_import` and `resolve_merge_import` (6 params each, also above the threshold).
- Fix: Bundle the import-specific parameters into an `ImportCtx` or add `offset` and `path` to the `ModuleCtx` struct. Alternatively, since `resolve_import` already destructures `ImportDirective` into these fields, consider keeping the match inline in `resolve_import` and moving each arm's body to a helper that takes `(ImportDirective, scope, ctx, warnings)` instead of destructured fields:
  ```rust
  // Instead of destructuring in resolve_import and passing 6-7 params,
  // pass the directive variant directly:
  fn resolve_alias_import(
      &mut self,
      import: &ImportDirective, // contains path, alias, offset
      scope: &mut Scope,
      ctx: &ModuleCtx<'_>,
      warnings: &mut Vec<String>,
  ) -> Result<(), MdsError> { ... }
  ```

### MEDIUM

**`VirtualFs::normalize` has nesting depth 4** - `fs.rs:80-128`
**Confidence**: 80%
- Problem: Lines 102-118 contain a `for` loop with a `match` inside it, where the `".."` arm has an `if` guard -- yielding 4 levels of nesting (fn -> if/for -> match -> if). The function is 48 lines total, approaching the 50-line warning threshold for Rust production code.
- Fix: This function is well-structured with early returns and the logic is inherently about path segment resolution, so the nesting is justified by the problem domain. No action required unless the function grows further.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`process_module` is 42 lines, acceptable but approaching threshold** - `resolver.rs:272-313`
**Confidence**: 65% (reported in Suggestions)

## Pre-existing Issues (Not Blocking)

_No pre-existing complexity issues at CRITICAL severity in unchanged code._

## Suggestions (Lower Confidence)

- **`process_module` pipeline length** - `resolver.rs:272-313` (Confidence: 65%) -- At 42 lines with 8 sequential pipeline steps (tokenize, parse, capture frontmatter, build scope, collect definitions, validate exports, validate semantics, evaluate), this is a well-organized orchestration function. The length is justified by the pipeline nature, but any further growth should trigger extraction.

- **Parameter threading pattern across import resolution** - `resolver.rs:406-513` (Confidence: 70%) -- The trio of `resolve_alias_import`, `resolve_merge_import`, and `resolve_selective_import` all share the pattern of `(path, offset, scope, ctx, warnings)` threading. This is a code smell suggesting a missing abstraction, but the current decomposition is already a significant improvement over the pre-refactor state where all logic was inline.

- **`fs.rs` file length at 709 lines** - `fs.rs` (Confidence: 62%) -- The file is 709 lines, above the 500-line critical threshold. However, 388 of those lines (55%) are unit tests in the `#[cfg(test)] mod tests` block. The production code portion is ~320 lines, which is well within acceptable limits.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 3 | 1 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

This PR is a significant net improvement in complexity. The refactoring from hardcoded OS calls to the `FileSystem` trait abstraction *reduced* overall complexity by:

1. **Extracting `NativeFs` concerns** from `ModuleCache` into a dedicated struct, giving each type a single responsibility.
2. **Introducing `ModuleCtx`** to bundle 4 related parameters, reducing the arity of `process_module` from 6 parameters to 3.
3. **Introducing `resolve_import_from`** to centralize import validation and normalization, eliminating 3 instances of duplicated `validate_import_path` + `resolve_path` calls.
4. **Extracting `has_type_mds_frontmatter`** as a reusable helper from an inline block.

The remaining complexity findings are at HIGH and MEDIUM severity:
- The 7-parameter `resolve_selective_import` should be addressed before more import variants are added.
- The `resolve_by_key` LIFO double-fault handling adds cognitive load that could be extracted into a helper.
- None of these are merge-blocking on their own, but addressing the parameter count issue would make the codebase more maintainable.

**Condition**: Consider bundling the destructured import parameters to bring `resolve_selective_import` below the 5-parameter threshold. This can be done in a follow-up.

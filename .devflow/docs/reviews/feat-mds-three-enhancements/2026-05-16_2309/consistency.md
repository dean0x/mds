# Consistency Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent invariant enforcement pattern between evaluator and validator** - `src/evaluator.rs:321`, `src/evaluator.rs:339`
**Confidence**: 95%
- Problem: The evaluator uses `assert!(!block.condition.is_empty())` and `assert!(!block.iterable.is_empty())` which panics in release mode. However, commit `72096c1` in this same branch explicitly changed the validator from `debug_assert!+index` to `.first().ok_or_else()` "for release safety" (validator.rs lines 27-29, 53-55). The validator comments state: "Use .first() with an error return rather than a debug_assert!+index so this holds in release builds too." The evaluator contradicts this by using a panicking `assert!` for the exact same invariant on the exact same data types (`IfBlock.condition`, `ForBlock.iterable`).
- Fix: Replace `assert!` with the same `.first().ok_or_else()` pattern used in the validator:
  ```rust
  // In evaluate_if:
  let root = block.condition.first().ok_or_else(|| {
      MdsError::syntax("internal error: IfBlock.condition is empty")
  })?;
  let value = resolve_dot_path(root, &block.condition[1..], scope)?;

  // In evaluate_for:
  let root = block.iterable.first().ok_or_else(|| {
      MdsError::syntax("internal error: ForBlock.iterable is empty")
  })?;
  let iterable = resolve_dot_path(root, &block.iterable[1..], scope)?;
  ```

### MEDIUM

**KNOWLEDGE.md states non-string YAML keys are "silently skipped" but code returns an error** - `.features/mds-compiler/KNOWLEDGE.md` (line 156 in diff context)
**Confidence**: 92%
- Problem: The updated KNOWLEDGE.md (from commit `8aaceca`) states: "Non-string YAML keys are silently skipped (YAML allows non-string keys; MDS does not)". However, commit `565f438` in this same branch changed the behavior to reject non-string keys with a clear diagnostic error (`return Err(MdsError::yaml_error(...))`). The code at `src/value.rs:84-89` explicitly returns an error with the message "MDS only supports string keys in objects; found {type} key". The documentation contradicts the implementation.
- Fix: Update the KNOWLEDGE.md entry to:
  ```
  - Non-string YAML keys produce a clear diagnostic error (YAML allows non-string keys; MDS does not)
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`resolve_dot_path` error messages inconsistently include root variable name** - `src/evaluator.rs:109-111`
**Confidence**: 82%
- Problem: The "field not found" error message includes the root variable name: `"field '{field}' not found on object '{root}'"`. However, for multi-level paths like `{a.b.c}` where `a.b` is valid but `c` is not found, the error says "field 'c' not found on object 'a'" which is misleading -- the object is actually `a.b`, not `a`. The "cannot access field" error at line 116 omits the root entirely, only showing the type name. These two error patterns are inconsistent with each other and the second one loses context about which variable was being traversed.
- Fix: Consider including the full path traversed so far in both error messages for consistent diagnostics:
  ```rust
  // For field not found:
  format!("field '{field}' not found on object (traversing '{root}')")
  // For type mismatch:
  format!("cannot access field '{field}' on {} (traversing '{root}')", current.type_name())
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing issues at CRITICAL severity.

## Suggestions (Lower Confidence)

- **`resolve_dot_path` lacks `_at` source span** - `src/evaluator.rs:100-123` (Confidence: 70%) -- The function uses `MdsError::syntax(...)` and `MdsError::undefined_var(...)` (bare constructors without source location) even though the caller sites have access to `block.offset`. The codebase convention (documented in KNOWLEDGE.md and feature knowledge) prefers `_at` variants when source offsets are available. However, `resolve_dot_path` deliberately does not accept offset parameters, and the callers could wrap errors with span info if needed, so this may be an intentional tradeoff for simplicity.

- **Key iteration in `evaluate_for` uses `MdsError::syntax` where `MdsError::type_error` would be more consistent** - `src/evaluator.rs:347-350` (Confidence: 65%) -- The single-var-on-object path at line 387 also uses `MdsError::syntax(...)` rather than `MdsError::type_error(...)` which is the established pattern for type mismatches (used at line 394 for the non-array case). However, the syntax error carries a more helpful message with usage guidance.

- **`EscapedBrace` only produces `{` in evaluator** - `src/evaluator.rs:64` (Confidence: 62%) -- The spec update (line 65 in diff) documents that `\}` produces a literal `}` in output, but `Node::EscapedBrace` still only pushes `{`. This may be handled at the lexer level rather than the evaluator, making it consistent at the token level even if the evaluator's handling looks one-sided.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The overall consistency of the PR is high. New code follows existing patterns well: `#[must_use]` on public API functions, warnings via `&mut Vec<String>`, errors using `MdsError` constructors, `pub(crate)` visibility for internal helpers, exhaustive match arms on enums, and the resolver-as-orchestrator pattern. The `Expr::MemberAccess` and `Arg::MemberAccess` variants are properly mirrored in all three match sites (parser, evaluator, validator) as the codebase requires.

The one blocking consistency issue is the contradictory approach to invariant enforcement: the validator was specifically hardened in this branch to avoid panics in release mode, but the evaluator introduces new `assert!` calls for the same invariants. This should be harmonized before merge.

# Reliability Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Resolver LIFO invariant check occurs after early return on process_module error** - `src/resolver.rs:212-222`
**Confidence**: 85%
- Problem: When `process_module` returns `Err`, line 216 (`let resolved = resolved?;`) performs an early return. The LIFO invariant check at line 218 never executes. While `self.resolving.pop()` at line 212 does run unconditionally (good), the invariant *assertion* that the popped value matches `canonical` is silently skipped on the error path. If a bug were to corrupt the resolving stack during a failed `process_module` call, the LIFO violation would go undetected and the resolving set would be left in an inconsistent state for subsequent resolve calls.
- Fix: Move the LIFO invariant check before the early return. The `prefer_first_error` pattern used in the evaluator's `invoke_function` (lines 208-220 of evaluator.rs) is the correct model for this exact scenario. Apply the same pattern here:
```rust
let popped = self.resolving.pop();
let lifo_result = if popped.as_ref() == Some(&canonical) {
    Ok(())
} else {
    Err(MdsError::syntax(
        "internal error: resolving stack LIFO invariant violated -- this is a compiler bug, please report it",
    ))
};
// Prefer the module processing error over the LIFO violation.
let resolved = match (resolved, lifo_result) {
    (Err(e), _) => return Err(e),
    (Ok(_), Err(lifo_err)) => return Err(lifo_err),
    (Ok(resolved), Ok(())) => resolved,
};
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Validator `@if` body validates against shared mutable scope -- nested defines leak into sibling branches** - `src/validator.rs:23-38`
**Confidence**: 82%
- Problem: The `@if` handler at line 34 validates `then_body` and `else_body` against the same `&mut Scope`. If the `then_body` contains a `@define`, the validator's `Node::Define` handler (line 65-75) pushes a scope, injects params, validates the body, and pops -- which is correct and does not leak. However, the `@if` block does not push/pop its own scope frame, meaning any `@define` at the top level of the then-body would register params in the *same* scope frame used for else-body validation. This matches the evaluator's behavior (the evaluator also does not push a scope for `@if`), so it is semantically correct. But it means a variable defined inside `then_body` via some future mechanism (e.g., a hypothetical `@let`) would incorrectly be visible to `else_body` validation. Since no such construct exists in v0.1, this is not a current bug, but it is a latent coupling.
- Fix: No action required for v0.1. Document the invariant: `@if` does not push a scope frame because no directive within `@if` creates bindings at the `@if` body level (defines have their own scope). If `@let` is ever added, revisit.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **MAX_TRAVERSAL_DEPTH duplicated as separate constants** - `src/main.rs:29`, `src/resolver.rs:47` (Confidence: 65%) -- Both modules define `const MAX_TRAVERSAL_DEPTH: usize = 256` independently. While the PR description notes this is intentional (separate modules), a shared constant or at least a cross-reference comment would prevent accidental drift if one is changed without the other.

- **`load_config` TOCTOU fix reads full file into memory before size check** - `src/main.rs:59-62` (Confidence: 60%) -- The TOCTOU fix is correct: reading bytes first then checking `bytes.len()` eliminates the race. However, this means a maliciously large `mds.json` (e.g., 4 GB) would be fully read into memory before rejection. Since `MAX_CONFIG_SIZE` is 1 MB and the read target is specifically `mds.json` (not user-provided input paths), the practical risk is very low -- an attacker who can place a 4 GB `mds.json` in the project tree already has write access.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

This branch is a strong reliability improvement over `main`. The changes systematically harden the compiler:

1. **TOCTOU fix in `load_config`** -- Reads bytes first, then checks size. Eliminates the metadata-then-read race. Correct and well-documented.
2. **Panic path elimination** -- `names[0]` replaced with `names.first().map(...).unwrap_or(...)` in `auto_detect_mds_file`. Eliminates a panic on an impossible-but-defensive edge case.
3. **`assert_eq!` to Result-based LIFO check in resolver** -- Replaces a panic (`assert_eq!`) with a structured `MdsError::syntax` return, matching the same pattern already used in the evaluator. This is the right direction, though the LIFO check is currently skipped on the error path (the one MEDIUM blocking finding above).
4. **Named constants** -- Magic number `256` replaced with `MAX_TRAVERSAL_DEPTH` in both `load_config` and `find_project_root`. Improves readability and auditability.
5. **Validator push/pop** -- Scope cloning replaced with push/pop in validator `@for` and `@define` handlers. The `let _ = scope.pop()` is safe because it immediately follows a `scope.push()` -- the `pop()` cannot fail. Well-commented.
6. **`#[non_exhaustive]`** on `MdsError` and `Value` -- Prevents external crate exhaustive matching, allowing future variant additions without semver breaks. Good API hygiene.
7. **`pub(crate)` on constructors and converters** -- Reduces public API surface. Prevents external code from constructing error/value types in ways that bypass internal invariants.
8. **Comprehensive resource limit tests** -- New tests for `MAX_CALL_DEPTH`, `MAX_OUTPUT_SIZE`, `MAX_NESTING_DEPTH`, and `MAX_WARNINGS` cap. All pass.

The one blocking finding (LIFO check skipped on error path) is a correctness gap in the invariant verification, not a runtime risk -- the pop itself always executes. The condition to fix is straightforward.

# Regression Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**LIFO invariant check now runs after `resolved?` -- bug in error processing hides LIFO violation on module failure** - `src/resolver.rs:212-222`
**Confidence**: 82%
- Problem: The old code used `assert_eq!(popped.as_ref(), Some(&canonical), ...)` which ran unconditionally before processing the module result. The new code calls `let resolved = resolved?;` on line 216 BEFORE checking the LIFO invariant on line 218. If `process_module` returns an `Err`, the function returns early at line 216 and the LIFO invariant check on lines 218-222 never executes. In the error path, `popped` was already removed from `self.resolving` (line 212), so the IndexSet is still in a consistent state -- but a genuine LIFO violation (a compiler bug) would be silently hidden whenever the module also fails. The old assert would have caught this double-fault.
- Impact: If a LIFO invariant violation ever occurs simultaneously with a module processing error, the compiler bug would go undetected. The user sees the module error, but the corrupted cycle-detection state could cause subsequent resolves in the same `ModuleCache` to behave incorrectly. In practice, this is mitigated because each `ModuleCache` is created per top-level compile call.
- Fix: Check the LIFO invariant before returning the module error, or log a warning on LIFO mismatch even when returning the module error:
```rust
let popped = self.resolving.pop();
let lifo_ok = popped.as_ref() == Some(&canonical);

// Prefer the module error, but still detect LIFO bugs.
let resolved = resolved?;

if !lifo_ok {
    return Err(MdsError::syntax(
        "internal error: resolving stack LIFO invariant violated -- this is a compiler bug, please report it",
    ));
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`#[non_exhaustive]` on `Value` and `MdsError` is a semver commitment before first publish** - `src/value.rs:9`, `src/error.rs:21` (Confidence: 65%) -- Adding `#[non_exhaustive]` prevents downstream crates from exhaustively matching. Since this is v0.1.0 (not yet published to crates.io), this is the correct time to add it. However, the CHANGELOG does not explicitly call out `#[non_exhaustive]` as a design decision. If any pre-release consumers exist (e.g., internal users matching on `Value` or `MdsError` variants without a wildcard), they will get a compile error. Low risk given this is the initial release.

- **`pub(crate)` on `MdsError` constructors and `Value::from_yaml`/`from_json` narrows the API surface** - `src/error.rs:179-462`, `src/value.rs:35,68` (Confidence: 70%) -- These were previously `pub` and are now `pub(crate)`. The integration tests that called `mds::Value::from_yaml` and `mds::Value::from_json` have been correctly relocated to unit tests inside `src/value.rs`. No external consumers exist yet (v0.1.0 unpublished). The migration is complete: no test or code path in the repo still attempts to access these as public API.

- **`assert!` replaced with soft error for LIFO invariant in resolver** - `src/resolver.rs:218-222` (Confidence: 60%) -- The old `assert_eq!` would panic on a LIFO violation, making the bug immediately visible. The new code returns an `MdsError::syntax(...)` error, which is a softer signal. In production this is more user-friendly, but in development/testing it could allow a compiler bug to be caught later as a confusing downstream error rather than an immediate panic with a clear stack trace. This is a deliberate design choice for release readiness.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions

1. Consider whether the LIFO invariant check ordering in `resolver.rs:212-222` should be evaluated before the early-return on module error (MEDIUM should-fix above). The current code path silently skips the check on error, which is acceptable given single-use `ModuleCache` lifetime but deviates from the original safety-critical assertion behavior.

### What Went Well

- **Complete test migration**: The YAML/JSON depth limit tests that previously called `mds::Value::from_yaml`/`from_json` (public API) have been correctly relocated to unit tests in `src/value.rs` now that those methods are `pub(crate)`. No test coverage was lost.
- **Behavioral equivalence of extracted methods**: The `resolve_import` refactoring into `resolve_alias_import`, `resolve_merge_import`, and `resolve_selective_import` preserves exact behavior. Each extracted method contains the same logic as the original match arm, with the `Ok(())` now returned from each method rather than after the match.
- **Validator signature change is safe**: The `validate()` signature change from `&Scope` to `&mut Scope` with push/pop instead of clone is purely a performance optimization. The push/pop pattern correctly mirrors the scope behavior of the old clone approach, and both call sites (resolver and unit tests) have been updated.
- **CLI refactoring preserves all behavior**: `run_build` preserves the auto-detect "Building" banner, `run_check` preserves the absence of such a banner, and `run_init` preserves path traversal rejection. The `resolve_input` helper is correctly shared only by `run_check` (which has no banner), not `run_build`.
- **All 292 tests pass** on the branch, including the newly added resource limit tests.
- **No deleted files or removed exports**: All changes are visibility narrowing (`pub` to `pub(crate)`) and method extraction -- no functionality was removed.
- **CHANGELOG documents the initial release comprehensively**: All features, security measures, and API surface are documented.

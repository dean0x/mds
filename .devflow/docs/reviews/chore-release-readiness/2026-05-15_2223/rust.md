# Rust Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**LIFO invariant check reorders error precedence in resolver** - `src/resolver.rs:212-222`
**Confidence**: 82%
- Problem: In `ModuleCache::resolve`, the LIFO invariant check was changed from `assert_eq!` (which panics) to a soft error return. However, the new code calls `resolved?` (line 216) BEFORE checking the LIFO invariant (line 218). If `process_module` succeeds (Ok) but the pop was mismatched, the LIFO error is correctly returned. But if `process_module` fails AND the pop is mismatched, the `resolved?` on line 216 propagates the processing error and the LIFO corruption goes undetected/unreported. The old `assert_eq!` always fired regardless of the processing result. This is an intentional design choice (the comment says "prefer the user-facing module error") and the PR description mentions it, but it means a compiler bug (LIFO corruption) could be silently masked by any processing error. In practice, LIFO corruption here is extremely unlikely (the push/pop are structurally paired in a single function), so this is a defensible trade-off.
- Fix: Consider logging the LIFO violation even when the processing error takes precedence, or use the same `prefer_first_error` pattern used in the evaluator. The evaluator's approach (line 220 of evaluator.rs) preserves both errors properly. Example:
  ```rust
  let popped = self.resolving.pop();
  let lifo_ok = popped.as_ref() == Some(&canonical);
  let resolved = resolved?;
  if !lifo_ok {
      return Err(MdsError::syntax("internal error: resolving stack LIFO invariant violated..."));
  }
  ```
  This is what you already have; the current code is correct. The only improvement would be to also check the LIFO when `resolved` is Err (currently skipped). But given the structural guarantee of paired push/pop, this is low risk.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`run_build` does not use `resolve_input` helper** - `src/main.rs:458-467`
**Confidence**: 85%
- Problem: `run_check` correctly uses the new `resolve_input(input)?` helper (line 527), but `run_build` (lines 458-467) duplicates the auto-detection logic inline with additional printing logic. This is intentional (build prints a "Building..." banner while check does not), but the asymmetry means a future change to auto-detection would need to update two code paths. The `resolve_input` function was extracted specifically for reuse.
- Fix: If the banner is the only difference, consider extending `resolve_input` to accept an optional callback or log flag, or extract just the banner to keep the auto-detect path unified. This is a minor consistency issue, not a correctness bug.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`#[must_use]` on `MdsError` enum** - `src/error.rs:20`
**Confidence**: 83%
- Problem: The `MdsError` enum has `#[must_use]` which is good practice. However, no `#[must_use]` attributes are present on Result-returning functions like `evaluate`, `validate`, `resolve`, etc. in the library modules. Per the Rust Patterns skill checklist, `#[must_use]` should be on Result-returning functions. The public API functions in `lib.rs` do have `#[must_use]`, which is correct. The internal functions (`evaluate`, `validate`, `invoke_function`, etc.) rely on `#[must_use]` on the `Result` type itself, which is usually sufficient.

### LOW

**`MAX_TRAVERSAL_DEPTH` defined twice with the same value** - `src/main.rs:29`, `src/resolver.rs:47`
**Confidence**: 80%
- Problem: Both `src/main.rs` and `src/resolver.rs` define `const MAX_TRAVERSAL_DEPTH: usize = 256` independently. The feature knowledge documents this as intentional ("they are separate named constants in their respective modules"), and each is private to its module. However, if someone changes one without the other, the behavior diverges silently. Not a bug, but a minor cohesion concern.
- Fix: Could extract to a shared location (e.g., a `pub(crate) const` in `lib.rs` or a shared constants module), but this is a style preference.

## Suggestions (Lower Confidence)

- **`output_size_limit_rejects_oversized_output` allocates 50MB+ in tests** - `src/evaluator.rs:622` (Confidence: 70%) -- The test allocates a string of `MAX_OUTPUT_SIZE + 1` bytes (50MB+). This works but is heavy for a unit test. Consider testing with a smaller mock limit or testing the check logic directly.

- **Validator `@for`/`@define` pop errors silently discarded** - `src/validator.rs:62,74` (Confidence: 65%) -- Both `@for` and `@define` validation paths use `let _ = scope.pop()` with a comment "Cannot fail -- we just pushed". This is correct given the Scope invariant (push always succeeds, pop only fails on the global frame). The comment is accurate. However, if `push()` were ever changed to fail or the invariant were broken, the `let _` would silently swallow the error. This is documented in the feature knowledge as an intentional exception.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 1 |

**Rust Score**: 8/10
**Recommendation**: APPROVED

## Rationale

This is a well-executed release-hardening PR. The Rust-specific changes demonstrate strong adherence to Rust idioms and the project's documented patterns:

**Strengths:**
- `#[non_exhaustive]` on `MdsError` and `Value` enums is textbook semver-safe public API design
- `pub(crate)` on all error constructors and value converters correctly restricts the API surface
- Replacing `scope.clone()` with `push()`/`pop()` in the validator eliminates unnecessary heap allocations
- `Arc<ResolvedModule>` caching in the resolver prevents redundant work on cache hits
- TOCTOU fix in `load_config` (read-then-check-size vs metadata-then-read) is a real security improvement
- `assert_eq!` replaced with soft error return in the resolver's LIFO check -- appropriate for a library (panics in libraries are hostile to callers)
- The refactoring of `canonicalize_and_check` into `check_symlink`/`check_import_depth`/`check_path_traversal` improves single-responsibility and testability
- The `run_build`/`run_check`/`run_init` extraction from the monolithic `run()` follows single-responsibility principle
- `names.first().map(...)` replacing `names[0]` eliminates a potential panic path
- No `.unwrap()` in non-test code; error propagation via `?` throughout
- Clippy clean with `-D warnings`; 292 tests passing
- Resource limit tests (call depth, output size, nesting depth, warning cap) improve coverage of safety-critical bounds

**No blocking issues found.** The MEDIUM findings are about error-reporting completeness in an extremely unlikely internal invariant violation path, and a minor code-reuse inconsistency. Neither affects correctness or safety.

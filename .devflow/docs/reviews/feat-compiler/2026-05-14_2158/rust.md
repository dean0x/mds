# Rust Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**Commits reviewed**: 4 (e7a7c6b, 4add2a7, 0300cfd, 7c49fc5)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`assert!` in `invoke_function` will panic in release builds on invariant violation** - `src/evaluator.rs:196-198`
**Confidence**: 82%
- Problem: The change promotes a `debug_assert!` to a full `assert!` for the call_stack LIFO invariant. The comment justifies this ("enforce in release mode -- cost is negligible at MAX_CALL_DEPTH = 128"). However, panicking in a library function violates Rust API guidelines -- library code should return `Result`, not panic. If a bug causes a LIFO violation, the user gets an opaque panic rather than a structured `MdsError` with a diagnostic message. The `evaluate_for` double-fault pattern (lines 299-307) correctly returns `Err`; the call_stack assert is the exception.
- Fix: Replace the `assert!` with an error return to keep the same invariant enforcement without panicking:
  ```rust
  let popped = ctx.call_stack.pop();
  if popped.as_deref() != Some(call_key) {
      return Err(MdsError::syntax(
          &format!("internal error: call_stack LIFO violated: expected '{call_key}', got {popped:?}")
      ));
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`canonicalize_and_check` performs security checks (import depth, path traversal) on cache hits** - `src/resolver.rs:73-138` (called at line 171)
**Confidence**: 80%
- Problem: In the `resolve()` method, `canonicalize_and_check` is called before the cache check (line 171 vs line 174). This means every cache hit still pays the cost of `canonicalize` syscalls and re-runs the import-depth guard (`self.resolving.len() >= MAX_IMPORT_DEPTH`). The decomposition comment states "cache hits pay only the cost of two `canonicalize` syscalls and no I/O" -- which is the design intent -- but the import-depth check at line 121 uses `self.resolving.len()` which is unrelated to the cached file itself. For a cached file, the depth check is a no-op that already passed during the original resolve. This is not a bug (the check is correct), but the depth check being inside `canonicalize_and_check` muddies the single-responsibility boundary. Consider: if `MAX_IMPORT_DEPTH` is lowered after a module is cached, a re-import at the new (shallower) depth would incorrectly pass because the cache hit returns before the depth is checked for the recursive sub-imports. This is an unlikely scenario but a logical inconsistency in the decomposition.
- Fix: Move the import-depth guard out of `canonicalize_and_check` and into the `resolve()` method between the cache check and the cycle detection, so the check only applies to cache misses (which are the only paths that recurse).

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`debug_assert_eq!` on resolving LIFO pop** - `src/resolver.rs:203-204` (Confidence: 65%) -- The `debug_assert_eq!` for the resolving pop is appropriate since `IndexSet::pop` is guaranteed LIFO by data structure semantics. However, unlike the evaluator's call_stack, this assertion disappears in release builds. If consistent with the evaluator's approach (full `assert!` for safety-critical invariants), this could also be a full `assert!` or an error return for consistency. The cost is one PathBuf comparison per resolve.

- **Doc comment placement for `MAX_CONFIG_SIZE`** - `src/main.rs:33-34` (Confidence: 70%) -- The `MAX_CONFIG_SIZE` doc comment runs into the end of the previous `load_config` doc comment rather than sitting above the constant. The rendered rustdoc for `load_config` will include the line "Maximum allowed size for `mds.json` (1 MB) to prevent runaway memory use." as part of the function docs. This is a cosmetic issue in a binary crate (no public rustdoc), but the intent was clearly to document the constant.

- **`Arc::new(f.clone())` inside loop restoring captured functions** - `src/evaluator.rs:179` (Confidence: 60%) -- Each function invocation clones every captured `FunctionDef` and wraps it in a new `Arc`. For functions with many captured siblings, this is O(n) allocations per call. If functions were stored as `Arc<FunctionDef>` in `CapturedScope` (breaking the cycle-avoidance design), this would be O(1). This is an architectural trade-off documented in comments -- noting it for awareness if profiling shows hot paths in deeply nested calls.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The code demonstrates strong Rust practices overall:
- Proper use of `Result` for error handling throughout (with the one `assert!` exception noted)
- Good use of `Arc` for shared ownership with documented cycle-avoidance strategy
- `IndexSet::pop()` replacing `shift_remove()` is a correct O(1) optimization
- The resolver decomposition into `canonicalize_and_check` / `read_validated_file` cleanly separates syscall costs from I/O costs
- `CollectedDefs` struct replacing a 3-tuple improves readability
- Double-fault error preservation pattern is well-reasoned and consistently applied
- Clippy passes with zero warnings
- All 280 tests pass

Conditions for approval: Address the HIGH-severity `assert!` in `invoke_function` -- either convert to an error return or add a comment explicitly justifying the panic in library code (e.g., "this invariant is structurally guaranteed by the push/pop bracket above; violation indicates memory corruption").

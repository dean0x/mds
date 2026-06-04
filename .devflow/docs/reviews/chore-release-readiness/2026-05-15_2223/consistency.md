# Consistency Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### HIGH

**`run_build` does not use `resolve_input()` helper while `run_check` does** - `src/main.rs:458-466`
**Confidence**: 92%
- Problem: The PR introduces a `resolve_input()` helper (line 440-444) that encapsulates the `match input { Some(p) => Ok(p), None => auto_detect_mds_file() }` pattern. `run_check` calls `resolve_input(input)?` at line 527, but `run_build` inlines the same logic manually at lines 458-466 with an added `eprintln!` banner. The knowledge doc explicitly states "New CLI commands should have a dedicated `run_*` function" and the anti-pattern section warns against duplicating patterns. While `run_build` has extra banner-printing logic on the `None` branch, the duplication of the `Some`/`None` dispatch and `auto_detect_mds_file()` call is inconsistent with the stated purpose of `resolve_input`.
- Fix: Either (a) refactor `resolve_input` to accept an optional callback for the auto-detect case, or (b) use `resolve_input` in `run_build` and print the banner separately:
  ```rust
  let input = resolve_input(input)?;
  // If input was auto-detected (user didn't pass a path), print the banner.
  // This requires a different approach since resolve_input doesn't signal detection.
  ```
  The cleanest fix: have `resolve_input` return a `(PathBuf, bool)` where the bool indicates auto-detection, or accept a `quiet` param and handle the banner internally.

### MEDIUM

**LIFO invariant enforcement is inconsistent between resolver and evaluator** - `src/resolver.rs:218-222` vs `src/evaluator.rs:208-215`
**Confidence**: 82%
- Problem: Both the resolver (`resolving` stack) and the evaluator (`call_stack`) enforce a LIFO pop invariant, but they use different error construction patterns. The resolver uses `MdsError::syntax("internal error: resolving stack LIFO invariant violated ...")` (bare string), while the evaluator uses `MdsError::syntax(format!("internal error: call_stack LIFO violated: expected '{call_key}', got {popped:?}"))` (includes diagnostic details of what was expected vs. what was popped). The evaluator's version is strictly more debuggable -- it tells you what went wrong. The resolver's version provides no actionable debug context.
- Fix: Align the resolver's LIFO error to include diagnostic details:
  ```rust
  if popped.as_ref() != Some(&canonical) {
      return Err(MdsError::syntax(format!(
          "internal error: resolving stack LIFO violated: expected '{}', got {:?} — this is a compiler bug, please report it",
          canonical.display(),
          popped,
      )));
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Constant visibility is inconsistent across modules** - `src/evaluator.rs:9-22`, `src/resolver.rs:44-50`, `src/main.rs:26-29`, `src/value.rs:6`
**Confidence**: 85%
- Problem: The PR establishes `pub(crate)` as the intended visibility for constants that are referenced across modules (e.g., `MAX_NESTING_DEPTH` in parser.rs was elevated to `pub(crate)` so the validator can import it). However, the evaluator's resource limit constants (`MAX_CALL_DEPTH`, `MAX_OUTPUT_SIZE`) are private `const` yet are tested in the evaluator's own `mod tests` block, which is fine -- but they are also semantically important limits that the integration tests reference in comments (line 3130: "MAX_CALL_DEPTH (128)"). Meanwhile, `MAX_FILE_SIZE` in resolver.rs is `pub(crate)` and is re-exported as `pub const` in lib.rs. The knowledge doc says `MAX_NESTING_DEPTH` was elevated to `pub(crate)` specifically for cross-module use. The pattern is: private when used only in the defining module, `pub(crate)` when used cross-module. This is consistent. No actual issue with correctness, but the inconsistency between `MAX_FILE_SIZE` being `pub(crate)` (and re-exported) while all other limits are private is worth noting as a design decision to document rather than a bug.
- Fix: This is intentional per the knowledge doc -- `MAX_FILE_SIZE` is re-exported because the CLI stdin reader needs it. No code change needed, but a doc comment explaining why `MAX_FILE_SIZE` is the only `pub(crate)` limit constant in resolver.rs would help future contributors understand the pattern.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Duplicate `MAX_TRAVERSAL_DEPTH` constants in separate modules** - `src/main.rs:29`, `src/resolver.rs:47`
**Confidence**: 80%
- Problem: Both `src/main.rs` and `src/resolver.rs` define `const MAX_TRAVERSAL_DEPTH: usize = 256` independently. If one is changed without the other, the traversal bounds diverge silently. The feature knowledge explicitly documents this as intentional ("they are separate named constants in their respective modules"), but it remains a maintenance risk for a value that has the same semantic meaning.
- Fix: Consider defining a single `pub(crate)` constant in a shared location (e.g., `resolver.rs` or a new `constants.rs`) and importing it in `main.rs`. This is a pre-existing design choice documented in the feature knowledge, so not blocking.

## Suggestions (Lower Confidence)

- **`check_symlink` is an associated function while `check_import_depth` and `check_path_traversal` are instance methods** - `src/resolver.rs:74,99,109` (Confidence: 65%) -- The three extracted security check helpers have inconsistent receiver types. `check_symlink` takes only `path: &Path` (no `&self`), while the other two take `&self`. This is technically correct (symlink detection doesn't need `ModuleCache` state), but the asymmetry may surprise readers expecting a uniform pattern among the three sibling methods.

- **`run_check` does not print "Checking ..." banner on auto-detect, unlike `run_build` which prints "Building ..."** - `src/main.rs:527` vs `src/main.rs:462-464` (Confidence: 70%) -- When auto-detecting input, `run_build` prints `"Building {path}"` to stderr, but `run_check` prints nothing on detection -- only `"OK: {path}"` after success. A user running `mds check` with no argument gets no immediate feedback about which file was selected. This is a UX consistency gap between the two commands.

- **`resolve_import` return type no longer has its own `Ok(())` -- it relies on match arm returns** - `src/resolver.rs:469-487` (Confidence: 62%) -- The refactored `resolve_import` now returns the result of each helper directly from the match arm. The old version had an explicit `Ok(())` at the bottom. This is fine and even idiomatic Rust (match-as-expression), but the three helper functions return `Result<(), MdsError>` while the parent also returns `Result<(), MdsError>` -- the composition is clean. No action needed.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR achieves strong consistency in its primary goals: `pub(crate)` visibility is applied uniformly across all error constructors and value converters, `#[non_exhaustive]` is correctly placed on the two public enums (`MdsError`, `Value`), magic numbers are replaced with named constants, error constructor patterns follow the established `_at` convention, and the CLI is cleanly decomposed into `run_build`/`run_check`/`run_init`. The extracted resolver helpers follow consistent parameter ordering. The one blocking HIGH issue is the `run_build` / `resolve_input` inconsistency -- `resolve_input` was introduced in this PR but is only used by one of its two intended consumers.

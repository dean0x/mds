# Consistency Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Inconsistent entry-key exclusion strategy across `_with_deps` functions** - `lib.rs:530-534` vs `lib.rs:610`
**Confidence**: 90%
- Problem: The three `compile_*_with_deps` functions use three different approaches to exclude the entry module from the dependencies list:
  - `compile_with_deps` (line 530-534): Uses `split_last()` on the Vec from `cache.dependencies()`, assuming the entry is always last (post-order DFS invariant).
  - `compile_str_with_deps` (line 571-573): No filtering at all -- relies on the fact that `resolve_source` does not insert the inline source into the cache.
  - `compile_virtual_with_deps` (line 610): Uses `.into_iter().filter(|k| k != entry).collect()` -- a string-equality filter on the entry key.
- Impact: Three different exclusion mechanisms for the same semantic operation (exclude entry from deps). The `split_last()` approach in `compile_with_deps` relies on an ordering invariant, the `filter` approach in `compile_virtual_with_deps` is a linear scan by value, and `compile_str_with_deps` implicitly relies on `resolve_source` not caching. If any of these invariants drift, the behavior diverges silently. More importantly, someone maintaining one function may not realize the pattern differs in the other two.
- Fix: Document the intentional divergence with a shared comment block, or extract a common helper. The differences are intentional (each path has different cache semantics), but a comment on each noting "See also: compile_with_deps, compile_virtual_with_deps for the other exclusion strategies" would prevent future drift. Alternatively, a small shared function could encapsulate the "deps minus entry" logic:
  ```rust
  fn deps_excluding_entry(cache: &ModuleCache, entry_key: Option<&str>) -> Vec<String> {
      let deps = cache.dependencies();
      match entry_key {
          Some(key) => deps.into_iter().filter(|k| k != key).collect(),
          None => deps,
      }
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`#[must_use]` message style inconsistency between old and new functions** - `lib.rs:102,516`
**Confidence**: 92%
- Problem: The existing `compile` function uses `#[must_use = "the compiled Markdown output should be used"]` while the new `compile_with_deps` uses `#[must_use = "the compiled output, warnings, and dependencies should be used"]`. The new message is content-focused (listing struct fields), whereas existing messages are type-focused ("Markdown output"). The commit message (4d2f097) explicitly states the change was intentional, moving from type-name-referencing to content-focused -- but only the three new `_with_deps` functions follow the new style. The 16 existing functions still use the old style.
- Impact: Two coexisting `#[must_use]` message styles in the same module. Neither is wrong, but the lack of consistency across the public API surface creates a minor cognitive inconsistency.
- Fix: This is a low-priority cleanup. Either leave both styles as-is (the new style for new functions is fine as a forward-going convention), or update the existing functions to match in a separate PR. No blocking action needed.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`CompileOutput` placement in `lib.rs` vs dedicated module** - `lib.rs:67` (Confidence: 65%) -- `SerializedError` and `SerializedSpan` live in `error.rs` (a dedicated module), but `CompileOutput` is defined directly in `lib.rs`. As more output types are added, a dedicated `output.rs` module may be more consistent with the crate's module-per-concern pattern. However, `CompileOutput` is small and closely tied to the public API surface, so `lib.rs` is a defensible location.

- **Derive trait set consistency between `CompileOutput` and `MdsError`** - `lib.rs:67` vs `error.rs:73` (Confidence: 62%) -- `CompileOutput` derives `(Debug, Clone, PartialEq, serde::Serialize)` while `MdsError` derives `(Error, Debug, Diagnostic, Clone)` without `PartialEq` or `Serialize`. `SerializedError` matches `CompileOutput`'s derive set exactly. The divergence is justified (MdsError contains `Arc` which makes PartialEq semantically complex), but worth noting that the new types are internally consistent with each other.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates strong consistency overall: naming conventions are uniform (snake_case functions, PascalCase types), error handling consistently uses `Result<T, MdsError>`, doc comment style is thorough and consistent across all new public functions, `#[derive]` sets on new types are internally consistent, and the new `_with_deps` function naming follows the established `compile_*` / `compile_*_collecting_warnings` pattern. The two MEDIUM issues are both about internal consistency within the new code itself -- the entry-key exclusion divergence is the most substantive and should be addressed with either a shared helper or cross-referencing comments before or shortly after merge.

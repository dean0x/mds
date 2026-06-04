# Consistency Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18
**Commits**: 03630e8, 772882c, 72a0037, 69f23ea

## Issues in Your Changes (BLOCKING)

### HIGH

**Naming convention break: `_with_deps` vs established `_collecting_warnings` suffix pattern** - `crates/mds-core/src/lib.rs:506,538,564`
**Confidence**: 85%
- Problem: The existing API surface establishes a clear naming convention for function variants that return extra data beyond the base return type. The pattern is `{base_fn}_collecting_warnings` (e.g., `compile_collecting_warnings`, `compile_str_collecting_warnings`, `compile_virtual_collecting_warnings`). The new functions use `_with_deps` (e.g., `compile_with_deps`, `compile_str_with_deps`, `compile_virtual_with_deps`). While `_with_deps` is shorter and arguably more descriptive of the additional data returned, it introduces a second naming convention for what is conceptually the same pattern: "like the base function, but returns additional metadata." The existing convention describes the mechanism (`collecting_warnings`), while the new convention describes the added content (`with_deps`). This creates a bifurcated naming idiom in the public API.
- Fix: Consider aligning to either convention consistently. Options:
  1. Rename to `compile_collecting_deps` / `compile_str_collecting_deps` / `compile_virtual_collecting_deps` to match the existing `_collecting_X` pattern.
  2. Accept `_with_deps` as a new pattern layer, justified by the fact that these functions return a `CompileOutput` struct rather than a tuple, making them semantically different from the `_collecting_warnings` family. If choosing option 2, document this as an intentional divergence in the module-level docs.

### MEDIUM

**Missing `# Examples` doc sections on all three `_with_deps` functions** - `crates/mds-core/src/lib.rs:498-504,529-536,555-562`
**Confidence**: 90%
- Problem: Every existing public function in `lib.rs` includes a `# Examples` section in its doc comment (15 total across `compile`, `compile_str`, `compile_str_with`, `check`, `check_str`, `compile_collecting_warnings`, `compile_str_collecting_warnings`, `check_collecting_warnings`, `check_str_collecting_warnings`, `compile_virtual`, `compile_virtual_collecting_warnings`, `check_virtual`, `check_virtual_collecting_warnings`, `compile_file`, `load_vars_file`). All three new `_with_deps` functions lack `# Examples` sections. This is a documentation style inconsistency.
- Fix: Add `# Examples` sections following the established pattern. For example:
  ```rust
  /// # Examples
  ///
  /// ```rust
  /// use std::collections::HashMap;
  ///
  /// let mut modules = HashMap::new();
  /// modules.insert("main.mds".to_string(), "---\nname: World\n---\nHello {name}!\n".to_string());
  ///
  /// let result = mds::compile_virtual_with_deps(modules, "main.mds", None)?;
  /// assert_eq!(result.output, "---\nname: World\n---\nHello World!\n");
  /// assert!(result.dependencies.is_empty());
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  ```

**`#[must_use]` message inconsistency: `"the CompileOutput should be used"` vs existing pattern** - `crates/mds-core/src/lib.rs:505,537,563`
**Confidence**: 82%
- Problem: The existing `#[must_use]` messages follow a pattern of describing what the return value contains: `"the compiled Markdown output should be used"`, `"the compiled Markdown output and warnings should be used"`, `"warnings should be used"`, `"errors should be handled"`, `"the loaded variables should be used"`. The new functions use `"the CompileOutput should be used"` which references the type name rather than describing the content. This is the only place in the file where a type name appears in a `#[must_use]` message.
- Fix: Change to content-descriptive messages like: `"the compiled output, warnings, and dependencies should be used"` to match the established style.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`compile_with_deps` calls `path.canonicalize()` directly instead of using `FileSystem::canonicalize()`** - `crates/mds-core/src/lib.rs:521-523`
**Confidence**: 85%
- Problem: This PR specifically added `FileSystem::canonicalize()` and updated `resolve_source` to use it (commit 03630e8, fixing #21). However, `compile_with_deps` still calls `std::path::Path::canonicalize()` directly to compute the entry key for filtering. This is internally inconsistent within the PR itself -- the stated goal is to route canonicalization through the `FileSystem` trait, but one callsite was missed. While this particular callsite only affects dependency filtering (not resolution), the inconsistency could surprise contributors and defeats the purpose of the abstraction for any custom `FileSystem` implementation.
- Fix: Route the canonicalization through the `ModuleCache`'s filesystem. This may require exposing a canonicalize method on `ModuleCache`, or accepting that the entry-key computation in `compile_with_deps` operates at the public API level where only `NativeFs` is used. If the latter, add a comment explaining why direct `canonicalize()` is acceptable here.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Test naming convention divergence in error.rs** - `crates/mds-core/src/error.rs:579-809` (Confidence: 65%) -- The existing test naming pattern in error.rs uses `{variant}_{assertion}` format (e.g., `syntax_display_contains_message`, `syntax_at_populates_span_and_src`). The new serialization tests use `serialize_{variant}_{detail}` (e.g., `serialize_syntax_with_span`, `serialize_arity_code`). Similarly, `line_col_*` is a new prefix. Both patterns are reasonable and internally consistent within their test groups, so this is a stylistic observation rather than a clear violation.

- **`MdsError::serialize()` method is `pub` while all constructors are `pub(crate)`** - `crates/mds-core/src/error.rs:528` (Confidence: 70%) -- The established pattern for `MdsError` is that all constructor methods (e.g., `syntax()`, `io()`, `arity_at()`) are `pub(crate)`. The new `serialize()` method is `pub`, which is intentional since it is part of the public API. However, this is the first and only `pub` method on `MdsError` -- all others are `pub(crate)`. The visibility difference is justified by the different purpose (construction vs consumption), but worth noting.

- **`CompileOutput` doc comment says "excludes the entry module itself" but `compile_str_with_deps` does not exclude anything** - `crates/mds-core/src/lib.rs:65-66` (Confidence: 75%) -- The struct-level doc says "excludes the entry module" but `compile_str_with_deps` explicitly notes it does NOT exclude anything since there is no entry key. The struct doc could be more precise: "typically excludes the entry module" or "see individual function docs for filtering behavior."

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The changes are well-structured and internally consistent within each new feature (serialization, dependency tracking, canonicalize trait method). The main consistency concern is the `_with_deps` naming pattern diverging from the established `_collecting_warnings` convention, and the missing `# Examples` doc sections that every other public function provides. The direct `canonicalize()` call in `compile_with_deps` is a within-PR inconsistency that undermines the #21 fix. None of these are blocking on correctness, but they create API surface inconsistencies that will compound as the project grows.

# Complexity Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18
**PR**: #22 — SerializedError/SerializedSpan, CompileOutput, FileSystem::canonicalize

## Issues in Your Changes (BLOCKING)

### MEDIUM

**API surface combinatorial growth: 19 public functions in lib.rs** - `crates/mds-core/src/lib.rs`
**Confidence**: 82%
- Problem: lib.rs now exposes 19 public `fn` signatures. This PR adds 3 more (`compile_with_deps`, `compile_str_with_deps`, `compile_virtual_with_deps`), continuing an existing pattern of `compile` / `compile_collecting_warnings` / `compile_with_deps` triads for every entry point variant. The combinatorial expansion (base x str x virtual) x (simple, warnings, deps) is approaching a point where new contributors must study many near-identical functions to understand the API. Each new cross-cutting concern (e.g., progress callbacks) would add another 3-6 functions.
- Fix: Not blocking for this PR since the pattern is pre-existing and the new functions are consistent with it. Consider a builder pattern or options struct in a future PR to collapse the matrix:
  ```rust
  mds::compile(path)
      .with_vars(vars)
      .collecting_warnings()
      .with_deps()
      .run()?;  // -> CompileOutput
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### HIGH

**error.rs is 983 lines (file length > 500)** - `crates/mds-core/src/error.rs`
**Confidence**: 85%
- Problem: The file has grown to 983 lines, nearly twice the warning threshold (500). Roughly 400 lines are test code and 290 lines are repetitive `_at` constructor pairs (one without span, one with span). The new `serialize()` method and its 200+ lines of tests added to an already large file.
- Fix: Not blocking since the structure is coherent (all error types in one module). Future cleanup could extract `mod serialization` and/or `mod tests::serialization` into submodules. The repetitive constructor pairs could also be macro-generated to cut ~200 lines.

### MEDIUM

**lib.rs is 807 lines (file length > 500)** - `crates/mds-core/src/lib.rs`
**Confidence**: 80%
- Problem: The file is 807 lines. The new `compile_*_with_deps` family adds ~70 lines of near-identical boilerplate on top of an already large public API surface. All 19 public functions live in a single file.
- Fix: Informational. Group by concern (compile, check, virtual, deps) into submodules or a `compile/mod.rs` in a future refactor.

## Suggestions (Lower Confidence)

- **`compile_with_deps` uses `path.canonicalize()` directly instead of `self.fs.canonicalize()`** - `crates/mds-core/src/lib.rs:521-524` (Confidence: 72%) — The PR's stated goal is to route canonicalization through the `FileSystem` trait (fixing #21). `resolve_source` was updated to use `self.fs.canonicalize()`, but `compile_with_deps` still calls `std::path::Path::canonicalize()` directly (line 522). Since `compile_with_deps` always uses `NativeFs` via `ModuleCache::new()`, this is functionally correct today, but it breaks the abstraction the PR is establishing. If `compile_with_deps` is ever generalized to accept a custom `FileSystem`, this will be a latent bug.

- **Repetitive body pattern across compile_*_with_deps functions** - `crates/mds-core/src/lib.rs:506-576` (Confidence: 68%) — The three `compile_*_with_deps` functions share a nearly identical body pattern (setup cache, resolve, build_output, filter deps, return CompileOutput). This mirrors the same duplication in the existing `compile_collecting_warnings` family. A shared helper taking a closure for resolution could eliminate the repetition.

- **`MdsError::serialize()` match arm enumerates 11 variants** - `crates/mds-core/src/error.rs:537-567` (Confidence: 62%) — The `match self` in `serialize()` lists 11 span-bearing variants by name with `|` alternation. Adding a new `MdsError` variant with `(span, src)` requires remembering to add it here. The `#[non_exhaustive]` attribute on the enum means the wildcard arm catches new variants, which would silently produce `span: None` for a span-bearing variant. A helper method like `fn span_and_src(&self) -> Option<(&SourceSpan, &NamedSource)>` on `MdsError` would centralize the extraction.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 1 | 1 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new code is well-structured with clean function boundaries. Individual functions are short (serialize: 42 lines, compile_with_deps: 21 lines, compile_str_with_deps: 15 lines, compile_virtual_with_deps: 13 lines) and nesting depth stays at 4 levels maximum (the serialize match arm's closure). The `compute_line_column` helper is 15 lines with a single loop.

The primary complexity concern is at the API surface level, not the implementation level: the combinatorial explosion of public functions (now 19) makes the library harder for new contributors to navigate. This is a pre-existing pattern that the PR follows consistently, not something it introduces. The new code is clean, well-documented, and individually simple.

Condition: consider the `path.canonicalize()` abstraction leak in `compile_with_deps` (see Suggestions) before merging, as it partially contradicts the PR's #21 fix intent.

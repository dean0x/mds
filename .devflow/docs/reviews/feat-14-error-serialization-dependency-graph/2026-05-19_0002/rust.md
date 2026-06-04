# Rust Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Inconsistent entry-key exclusion strategies across `compile_with_deps` variants** - `lib.rs:531` vs `lib.rs:610`
**Confidence**: 82%
- Problem: `compile_with_deps` uses `split_last()` (positional — assumes entry is the last element in the IndexMap) to exclude the entry key from dependencies. `compile_virtual_with_deps` uses `.filter(|k| k != entry)` (value-based comparison). The positional approach in `compile_with_deps` is correct today because `resolve_by_key` inserts modules in post-order DFS, but it relies on an implicit ordering invariant that is not enforced by contract. If the caching logic in `resolve_by_key` were ever refactored (e.g., early cache insertion for error recovery), the `split_last()` assumption would silently break and exclude the wrong module from the dependency list.
- Fix: Use the value-based filtering consistently. `compile_with_deps` can capture the entry key after normalization and filter by value, matching the `compile_virtual_with_deps` pattern:
  ```rust
  pub fn compile_with_deps(
      path: impl AsRef<Path>,
      runtime_vars: Option<HashMap<String, Value>>,
  ) -> Result<CompileOutput, MdsError> {
      let path = path.as_ref();
      let vars = runtime_vars.unwrap_or_default();
      let mut cache = ModuleCache::new();
      let mut warnings = vec![];
      let resolved = cache.resolve_path(path, &vars, &mut warnings)?;
      let output = build_output(&resolved);
      let deps = cache.dependencies();
      // Use the last key (entry) for value-based filtering, consistent
      // with compile_virtual_with_deps.
      let entry_key = deps.last().cloned();
      let dependencies = deps.into_iter()
          .filter(|k| entry_key.as_ref() != Some(k))
          .collect();
      Ok(CompileOutput { output, warnings, dependencies })
  }
  ```
  Or expose the entry key from `resolve_path` directly.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **CRLF line endings produce inflated column counts in `compute_line_column`** - `error.rs:40` (Confidence: 65%) -- The `\r` byte in CRLF sequences counts as a regular column byte, so `\r\n` at the end of a line produces column values 1 higher than editors display on Windows. The documentation explicitly says "byte offsets from line start," making this technically correct, but consumers on Windows may find it surprising. Consider documenting this CRLF behavior explicitly or handling `\r` before `\n` as a line-ending component.

- **`unwrap_or_default()` on `Diagnostic::code()` silently produces empty string for variants missing a code attribute** - `error.rs:530` (Confidence: 62%) -- If a new `MdsError` variant were added without a `#[diagnostic(code(...))]` attribute, `serialize()` would produce an empty `code: ""` field instead of signaling the omission. Since all 16 current variants have codes, and the enum is `#[non_exhaustive]`, this would only bite future contributors. A debug assertion or a `"mds::unknown"` sentinel could make the gap more visible.

- **`dependencies()` returns a `Vec<String>` (cloned) rather than exposing an iterator or slice** - `resolver.rs:112` (Confidence: 60%) -- `cache.dependencies()` clones all keys into a new `Vec<String>`, then both `compile_with_deps` and `compile_virtual_with_deps` immediately consume or filter it. Returning `impl Iterator<Item = &str>` or `&[String]` would avoid the allocation, though for typical dependency counts (dozens, not thousands) the practical impact is negligible.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The code is well-structured, idiomatic Rust. Key positives:

- `thiserror` + `miette` for library-grade error handling with diagnostic codes -- textbook Rust error design.
- `#[non_exhaustive]` on `MdsError` protects downstream consumers from future variant additions.
- `serialize()` match is exhaustive across all 16 variants with explicit no-span arms -- drift-proof.
- `compute_line_column` is clean, byte-indexed, well-documented, and correctly handles the `offset == source.len()` boundary.
- `FileSystem::canonicalize()` default implementation (identity) is correct for virtual/WASM backends; `NativeFs` override properly routes through `check_symlink()` for symlink rejection.
- `HashMap` -> `IndexMap` migration in `ModuleCache` cleanly preserves insertion-order semantics for dependency extraction.
- `#[must_use]` annotations on all `compile_*_with_deps` functions with descriptive messages.
- All 481 tests pass. Clippy clean with zero warnings.

The single MEDIUM finding (inconsistent entry-key exclusion) is not a correctness bug today but represents a fragility that would be easy to eliminate for consistency. The APPROVED_WITH_CONDITIONS recommendation reflects that this is a non-blocking improvement worth addressing.

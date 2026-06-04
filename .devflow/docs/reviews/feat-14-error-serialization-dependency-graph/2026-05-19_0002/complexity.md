# Complexity Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Repetitive _at / no-span constructor pairs (11 pairs, 290+ lines)** -- Confidence: 82%
- `error.rs:230-517` (27 constructors total: 11 `_at` variants, 16 plain)
- Problem: Each MdsError variant with span support has two near-identical constructors -- a plain version setting `span: None, src: None` and an `_at` version calling `at(file, source, offset, len)` then constructing the same variant. This is a repeating 12-14 line pattern duplicated 11 times. While each constructor is individually simple (cyclomatic complexity 1), the cumulative volume pushes error.rs to 1077 lines and increases maintenance surface -- adding a new variant requires adding two constructors following the exact same template.
- Fix: This is a known Rust idiom for enum constructors and the `at()` helper already extracts the shared span logic. A macro_rules! could reduce boilerplate but would trade readability for brevity. This is a judgment call -- the current approach is explicit and searchable. Consider a macro only if more variants are expected.

**compile_*_with_deps functions duplicate compile_*_collecting_warnings structure** -- Confidence: 80%
- `lib.rs:517-536`, `lib.rs:560-575`, `lib.rs:600-612`
- Problem: The three `compile_*_with_deps` functions share the same structure as the corresponding `compile_*_collecting_warnings` functions (lines 268-296), differing only in the final 3-5 lines where dependencies are extracted. This brings lib.rs to 843 lines and 19+ public functions. Each new "variant" of the compile API requires a full copy of the cache setup boilerplate.
- Fix: Extract the shared cache-setup + resolve pattern into an internal helper that returns `(ResolvedModule, ModuleCache, Vec<String>)`, then have both the `_collecting_warnings` and `_with_deps` families call it. Example for the file-path variant:
  ```rust
  fn resolve_path_internal(
      path: &Path,
      runtime_vars: Option<HashMap<String, Value>>,
  ) -> Result<(Arc<ResolvedModule>, ModuleCache, Vec<String>), MdsError> {
      let vars = runtime_vars.unwrap_or_default();
      let mut cache = ModuleCache::new();
      let mut warnings = vec![];
      let resolved = cache.resolve_path(path, &vars, &mut warnings)?;
      Ok((resolved, cache, warnings))
  }
  ```
  This would cut 6 functions down to 3 helpers + 6 thin wrappers.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**error.rs exceeds 500-line file length threshold (1077 lines)** -- Confidence: 85%
- `error.rs:1-1077`
- Problem: The file now contains the MdsError enum definition (227 lines), 27 constructor methods (290 lines), the `serialize()` method (43 lines), and 500+ lines of tests. At 1077 lines it is above the "Critical" threshold (>500 lines per the complexity metrics). The test module alone is over 500 lines.
- Fix: Move `#[cfg(test)] mod tests` to a separate `tests/error_tests.rs` integration test file or use `#[path = "error_tests.rs"] mod tests;` to split the module while keeping `pub(crate)` access. This would bring the source file to ~570 lines, still slightly above threshold but much more manageable.

**lib.rs public API surface is growing wide (19+ public functions)** -- Confidence: 83%
- `lib.rs:1-843`
- Problem: lib.rs now exposes 19+ public functions across 843 lines, many following the pattern `compile[_variant][_collecting_warnings|_with_deps]`. The combinatorial growth (path/str/virtual x simple/warnings/deps) creates a wide API surface that is harder to navigate and maintain. Each new capability multiplies across all three input variants.
- Fix: This is a pre-existing design trajectory that the PR follows faithfully. No change needed in this PR, but consider a builder pattern in a future iteration: `mds::compile(path).with_deps().run()` would reduce the public function count while preserving the same functionality.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Entry-key exclusion uses two different strategies** - `lib.rs:531` vs `lib.rs:610` (Confidence: 70%) -- `compile_with_deps` uses `split_last()` on the Vec while `compile_virtual_with_deps` uses `.filter(|k| k != entry)`. Both are correct but the inconsistency may confuse future maintainers. A shared helper `exclude_entry_key(deps, entry)` would unify the approach.

- **serialize() match arm lists all 11 span-bearing variants** - `error.rs:537-567` (Confidence: 65%) -- The match block in `serialize()` enumerates every span-bearing variant with an or-pattern. Adding a new span-bearing variant requires updating this match arm. The `#[non_exhaustive]` attribute on MdsError means external callers cannot construct variants, but internal drift is possible. The wildcard fallback arm for no-span variants does provide safety, so this is more a maintenance observation than a bug risk.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The code is well-structured and each individual function has low cyclomatic complexity (1-3). The `serialize()` method is the most complex new function at ~43 lines with nesting depth 3, which is within acceptable limits. The `compute_line_column` function is a clean 15-line loop with clear termination. The `build_output` extraction is a good refactoring move that reduced duplication in the existing compile functions.

The main complexity concerns are cumulative rather than per-function: the growing file sizes (error.rs at 1077 lines, lib.rs at 843 lines) and the combinatorial API surface. These are not blocking for this PR but should be addressed before the next major feature addition. The new code faithfully follows the established patterns in the codebase, which is the correct approach for this PR.

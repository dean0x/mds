# Performance Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`compute_line_column` linear scan per serialization** - `error.rs:39-54` (Confidence: 65%) -- The `compute_line_column` function iterates byte-by-byte through `source[..offset]` for each call, making it O(n) in source length. If `serialize()` is called in a hot loop over many errors against the same source, this could become a bottleneck. However, error serialization is inherently a cold path (errors are infrequent), so the linear scan is appropriate for the current use case. Only worth revisiting if profiling shows this function appearing in flame graphs during batch error reporting.

- **`dependencies()` clones all keys on every call** - `resolver.rs:112-114` (Confidence: 70%) -- `ModuleCache::dependencies()` calls `.keys().cloned().collect()`, allocating a new `Vec<String>` and cloning every key string. Both `compile_with_deps` (line 530) and `compile_virtual_with_deps` (line 610) call this method once per compilation, then immediately consume the result. The cost is proportional to the number of resolved modules, which is bounded by `MAX_IMPORT_DEPTH` (64). For typical compilation runs (a handful of modules), this is negligible. A `Cow`-based or iterator-returning API would eliminate the allocation but adds complexity for no measurable gain at current scale.

- **`compile_virtual_with_deps` entry filtering uses string comparison** - `lib.rs:610` (Confidence: 60%) -- The `.filter(|k| k != entry)` on the dependencies iterator does an O(n) string comparison per element. The `compile_with_deps` variant (line 531) uses the more efficient `split_last()` approach which avoids per-element comparison entirely by relying on the post-order DFS invariant (entry is always last). The inconsistency is minor since module counts are small, but `compile_virtual_with_deps` could use the same `split_last()` pattern for consistency and marginal efficiency.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 9/10
**Recommendation**: APPROVED

### Rationale

This PR introduces no performance regressions. The key architectural decisions are sound:

1. **HashMap to IndexMap migration** (`resolver.rs`): IndexMap provides the same O(1) `get`/`insert`/`contains_key` as HashMap while preserving insertion order. The overhead (slightly larger per-entry memory for order tracking) is negligible for the bounded number of modules (capped at `MAX_IMPORT_DEPTH = 64`). This is the correct data structure choice for ordered dependency tracking without sacrificing lookup performance.

2. **`compute_line_column` is O(n) but on a cold path**: Error serialization only fires when errors occur, and the byte-scan over source text is the simplest correct approach. More complex alternatives (pre-built line offset tables) would add memory overhead and complexity for a path that rarely executes.

3. **`build_output` helper**: Extracting the repeated body/frontmatter pattern into a shared function eliminates code duplication without changing the execution profile. Each call site invokes it exactly once.

4. **`FileSystem::canonicalize` default is identity**: The virtual FS identity implementation (returning `path.to_string()`) allocates a new `String`, but this is a one-time cost per compilation call to `resolve_source`, not per-module.

5. **Dependency collection bounded**: All dependency lists are inherently bounded by `MAX_IMPORT_DEPTH` (64 modules), so the `.collect()` and `.to_vec()` calls operate on small, bounded collections.

No blocking or should-fix performance issues were identified.

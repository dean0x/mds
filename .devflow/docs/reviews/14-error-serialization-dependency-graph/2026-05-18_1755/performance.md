# Performance Review Report

**Branch**: HEAD -> main (PR #22)
**Date**: 2026-05-18

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`dependencies()` clones all keys into a new Vec on every call** - `crates/mds-core/src/resolver.rs:112`
**Confidence**: 85%
- Problem: `dependencies()` calls `self.modules.keys().cloned().collect()`, which heap-allocates a `Vec<String>` and clones every key. Each caller (`compile_with_deps`, `compile_virtual_with_deps`, `compile_str_with_deps`) then immediately chains `.into_iter().filter().collect()` — a second allocation. For the typical compile-once workflow this is negligible, but the API returns owned `String`s where borrowed `&str` references would avoid all cloning.
- Impact: Two heap allocations and N string clones per compile call. With MAX_IMPORT_DEPTH=64, worst case is 64 cloned strings plus two Vec allocations. Acceptable for a template compiler, but the API design forces cloning even when callers only need to iterate.
- Fix: Return `impl Iterator<Item = &str>` or `Vec<&str>` instead. This avoids cloning and lets callers who need owned strings `.to_string()` on demand. However, this changes the public API signature and may not be worth it if the dep count is always small.

```rust
// Current (allocates + clones all keys):
pub fn dependencies(&self) -> Vec<String> {
    self.modules.keys().cloned().collect()
}

// Alternative (zero-copy iteration):
pub fn dependencies(&self) -> impl Iterator<Item = &str> {
    self.modules.keys().map(|k| k.as_str())
}
```

Note: Since `CompileOutput.dependencies` is `Vec<String>` (needed for serde serialization), the cloning would still happen at the boundary. The real benefit is for callers who use `dependencies()` directly on `ModuleCache` without needing owned strings — e.g., the filter+collect in `compile_with_deps` could compare `&str` directly, then only clone the survivors.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`compute_line_column` linear scan is O(n) per call** - `crates/mds-core/src/error.rs:39` (Confidence: 65%) — The function iterates byte-by-byte over `source[..offset]` to compute line/column. This is called inside `serialize()`, which is only invoked on error paths (not hot paths). For error reporting the O(n) scan is fine. A line-offset index would only matter if serializing many errors against the same large source, which is not a current use case.

- **`compile_with_deps` calls `path.canonicalize()` after `resolve_path`** - `crates/mds-core/src/lib.rs:521` (Confidence: 70%) — This is a second filesystem `canonicalize()` syscall for the entry path (the first happens inside `NativeFs::normalize`). The result is used to filter the entry from the dependency list. The cost is one extra syscall per compile call, which is trivial compared to the actual compilation work. Could be avoided by having `resolve_path` return the canonical key, but this would require a larger API change.

- **`HashMap -> IndexMap` switch has minor overhead** - `crates/mds-core/src/resolver.rs:50` (Confidence: 60%) — IndexMap uses a hash table plus an ordered Vec internally. Per-operation overhead is ~constant-factor slower than HashMap (same O(1) amortized complexity). With MAX_IMPORT_DEPTH=64, the module cache is tiny. The indexmap crate was already a dependency (used for `IndexSet`), so no new dependency weight. The trade-off is well-justified: insertion-order preservation is required for correct dependency ordering, and the alternative (HashMap + separate Vec) would use more memory and be more error-prone.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The single MEDIUM finding (cloning in `dependencies()`) is a minor inefficiency that does not affect real-world performance given the bounded module count (max 64). The HashMap-to-IndexMap switch is well-justified and carries negligible overhead. `compute_line_column` is error-path-only and appropriately simple. The `canonicalize` double-call is one extra syscall per compile invocation. Overall, the performance characteristics of this PR are sound for a template compiler workload.

Condition: Consider whether `dependencies()` should return references in a future API iteration, but this is not blocking for merge.

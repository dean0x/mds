# Reliability Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18

## Issues in Your Changes (BLOCKING)

### HIGH

**`compile_with_deps` calls `path.canonicalize()` directly, bypassing `FileSystem` trait** - `crates/mds-core/src/lib.rs:521-524`
**Confidence**: 85%
- Problem: `compile_with_deps` uses `std::path::Path::canonicalize()` directly on the entry path to build the `entry_key` for filtering, while `resolve_source` was specifically changed in this PR to use `self.fs.canonicalize()` (fixing issue #21). This inconsistency means that if a custom `FileSystem` implementation overrides `canonicalize()`, the entry-key filtering in `compile_with_deps` will use a different normalization than the resolver used to build the cache key. The entry file could appear in the `dependencies` list (a correctness issue, not a reliability crash, but violates the documented contract). The code comments acknowledge this degradation ("If canonicalize fails we fall back...") but the root mismatch with the `FileSystem` abstraction is the real concern.
- Fix: Use the same `FileSystem::canonicalize()` that the resolver uses. Since `ModuleCache` owns the `fs`, expose a forwarding method or use the cache's `dependencies()` output to find the entry key by matching on the path that `resolve_path` returned:
```rust
// After resolve_path succeeds, the entry is the last-inserted key in the cache.
// Since resolve_path normalizes via fs.normalize("", &path_str), the cache key
// is already the canonical form. Use it directly:
let all_deps = cache.dependencies();
let entry_key = all_deps.last().cloned().unwrap_or_default();
let dependencies = all_deps.into_iter().filter(|k| k != &entry_key).collect();
```
Alternatively, since `resolve_path` returns the resolved module and the module was just inserted into the `IndexMap`, the entry key is always the last key in insertion order.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`dependencies()` allocates a full `Vec<String>` clone on every call** - `crates/mds-core/src/resolver.rs:112-114` (Confidence: 65%) -- `self.modules.keys().cloned().collect()` clones every key string into a new Vec. For large dependency graphs this is wasteful when callers only need to iterate or filter. Returning an iterator or a borrowed slice (`self.modules.keys()`) would be zero-allocation. Low severity since dependency graphs in template compilers are typically small.

- **`compute_line_column` is O(n) per error; no issue for single errors but could accumulate** - `crates/mds-core/src/error.rs:40-55` (Confidence: 60%) -- The function scans bytes from the start of source for every error serialization. If a caller serializes many errors from the same large source, the linear scans add up. Not a practical concern at current usage (single error per compilation), but worth noting if batch error serialization is added later. The loop is correctly bounded by `source.len()` so there is no unbounded-iteration issue.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The changes demonstrate strong reliability practices overall:

- **Bounded iteration**: `compute_line_column` iterates over `source[..offset].bytes()` which is bounded by `source.len()`. The `offset > source.len()` guard at line 41 ensures the slice is always valid.
- **No panic potential from unwrap/index access**: The new code avoids `.unwrap()` on fallible operations. The `unwrap_or_default()` and `unwrap_or_else()` patterns in `compile_with_deps` are correctly applied.
- **`dependencies()` collection is bounded**: The `IndexMap` can only contain as many entries as there are resolved modules, which is bounded by `MAX_IMPORT_DEPTH` (64).
- **Resource cleanup**: The `resolving` set push/pop with LIFO invariant checking (pre-existing, not modified) correctly handles cleanup even on error paths.
- **`FileSystem::canonicalize()` default is safe**: The identity default for virtual/in-memory backends is correct. The `NativeFs` override properly propagates errors as `MdsError::Io`.

The single blocking issue is the `std::path::Path::canonicalize()` call in `compile_with_deps` that bypasses the `FileSystem` trait abstraction that this PR specifically introduced for `resolve_source`. This is an inconsistency that should be addressed to maintain the reliability of the abstraction boundary, particularly for custom `FileSystem` implementations.

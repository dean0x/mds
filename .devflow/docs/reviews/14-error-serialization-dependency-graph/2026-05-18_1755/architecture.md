# Architecture Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18T17:55
**PR**: #22

## Issues in Your Changes (BLOCKING)

### HIGH

**`resolve_source` retains `&Path` parameter despite FileSystem abstraction** - `crates/mds-core/src/resolver.rs:238`
**Confidence**: 85%
- Problem: `resolve_source` accepts `base_dir: &Path` (an OS-specific type) and immediately converts it to a string via `base_dir.display().to_string()` to pass through the `FileSystem` abstraction. The method's doc comment already acknowledges this is "NativeFs-only," but the parameter type creates an ISP violation: callers using a custom `FileSystem` implementation must construct a `std::path::Path` from a string, only for it to be immediately converted back. The `canonicalize` fix (issue #21) correctly routes through the trait, but the `&Path` parameter undermines the abstraction boundary by coupling the public API to OS path semantics.
- Fix: Accept `base_dir: &str` instead of `&Path` to align with the string-key model used by the rest of the `FileSystem` trait. This would be a breaking change to a `pub` method, but the `#[non_exhaustive]` enum and pre-1.0 versioning indicate the API surface is still stabilizing. Alternatively, add a parallel `resolve_source_str(&str, ...)` method and deprecate the `Path` variant.

```rust
// Option A: change parameter type (breaking, appropriate pre-1.0)
pub fn resolve_source(
    &mut self,
    source: &str,
    base_dir: &str,
    runtime_vars: &HashMap<String, Value>,
    warnings: &mut Vec<String>,
) -> Result<Arc<ResolvedModule>, MdsError> {
    let canonical_str = self.fs.canonicalize(base_dir)?;
    self.fs.set_root(&canonical_str)?;
    // ...
}
```

### MEDIUM

**`CompileOutput.dependencies` uses `Vec<String>` instead of leveraging `IndexMap` iteration** - `crates/mds-core/src/lib.rs:75`
**Confidence**: 82%
- Problem: `CompileOutput` stores `dependencies: Vec<String>`, and the caller builds this by calling `cache.dependencies()` which clones all keys out of the `IndexMap` into a `Vec`, then filters. The `IndexMap` was introduced specifically for ordered dependency tracking, but the public type discards the map's O(1) lookup capability. For callers who need to check "is X a dependency?" (a common operation for build systems), this forces O(n) linear scan.
- Fix: This is a design trade-off rather than a clear bug. The `Vec<String>` is simpler for serialization and most consumers only iterate. If lookup becomes a need, consider exposing an `IndexSet<String>` or providing a `has_dependency(&str) -> bool` helper on `CompileOutput`. No immediate action required -- flagging for awareness as the dependency-graph API matures.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`compile_with_deps` duplicates canonicalize logic outside the FileSystem trait** - `crates/mds-core/src/lib.rs:521-524`
**Confidence**: 83%
- Problem: `compile_with_deps` calls `path.canonicalize()` (i.e., `std::fs::canonicalize`) directly to compute the entry key for filtering, rather than using the `FileSystem::canonicalize` method that `resolve_source` now correctly delegates to. This creates an inconsistency: `resolve_source` routes through the trait abstraction (the fix for #21), but `compile_with_deps` bypasses it. If a custom `FileSystem` implementation overrides `canonicalize` with different normalization semantics, the entry key computed here will not match the cache key, and the entry module will incorrectly appear in the `dependencies` list.
- Fix: Access the filesystem's `canonicalize` through the `ModuleCache`, or expose a method on `ModuleCache` that returns the entry key after resolution. Since `ModuleCache` owns the `fs: Box<dyn FileSystem>`, adding a `pub fn entry_key(&self, path: &Path) -> Result<String, MdsError>` method would centralize this logic.

```rust
// In lib.rs compile_with_deps, after resolve_path succeeds:
// Instead of:
let entry_key = path
    .canonicalize()
    .map(|p| p.display().to_string())
    .unwrap_or_else(|_| path.display().to_string());

// Prefer routing through the cache's filesystem:
// Option: add ModuleCache::canonicalize_key(&self, path_str: &str) -> Result<String, MdsError>
// that delegates to self.fs.canonicalize(path_str)
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`serialize()` match exhaustiveness vs `#[non_exhaustive]` tension** - `crates/mds-core/src/error.rs:537-567` (Confidence: 70%) -- The explicit match arms in `serialize()` enumerate all 16 variants and the refactor in commit 69f23ea makes the wildcard branch only match no-span variants. However, `MdsError` is `#[non_exhaustive]`, meaning external consumers cannot match exhaustively. The internal `serialize()` method is correct to be exhaustive (it lives in the same crate), but adding a new variant will require updating both the match in `serialize()` and the `at()` constructor pattern. A brief comment noting this coupling would help future maintainers.

- **`serde` as a runtime dependency for `SerializedError`/`SerializedSpan`** - `crates/mds-core/Cargo.toml:18` (Confidence: 65%) -- The `serde` derive on `SerializedError`, `SerializedSpan`, and `CompileOutput` makes `serde` a mandatory runtime dependency. For a compiler library, some consumers may want error types without paying the serde compilation cost. Consider gating `Serialize` derives behind a `serde` feature flag. This is a common pattern in Rust ecosystem crates (e.g., `miette/serde`, `indexmap/serde`). However, `serde` is already a transitive dependency through `serde_json` and `serde_yml`, so the marginal cost is currently zero -- this becomes relevant only if those dependencies are ever made optional.

- **`FileSystem::canonicalize` default returns identity -- potential semantic surprise** - `crates/mds-core/src/fs.rs:70-72` (Confidence: 62%) -- The default `canonicalize` implementation returns the input unchanged, which is correct for `VirtualFs` but could surprise custom implementors who forget to override it. The doc comment explains the intent, but a `/// # Important` note in the trait-level doc warning that custom implementations "MUST override `canonicalize` if path normalization differs from identity" would strengthen the security contract already documented for `normalize`.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

This PR demonstrates strong architectural judgment across four distinct changes:

1. **Error serialization** (`SerializedError`/`SerializedSpan`) -- Clean separation of concerns. The serializable types live alongside the error type they mirror, use `miette::Diagnostic` for drift-proof code extraction, and keep `compute_line_column` private. The `serialize()` method on `MdsError` follows the "convert at the boundary" pattern rather than polluting the domain type with serialization logic. Well-designed.

2. **FileSystem::canonicalize()** -- The fix for #21 correctly routes through the trait abstraction. Dependency direction is preserved: `resolver.rs` depends on the `FileSystem` trait (abstraction), not on `std::fs` (concretion). This is a textbook DIP improvement.

3. **CompileOutput with IndexMap** -- The `HashMap` to `IndexMap` swap in `ModuleCache.modules` is a clean, minimal change that adds ordered dependency tracking without altering the resolution algorithm. The `dependencies()` method exposes insertion-order semantics through a stable public type (`Vec<String>`).

4. **Layer boundaries** -- `error.rs` depends only on `miette`, `thiserror`, `serde`, and `std` -- no upward dependencies into resolver, evaluator, or other compiler internals. The `fs.rs` module correctly imports limits from `resolver.rs` (constants flow downward). `lib.rs` re-exports types from `error.rs` and `fs.rs` without leaking internal structure.

The one condition: the `compile_with_deps` bypass of `FileSystem::canonicalize` (the Should Fix item) creates an inconsistency with the #21 fix pattern. Routing that logic through the trait would complete the abstraction.

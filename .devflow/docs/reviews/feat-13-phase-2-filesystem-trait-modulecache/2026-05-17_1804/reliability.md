# Reliability Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### MEDIUM

**VirtualFs::read bypasses MAX_FILE_SIZE enforcement** - `crates/mds-core/src/fs.rs:114-119`
**Confidence**: 88%
- Problem: `NativeFs::read` (line 250) enforces the `MAX_FILE_SIZE` (10 MB) limit before returning content, but `VirtualFs::read` returns content of any size via `.cloned()` with no size check. A caller passing a HashMap with oversized values to `VirtualFs::new` (or a custom `FileSystem` implementor) could feed arbitrarily large content into the tokenizer/parser/evaluator pipeline, which may have downstream resource assumptions based on the 10 MB limit.
- Impact: In WASM or testing scenarios (VirtualFs's primary use case), an unreasonably large module value would consume unbounded memory through the compilation pipeline. Since the caller controls the HashMap, this is lower severity than for NativeFs (where the file comes from disk), but the inconsistency violates the principle that resource limits should be enforced uniformly at the `FileSystem` trait boundary.
- Fix: Add a size check in `VirtualFs::read` or document the limit as a trait-level contract:
```rust
fn read(&self, normalized: &str) -> Result<String, MdsError> {
    let content = self.modules
        .get(normalized)
        .ok_or_else(|| MdsError::file_not_found(normalized.to_string()))?;
    if content.len() as u64 > MAX_FILE_SIZE {
        return Err(MdsError::resource_limit(format!(
            "file too large ({} bytes, max {} bytes): {normalized}",
            content.len(),
            MAX_FILE_SIZE,
        )));
    }
    Ok(content.clone())
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**NativeFs::init_root computes find_project_root even when OnceLock is already set** - `crates/mds-core/src/fs.rs:208-212`
**Confidence**: 82%
- Problem: `init_root` always calls `Self::find_project_root(canonical_dir)` (which does filesystem I/O -- up to 256 `exists()` checks) before attempting `self.root_dir.set(root)`. When the `OnceLock` is already initialized, the `set()` silently discards the result, but the I/O has already been performed. This is called from `set_root` (line 268-274) which is invoked by `resolve_source` on every call.
- Impact: Low in practice -- `resolve_source` typically runs once per compilation, and `find_project_root` terminates quickly when a `.git` marker is found near the start. But the wasted I/O violates allocation discipline: avoid unnecessary work when the result will be discarded.
- Fix: Guard with a `get()` check before computing:
```rust
fn init_root(&self, canonical_dir: &Path) {
    if self.root_dir.get().is_some() {
        return;
    }
    let root = Self::find_project_root(canonical_dir);
    let _ = self.root_dir.set(root);
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **FileSystem trait lacks a size-limit contract in its doc comment** - `crates/mds-core/src/fs.rs:20-40` (Confidence: 70%) -- The `FileSystem` trait doc describes security properties as "implementation-specific" but does not mention resource limits (file size). Custom implementors could unknowingly skip size enforcement. Consider adding a doc note that `read()` implementations should enforce `MAX_FILE_SIZE`.

- **VirtualFs::read clones full content string on every cache miss** - `crates/mds-core/src/fs.rs:117` (Confidence: 65%) -- `.cloned()` on the HashMap value creates a full copy of the file content. For large virtual module sets this doubles peak memory per module. In practice bounded by MAX_IMPORT_DEPTH (64 modules), so unlikely to matter.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The changes demonstrate strong reliability engineering overall:
- All loops are bounded (find_project_root with MAX_TRAVERSAL_DEPTH, import resolution with MAX_IMPORT_DEPTH, VirtualFs::normalize segments bounded by input length)
- Cycle detection via IndexSet with LIFO invariant assertion is well-defended
- OnceLock for root_dir initialization prevents data races in concurrent scenarios
- No panics in production code paths; all unwrap() calls are in test code
- The read-then-check pattern in NativeFs::read correctly avoids TOCTOU races

The one condition is the VirtualFs MAX_FILE_SIZE gap -- the resource limit should be enforced consistently across all FileSystem implementations to prevent downstream pipeline assumptions from being violated.

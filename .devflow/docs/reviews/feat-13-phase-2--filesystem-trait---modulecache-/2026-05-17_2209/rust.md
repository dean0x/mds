# Rust Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17T22:09

## Issues in Your Changes (BLOCKING)

### HIGH

**NativeFs::normalize missing empty-path rejection** - `crates/mds-core/src/fs.rs:253`
**Confidence**: 92%
- Problem: The `FileSystem` trait's doc-level security contract (lines 26-31) requires that `normalize` must reject empty paths. `VirtualFs::normalize` enforces this at line 81 (`if relative.is_empty()`), but `NativeFs::normalize` has no such check. An empty `relative` would create a `Path` from `""`, which on most OSes resolves to the current directory. While `check_symlink` would likely fail on a directory, the behavior is inconsistent with the trait's stated contract and with `VirtualFs`.
- Fix:
```rust
fn normalize(&self, base: &str, relative: &str) -> Result<String, MdsError> {
    if relative.is_empty() {
        return Err(MdsError::import_error("import path is empty"));
    }
    if relative.contains('\0') {
        return Err(MdsError::import_error("import path contains null byte"));
    }
    // ... rest unchanged
```

**Missing `Debug` on public structs `VirtualFs` and `NativeFs`** - `crates/mds-core/src/fs.rs:64,159`
**Confidence**: 85%
- Problem: Both `VirtualFs` and `NativeFs` are public types (exported at `crates/mds-core/src/lib.rs:52`) but neither derives `Debug`. The Rust API Guidelines (C-DEBUG) recommend all public types implement `Debug`. `ResolvedModule` in the same crate derives `Debug, Clone`. Without `Debug`, users cannot include these types in debug-printed structs or use `{:?}` formatting in error messages. `NativeFs` contains `OnceLock<PathBuf>` which does implement `Debug`, and `VirtualFs` contains `HashMap<String, String>` which also does. Both can derive it straightforwardly.
- Fix:
```rust
#[derive(Debug)]
pub struct VirtualFs {
    modules: HashMap<String, String>,
}

// NativeFs can derive Debug too since OnceLock<PathBuf>: Debug
#[derive(Debug)]
pub struct NativeFs {
    root_dir: OnceLock<PathBuf>,
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`compile_virtual` takes `HashMap` by value, forcing callers to give up ownership** - `crates/mds-core/src/lib.rs:440-448`
**Confidence**: 82%
- Problem: Both `compile_virtual` and `compile_virtual_collecting_warnings` accept `modules: HashMap<String, String>` by value. This forces callers to move their `HashMap` into the function, which is the expected pattern for single-use compilation. However, if a caller wants to compile multiple entry points from the same module set, they must clone the entire HashMap each time. This is a minor API ergonomics concern, not a correctness issue. The `VirtualFs::new` constructor similarly takes by value, so this is consistent internally.
- Fix: Consider accepting `impl Into<HashMap<String, String>>` or documenting that callers should clone if reuse is needed. Alternatively, this is acceptable as-is if single-shot compilation is the expected use case.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`Path::display().to_string()` for normalized keys is lossy on non-UTF-8 paths** - `crates/mds-core/src/fs.rs:277`, `crates/mds-core/src/resolver.rs:113`
**Confidence**: 80%
- Problem: `canonical.display().to_string()` at `fs.rs:277` is used to convert the canonical `PathBuf` into the `String` key for the module cache. `Path::display()` is documented as potentially lossy on platforms with non-UTF-8 paths. On macOS and Linux with valid UTF-8 filenames (the common case) this works correctly. The alternative would be `canonical.to_str().ok_or_else(|| MdsError::io("path contains invalid UTF-8"))?.to_string()`, which would fail explicitly rather than silently replacing invalid bytes. This is pre-existing (the same pattern existed in the old `resolver.rs` code before this PR).
- Fix: Not blocking. Consider using `to_str()` with explicit error handling in a future PR if non-UTF-8 path support matters.

## Suggestions (Lower Confidence)

- **`FileSystem` trait could require `Debug`** - `crates/mds-core/src/fs.rs:36` (Confidence: 65%) -- Adding `Debug` as a supertrait (`pub trait FileSystem: Send + Sync + Debug`) would allow `ModuleCache` to derive `Debug`, improving diagnostics. However, this is a stricter API requirement for custom implementors.

- **`VirtualFs::read` clones content on every call** - `crates/mds-core/src/fs.rs:142` (Confidence: 60%) -- The `.clone()` is necessary given the trait's `Result<String, MdsError>` return type. An alternative would be returning `Cow<'_, str>` from the trait, but that would require lifetime parameters on the trait, significantly complicating the design. Not actionable without a trait redesign.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 1 | 0 |

**Rust Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The code demonstrates strong Rust idioms overall: proper use of `thiserror` for typed errors, `OnceLock` for thread-safe lazy initialization, `Arc` for shared ownership, `Box<dyn Trait>` for runtime polymorphism, and good defensive coding with defense-in-depth size checks. The trait design is clean with `Send + Sync` bounds and a well-documented security contract. The two HIGH findings are both straightforward fixes: adding the empty-path guard to `NativeFs` for contract consistency, and deriving `Debug` on the two new public types.

# Architecture Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**`resolve_source` breaks abstraction by calling `Path::canonicalize()` directly** - `crates/mds-core/src/resolver.rs:244`
**Confidence**: 85%
- Problem: `resolve_source` calls `base_dir.canonicalize()` (an OS syscall) directly in `ModuleCache`, bypassing the `FileSystem` abstraction layer. This means `ModuleCache` still has a hard dependency on the OS for this code path. If a caller were to create a `ModuleCache::with_fs(custom_fs)` and then call `resolve_source`, the `canonicalize()` call would go to the real OS regardless of the custom filesystem backend. The doc comment acknowledges this ("NativeFs-only"), but the method is still `pub` and available on all `ModuleCache` instances regardless of backend.
- Impact: Violates DIP -- the resolver (domain logic) reaches past its injected dependency to call OS directly. This creates a Liskov Substitution problem: swapping the filesystem backend does not fully swap filesystem behavior.
- Fix: Consider making `resolve_source` available only on NativeFs-backed caches, or moving the canonicalization into the `FileSystem` trait (e.g., a `canonicalize` method with a default that returns an error for non-OS backends). Alternatively, gate this method at runtime with a clear error when called on non-native backends:
```rust
// Option A: Move canonicalization into the trait
pub trait FileSystem: Send + Sync {
    fn canonicalize(&self, path: &str) -> Result<String, MdsError> {
        Err(MdsError::io("canonicalize not supported on this filesystem backend"))
    }
    // ...existing methods...
}

// Option B: Runtime guard in resolve_source
pub fn resolve_source(...) -> Result<Arc<ResolvedModule>, MdsError> {
    // The set_root call will already fail for backends that don't support it,
    // but canonicalize() runs first and would succeed on the OS even for
    // a VirtualFs-backed cache.
}
```

**`set_root` as a trait method with default no-op creates a split-brain contract** - `crates/mds-core/src/fs.rs:49-55`
**Confidence**: 82%
- Problem: The `FileSystem` trait has `set_root` as an optional method (default no-op). This creates two classes of implementations: those that need root initialization for security (NativeFs) and those that silently ignore it (VirtualFs). A custom `FileSystem` implementation could forget to implement `set_root` and silently bypass path traversal prevention, since the default succeeds without doing anything. The security contract in the doc comment mentions this obligation, but the type system does not enforce it.
- Impact: The trait's interface does not make the security-critical distinction between "root-aware" and "root-unaware" filesystems visible at compile time. This is an ISP concern -- `set_root` is only meaningful for a subset of implementations.
- Fix: Consider one of:
  1. Remove `set_root` from the trait entirely and handle root initialization inside `NativeFs::normalize` (which it already does via `init_root` on first entry point resolution). The `resolve_source` path could call `normalize` with a synthetic entry point instead.
  2. Make `set_root` a required method (no default) so implementors must consciously decide what to do.
  3. Accept the current design but add a `#[doc(notable_trait)]` or a stronger warning in the security contract.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`ModuleCache` does not implement `Debug`** - `crates/mds-core/src/resolver.rs:46`
**Confidence**: 80%
- Problem: The old `ModuleCache` derived `Default` (now manually implemented) but never had `Debug`. With the introduction of `Box<dyn FileSystem>`, `Debug` cannot be derived automatically. This is a minor API completeness gap -- `ResolvedModule` derives `Debug` and `Clone`, but `ModuleCache` (now a public type) has neither.
- Impact: Consumers of the public API cannot debug-print a `ModuleCache` instance. For a newly-public type this is a minor but notable gap.
- Fix: Implement `Debug` manually for `ModuleCache`:
```rust
impl std::fmt::Debug for ModuleCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleCache")
            .field("modules_count", &self.modules.len())
            .field("resolving_count", &self.resolving.len())
            .finish_non_exhaustive()
    }
}
```

## Pre-existing Issues (Not Blocking)

_No critical pre-existing architecture issues found in the reviewed files._

## Suggestions (Lower Confidence)

- **Consider sealing the `FileSystem` trait** - `crates/mds-core/src/fs.rs:36` (Confidence: 70%) -- The `with_fs` constructor accepts any `Box<dyn FileSystem>`, but the security contract is complex (traversal prevention, null-byte rejection, size limits). A sealed trait (using the `mod private { pub trait Sealed {} }` pattern) would prevent external crates from implementing `FileSystem` while still allowing `NativeFs` and `VirtualFs`, reducing the security surface. This should be weighed against the stated goal of custom backends for WASM.

- **`OnceLock` in `NativeFs` limits reusability across multiple compilation roots** - `crates/mds-core/src/fs.rs:160` (Confidence: 65%) -- `NativeFs` uses `OnceLock<PathBuf>` for `root_dir`, meaning a single `NativeFs` instance can only ever serve one project root. This is fine when `ModuleCache` creates its own `NativeFs` internally, but could surprise callers who share a `NativeFs` across multiple `ModuleCache` instances or compilations. The current `ModuleCache` constructors always create a fresh `NativeFs`, so this is not a practical issue today.

- **`ModuleCtx` changed from `base_dir: &Path` to `base_key: &str` -- good architectural direction** - `crates/mds-core/src/resolver.rs:593-602` (Confidence: 75%) -- The refactoring from `Path`-based to `String`-based keys throughout `ModuleCtx` is the core of this abstraction. However, `resolve_source` still constructs a `base_key` by concatenating a canonical OS path with `"/<source>"` (line 257), which leaks OS-path assumptions into what should be a backend-agnostic key space. This works because `resolve_source` is documented as NativeFs-only, but it creates a subtle coupling between the key format and NativeFs's `normalize` implementation.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

This is a well-executed architectural refactoring that successfully introduces the Strategy pattern (DIP) to decouple the module resolver from the OS filesystem. The key architectural wins:

1. **Clean trait design**: `FileSystem` has a minimal, focused interface (4 methods) that maps well to the resolver's needs -- this is a "deep module" in Ousterhout's terminology.
2. **Proper dependency injection**: `ModuleCache` accepts `Box<dyn FileSystem>` via constructor, following the DIP precisely.
3. **Key-based abstraction**: Replacing `PathBuf` with `String` keys throughout the resolver was the correct design choice for filesystem-agnostic operation.
4. **Security responsibilities correctly distributed**: `NativeFs` handles OS-level security (symlink rejection, canonicalization), while `VirtualFs` relies on its closed key-space -- each implementation owns its own security model.
5. **Backward compatibility preserved**: `ModuleCache::new()` and the existing public API functions (`compile`, `check`, etc.) continue to work unchanged.

The two HIGH findings are about the `resolve_source` method leaking OS assumptions and the `set_root` default no-op creating a gap in the security contract. These are real architectural tensions but are mitigated by documentation and the fact that `resolve_source` is explicitly marked as NativeFs-only. They should be addressed before Phase 4 (WASM) to avoid carrying the OS-coupling forward.

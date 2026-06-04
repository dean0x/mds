# Architecture Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T12:07

## Issues in Your Changes (BLOCKING)

No blocking architectural issues found.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Repeated `Path -> &str` conversion pattern across 4 public functions** (4 occurrences) -- Confidence: 82%
- `crates/mds-core/src/lib.rs:180-182`, `crates/mds-core/src/lib.rs:295-297`, `crates/mds-core/src/lib.rs:342-344`, `crates/mds-core/src/lib.rs:550-552`
- Problem: Four public API functions (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`) each perform the identical `path.to_str().ok_or_else(|| MdsError::io("path is not valid UTF-8"))?` conversion inline. This is a textbook DRY violation -- if the error message or conversion logic changes, four sites need updating in lockstep.
- Fix: Extract a small helper `fn path_to_str(path: &Path) -> Result<&str, MdsError>` alongside the existing `resolve_base_dir` helper, then call it from each public function:
  ```rust
  fn path_to_str(path: &Path) -> Result<&str, MdsError> {
      path.to_str()
          .ok_or_else(|| MdsError::io("path is not valid UTF-8"))
  }
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing architectural issues at CRITICAL severity.

## Suggestions (Lower Confidence)

- **Public API still accepts `impl AsRef<Path>` while internal API uses `&str`** - `crates/mds-core/src/lib.rs:107-108` (Confidence: 65%) -- The public API functions (`compile`, `check`, etc.) accept `impl AsRef<Path>` and immediately convert to `&str`, while the internal `resolve_path`/`resolve_source` now take `&str`. This creates a thin adaptation layer in every public function. Since the PR description closes #23 (moving to `&str`), it may be worth considering whether the public API should also accept `&str` directly (with a convenience overload for `Path`). However, keeping `AsRef<Path>` at the public boundary is a defensible ergonomic choice for Rust CLI callers, and changes to public API signatures carry their own risk. No action required.

- **`_setTransformerForTesting` uses fire-and-forget `void lazy.get()`** - `packages/webpack-loader/src/index.ts:79` (Confidence: 62%) -- The `void lazy.get()` call pre-resolves the LazyInit so subsequent `.get()` calls return synchronously. If the factory somehow rejects (it returns a sync value wrapped in `async`, so rejection is nearly impossible), the rejection would be unhandled. The `void` operator intentionally discards the promise. Given the factory is `async () => t` (trivially resolves), this is safe in practice but worth a comment clarifying the invariant.

- **Module-level mutable singleton `lazy` in webpack-loader** - `packages/webpack-loader/src/index.ts:24` (Confidence: 60%) -- The `let lazy: LazyInit<Transformer> | null = null` module-level mutable state is a pre-existing pattern, but the refactoring retained it. The LazyInit extraction is an improvement (centralizes init/retry logic), though the outer `null` check + first-call-captures-options pattern still means the module has implicit stateful coupling. This is documented and accepted for webpack's loader model.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

## Architectural Assessment

This PR demonstrates clean architectural thinking across two language boundaries:

**Rust core (mds-core)**: The `&Path` to `&str` migration in `resolve_path` and `resolve_source` is architecturally sound. It eliminates the lossy `path.display().to_string()` conversion that could silently corrupt non-UTF-8 paths, replacing it with explicit error handling at the boundary. The change follows the principle of pushing validation to the boundary layer (`lib.rs` public functions) while keeping the internal resolver (`resolver.rs`) simple. The `resolve_base_dir` helper properly centralizes the `Option<&Path>` to `String` conversion with UTF-8 validation.

**TypeScript (bundler-utils, webpack-loader)**: Extracting `LazyInit<T>` into bundler-utils is a good DRY refactoring. The class cleanly encapsulates the single-init pattern with proper retry-on-rejection semantics, and the `resolved` boolean flag correctly handles `T = void` and `T = null` edge cases. The webpack-loader simplification from ~20 lines of manual init bookkeeping to a single `LazyInit` instance is a net readability win.

**Cross-boundary consistency**: The NAPI and WASM bindings are unaffected because they call through the public `lib.rs` wrappers (`compile_str_with_deps`, `compile_with_deps`) which still accept `Path` types. The signature change is fully internal to the `ModuleCache` methods. This layering is correct -- the FFI boundary adapts types at its own level.

**Single condition for full approval**: Extract the repeated `path.to_str()` conversion into a helper to eliminate the 4-site duplication.

# Architecture Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23
**Prior Resolution Cycle**: 6 fixed, 0 false positive, 0 deferred (all resolved)

## Issues in Your Changes (BLOCKING)

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### 1. Boundary Design: Path-to-String Conversion (Rust)

The core architectural change narrows `ModuleCache::resolve_path` and `resolve_source` from `&Path` to `&str`, pushing the `Path -> &str` conversion to the outermost public API boundary in `lib.rs`. This is architecturally sound:

- **Single Responsibility**: The `path_to_str` helper and `resolve_base_dir` own all Path-to-str conversion and UTF-8 validation. The resolver no longer performs implicit lossy conversion via `path.display()`.
- **Boundary Validation**: Conversion happens once at the public API boundary (`check`, `compile`, `compile_with_deps`, etc.) which still accept `impl AsRef<Path>` for ergonomics, then pass `&str` inward. This is a textbook "parse at the boundary, trust internally" pattern.
- **DIP Compliance**: The resolver depends on the `&str` abstraction, not on `std::path::Path`. This improves testability and aligns with the `FileSystem` trait's existing string-based API (`normalize`, `canonicalize`, `read` all take `&str`).
- **Consistent Error Handling**: Non-UTF-8 paths now produce an explicit `MdsError::Io` at the boundary instead of silent corruption via `display().to_string()`. All four boundary functions use the same `path_to_str` helper (DRY -- previously addressed in prior resolution cycle, commit aae9a77).

### 2. LazyInit Extraction (TypeScript)

The `LazyInit<T>` class extracted into `@mds/bundler-utils` is well-designed:

- **Deep Module**: Simple 2-method interface (`get()`, `reset()`) hides significant complexity (concurrent dedup, retry-on-rejection, generation-based TOCTOU prevention). This matches Ousterhout's deep module principle.
- **Single Responsibility**: One class, one concern -- lazy single-initialization with concurrency safety.
- **DIP Compliance**: Depends on an injected `factory: () => Promise<T>`, not on any concrete initialization logic. Consumers in `transform.ts` and `webpack-loader/src/index.ts` inject their own factories.
- **Consistent Application**: Replaces two hand-rolled lazy-init patterns (`ensureInit` in transform.ts, `ensureTransformer` in webpack-loader) with a single reusable primitive. Both consumers now follow the same pattern: `new LazyInit(factory)` + `lazy.get()`.
- **Correct Generics**: Uses a `resolved` boolean flag rather than `instance !== undefined`, allowing `T = void` and `T = null`. This eliminates a category of bugs for factories with non-truthy return values.

### 3. Transformer Type Alias Consistency

The `type Transformer = ReturnType<typeof createMdsTransformer>` alias was added consistently across `vite-plugin`, `rollup-plugin`, and `webpack-loader`. This was addressed in the prior resolution cycle (commit 691c5ad) and is correctly applied. All three plugin packages now use the same alias pattern instead of repeating the verbose `ReturnType<typeof createMdsTransformer>` inline.

### 4. webpack-loader: Module-Level State Management

The webpack-loader's module-level `lazy` variable is acceptable for the webpack-loader pattern (loaders are stateless functions invoked per-file; singleton state is the standard approach). The comment on lines 18-23 documents this constraint clearly. The `_resetForTesting` and `_setTransformerForTesting` functions correctly handle the `LazyInit` lifecycle (reset clears via `lazy.reset()` + `lazy = null`; set creates a pre-resolved instance via `await lazy.get()`).

The `_setTransformerForTesting` becoming `async` (returning `Promise<void>`) is an intentional change to fix the fire-and-forget issue identified in the prior resolution cycle (commit 8a6df04). The test correctly `await`s the call.

### 5. API Surface Tests

The new tests in `api_surface.rs` (lines 696-719) serve as compile-time guards: they verify that `resolve_path` accepts `&str` and `resolve_source` accepts `&str` for `base_dir`. If someone reverts the signature, these tests fail at compile time. This is a lightweight but effective regression guard for the architectural invariant.

### Score Rationale (9/10)

**Strengths**:
- Clean boundary pattern (parse at boundary, trust internally)
- Reusable primitive extracted with correct concurrency semantics
- Consistent application across all three bundler plugins
- Good test coverage including compile-time signature guards
- All prior-cycle issues were resolved

**Minor deduction (-1)**: The `vite-plugin` and `rollup-plugin` each define their own `Transformer` type alias locally rather than exporting/importing one from `bundler-utils`. This is acceptable (they also define local structural plugin interfaces), but a shared export would reduce duplication further. Not blocking -- the current approach avoids a coupling decision (exporting `Transformer` from bundler-utils would tie plugin packages to the type alias name).

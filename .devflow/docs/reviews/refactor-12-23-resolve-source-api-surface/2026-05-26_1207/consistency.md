# Consistency Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T12:07

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent type alias pattern across bundler plugins** - `packages/webpack-loader/src/index.ts:16`
**Confidence**: 85%
- Problem: The webpack-loader introduces `type Transformer = ReturnType<typeof createMdsTransformer>` to avoid repetition, but the vite-plugin (`packages/vite-plugin/src/index.ts:39,40,54`) and rollup-plugin (`packages/rollup-plugin/src/index.ts:33,34,48`) still use inline `ReturnType<typeof createMdsTransformer>` in 3 locations each. This creates a split convention across sibling packages.
- Fix: This is a positive change in isolation (the alias is cleaner), but if introduced in one plugin it should be applied across all three for consistency. Consider extracting the type alias in a follow-up PR to vite-plugin and rollup-plugin, or exporting it from `@mds/bundler-utils`.

### MEDIUM

**Inconsistent `_setTransformerForTesting` signature across plugins** - `packages/webpack-loader/src/index.ts:73`
**Confidence**: 82%
- Problem: The webpack-loader's `_setTransformerForTesting(t: Transformer)` no longer accepts `null`, while vite-plugin and rollup-plugin both accept `t: ReturnType<typeof createMdsTransformer> | null`. The webpack-loader now uses `lazy = new LazyInit(async () => t)` which effectively wraps the value, so passing `null` is no longer a valid path -- callers must use `_resetForTesting()` instead. This is semantically correct but creates an asymmetric API surface for the test helpers across sibling packages.
- Fix: This divergence is justified by the different underlying mechanism (LazyInit vs direct assignment), but document the intent. If the vite/rollup plugins are refactored to use LazyInit in the future, align their `_setTransformerForTesting` signatures too.

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Vite and Rollup plugins do not use LazyInit for initialization** - `packages/vite-plugin/src/index.ts:54`, `packages/rollup-plugin/src/index.ts:48`
**Confidence**: 80%
- Problem: The PR extracts `LazyInit<T>` into `@mds/bundler-utils` and uses it in both `createMdsTransformer` and the webpack-loader, but the vite-plugin and rollup-plugin still use raw `let transformer: ... | null = null` with manual null-checking. While justified (Vite/Rollup have `buildStart` lifecycle hooks making LazyInit unnecessary), the init pattern is now inconsistent: `createMdsTransformer` and webpack-loader use `LazyInit`, while vite-plugin and rollup-plugin use manual initialization.
- Fix: No action needed for this PR -- the architectural difference (lifecycle hooks vs stateless loader) justifies the divergence. Consider a future harmonization if the plugins are refactored.

## Suggestions (Lower Confidence)

- **Repeated UTF-8 conversion boilerplate in lib.rs** - `crates/mds-core/src/lib.rs:180-182`, `295-297`, `342-344`, `550-552` (Confidence: 70%) -- The identical 3-line `path.to_str().ok_or_else(|| MdsError::io("path is not valid UTF-8"))?` block appears in 4 public functions (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`). A small helper function like `fn path_to_str(path: &Path) -> Result<&str, MdsError>` could reduce this repetition while keeping the error messages consistent.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR is internally consistent: the Rust-side `&Path` -> `&str` refactor is applied uniformly across all 4 public API functions that take `impl AsRef<Path>`, the `resolve_base_dir` helper is correctly updated, and the `resolve_path`/`resolve_source` signature changes are clean. The `LazyInit<T>` extraction is well-tested and correctly replaces the ad-hoc init patterns in both `createMdsTransformer` and the webpack-loader.

The main consistency concern is the divergence this creates across the three sibling bundler plugins (webpack-loader now uses `LazyInit` + type alias, while vite-plugin and rollup-plugin do not). This is architecturally justified but should be tracked for future harmonization. The blocking HIGH finding (type alias inconsistency) is a should-fix-while-here item -- either apply the `Transformer` alias pattern to all three plugins or defer it explicitly.

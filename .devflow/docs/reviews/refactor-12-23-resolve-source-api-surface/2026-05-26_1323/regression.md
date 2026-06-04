# Regression Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23

## Issues in Your Changes (BLOCKING)

No blocking regression issues found.

## Issues in Code You Touched (Should Fix)

No should-fix regression issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing regression issues found.

## Suggestions (Lower Confidence)

No lower-confidence suggestions.

## Analysis Summary

### Regression Checklist

- [x] **No exports removed without deprecation** -- All Rust public API functions (`compile`, `check`, `compile_with_deps`, etc.) retain their `impl AsRef<Path>` signatures. The signature changes to `resolve_path(&str)` and `resolve_source(&str)` are internal to `ModuleCache` (crate-public, not consumed outside `mds-core` directly). The `LazyInit` class is a new export from `@mds/bundler-utils`. No TypeScript exports were removed -- `_setTransformerForTesting` signatures in rollup-plugin and vite-plugin changed from `ReturnType<typeof createMdsTransformer>` to the equivalent `Transformer` type alias (no behavioral change). The webpack-loader's `_setTransformerForTesting` changed from sync to async, but its only caller (loader.spec.mjs:133) was updated to `await`.
- [x] **Return types backward compatible** -- `resolve_base_dir` changed from `Result<PathBuf, MdsError>` to `Result<String, MdsError>`, but this is a private function. Public API return types are unchanged.
- [x] **Default values unchanged** -- No default value changes detected.
- [x] **Side effects preserved** -- Warning emission, error handling, and logging behavior are preserved across all changed functions.
- [x] **All consumers of changed code updated** -- All 5 `resolve_path` call sites in `lib.rs` now pass `path_str` (from `path_to_str`). All 4 `resolve_source` call sites pass `&dir` (from `resolve_base_dir` which now returns `String`). The NAPI layer (`mds-napi/src/lib.rs`) calls the public API functions (not `resolve_path`/`resolve_source` directly), so it is unaffected.
- [x] **Migration complete across codebase** -- The hand-rolled `ensureInit` pattern in `transform.ts` replaced by `LazyInit<void>`. The hand-rolled singleton in `webpack-loader/src/index.ts` replaced by `LazyInit<Transformer>`. The `initPromise` pattern in `packages/mds/src/node.ts` is a different layer (user-facing `@mds/mds` init, not the bundler transformer singleton) and is correctly outside the scope of this migration.
- [x] **Commit message matches implementation** -- PR description accurately describes all three changes (#23 path API, #12 compile-time signature tests, #32 LazyInit extraction).
- [x] **Breaking changes documented** -- PR description notes this is a pre-release project with no migration needed.
- [x] **Const assertion removal safe** -- The `const _: () = assert!(MAX_FILE_SIZE > 0)` and `const _: () = assert!(MAX_TRAVERSAL_DEPTH > 0)` lines removed from `cli_import_pattern_works` test were duplicates of the assertions in the existing `constants_have_expected_values` test (lines 181-183 of api_surface.rs). No coverage lost.

### Intent vs Reality Verification

| PR Claim | Verified |
|----------|----------|
| `resolve_path` accepts `&str` instead of `&Path` | Yes -- `resolver.rs:131` changed from `path: &Path` to `path: &str` |
| `resolve_source` accepts `&str` for `base_dir` | Yes -- `resolver.rs:239` changed from `base_dir: &Path` to `base_dir: &str` |
| Public API still accepts `impl AsRef<Path>` | Yes -- all public functions in `lib.rs` retain `impl AsRef<Path>` |
| Conversion happens at boundary in `lib.rs` | Yes -- `path_to_str()` helper used at all 4 path-based entry points |
| Two new tests document `&str` signatures | Yes -- `module_cache_resolve_path_accepts_str` and `module_cache_resolve_source_accepts_str` |
| `LazyInit<T>` extracted to `@mds/bundler-utils` | Yes -- new `lazy-init.ts` with export in `index.ts` |
| `LazyInit` replaces hand-rolled `ensureInit` in `transform.ts` | Yes -- closure replaced by `new LazyInit<void>` |
| `LazyInit` replaces hand-rolled singleton in webpack-loader | Yes -- `ensureTransformer` replaced by `getLazy().get()` |

### Behavioral Equivalence

The `LazyInit` implementation preserves the behavioral contract of the replaced patterns:
1. **Single-init guarantee**: Factory called exactly once (tested in lazy-init.spec.mjs)
2. **Concurrent dedup**: Multiple simultaneous `get()` calls share one promise (tested)
3. **Retry on failure**: Rejection clears pending, next call retries (tested)
4. **TOCTOU safety**: Generation counter prevents stale factory results after `reset()` (tested -- this is an improvement over the original pattern which had no TOCTOU protection)
5. **Void support**: `T = void` and `T = null` handled via `resolved` boolean flag (tested)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

The migration is thorough and complete. All call sites are updated, no exports are lost, public API signatures are preserved, and the new `LazyInit` abstraction is well-tested with stronger guarantees than the patterns it replaces. The `&Path` to `&str` conversion in the resolver eliminates silent UTF-8 corruption (the motivating issue) while maintaining backward compatibility at the public API boundary. New compile-time signature tests provide a regression guard against accidental reversion.

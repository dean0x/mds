# Regression Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T12:07

## Issues in Your Changes (BLOCKING)

No blocking regression issues found.

## Issues in Code You Touched (Should Fix)

No should-fix regression issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing regression issues at CRITICAL severity in changed files.

## Suggestions (Lower Confidence)

- **Intentional public API break on ModuleCache** - `crates/mds-core/src/resolver.rs:129,236` (Confidence: 65%) -- `resolve_path` and `resolve_source` changed from `&Path` to `&str`. This is intentional per #23 and the project has zero external users (pre-release), but consider adding a note in a future CHANGELOG entry when the crate is published. Not flagged as blocking because the PR description explicitly documents this as the goal, all internal callers are updated, and MEMORY.md confirms zero users.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

### Detailed Analysis

**1. Lost Functionality**: None detected.
- No exports removed. No files deleted. No CLI options changed.
- Public convenience functions (`compile`, `check`, `compile_str_with`, etc.) retain identical signatures (`impl AsRef<Path>`, `Option<&Path>`).
- `ModuleCache` public methods `resolve_path` and `resolve_source` intentionally changed from `&Path` to `&str` per issue #23.

**2. Broken Behavior**: None detected.
- The `&Path` to `&str` conversion now happens at the caller boundary in `lib.rs` with explicit UTF-8 error handling, replacing the lossy `path.display().to_string()` that silently corrupted non-UTF-8 paths. This is strictly better behavior.
- `resolve_base_dir` returns `String` instead of `PathBuf` -- private function, no external impact.
- `PathBuf` import removed from `lib.rs` -- no remaining uses, removal is correct.
- `std::path::Path` import removed from `resolver.rs` -- no remaining uses, removal is correct.

**3. Intent vs Reality Mismatch**: None detected.
- Commit message claims `resolve_path`/`resolve_source` changed from `&Path` to `&str` -- confirmed in code.
- Commit message claims LazyInit extracted into bundler-utils -- confirmed (new `lazy-init.ts` with 7 tests).
- Commit message claims webpack-loader refactored to use LazyInit -- confirmed.
- Commit message claims `ensureInit` in transform.ts replaced -- confirmed.

**4. Incomplete Migrations**: None detected.
- All `resolve_path` callers in `lib.rs` (6 call sites) updated to pass `&str`.
- All `resolve_source` callers in `lib.rs` (4 call sites) updated to pass `&str` via `resolve_base_dir`.
- `mds-napi` and `mds-wasm` do not call `resolve_path`/`resolve_source` directly -- they use stable convenience wrappers.
- Old `ensureInit`/`ensureTransformer`/`initPromise` pattern fully replaced in both `transform.ts` and `webpack-loader/index.ts`.
- No references to old patterns remain in source code (only in `@mds/mds` node.ts which is a separate package not in this PR's scope).

**5. Test Coverage**:
- 314 mds-core tests pass (including 2 new compile-time signature tests).
- 218 mds-cli tests pass (including `resolve_source_nonexistent_base_dir_errors`).
- 16 bundler-utils transform tests pass.
- 7 new LazyInit unit tests pass (single-init, concurrency dedup, retry, void/null support, reset).
- 8 webpack-loader tests pass (with NODE_ENV=test).
- Redundant `const _: ()` assertions removed from `cli_import_pattern_works` -- these were duplicates of assertions in `constants_have_expected_values`.

**6. LazyInit Behavioral Equivalence**:
- `createMdsTransformer`: old inline `ensureInit` pattern (dedup via shared promise, retry on rejection) exactly matches `LazyInit.get()` semantics.
- `webpack-loader`: old `ensureTransformer` (singleton with shared `initPromise`, retry on rejection) exactly matches `getLazy(options).get()` via `LazyInit`.
- `_setTransformerForTesting`: old `transformer = t; initPromise = Promise.resolve()` equivalent to new `lazy = new LazyInit(async () => t); void lazy.get()` -- both pre-resolve so next `.get()` returns immediately.
- `_resetForTesting`: old `transformer = null; initPromise = null` equivalent to new `lazy?.reset(); lazy = null` -- both clear all state.

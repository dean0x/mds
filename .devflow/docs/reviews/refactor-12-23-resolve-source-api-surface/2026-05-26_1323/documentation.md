# Documentation Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`resolve_path` doc comment says "OS filesystem path" but parameter is now `&str`** - `crates/mds-core/src/resolver.rs:125-128`
**Confidence**: 90%
- Problem: The doc comment reads "Resolve a module from an OS filesystem path" and "Normalizes `path` to a canonical key via the underlying FileSystem". The wording "OS filesystem path" implies a `Path`/`PathBuf` type, but the parameter is now `&str`. While the semantics are still a path string, the doc should clarify this is a UTF-8 string representation of a path, not an OS-native path.
- Fix: Update the doc comment to reflect the new `&str` signature:
```rust
/// Resolve a module from a filesystem path string.
///
/// `path` is a UTF-8 string representation of the OS path (callers convert
/// `&Path` to `&str` at the public API boundary via `path_to_str`).
/// Normalizes `path` to a canonical key via the underlying [`FileSystem`],
/// then resolves through the module cache with cycle detection and depth guarding.
```

**`LazyInit` missing JSDoc on `get()` and `reset()` methods** - `packages/bundler-utils/src/lazy-init.ts:19,42`
**Confidence**: 85%
- Problem: `LazyInit` is a new public export from `@mds/bundler-utils`. The class-level JSDoc is excellent, but the two public methods `get()` and `reset()` have no JSDoc. Since this class is part of the package's public API (exported from `index.ts`), callers benefit from per-method documentation describing return semantics, retry behavior, and reset side effects.
- Fix: Add JSDoc to both methods:
```typescript
/**
 * Returns the cached value, or invokes the factory if not yet resolved.
 * Concurrent calls share the same in-flight promise. If the factory rejects,
 * the next call will retry.
 */
get(): Promise<T> {

/**
 * Clears the cached value and pending promise. The next `get()` call will
 * re-invoke the factory. Safe to call while a factory is in-flight -- the
 * stale result will be discarded via generation counter.
 */
reset(): void {
```

**`LazyInit` not documented in bundler-utils README** - `packages/bundler-utils/README.md`
**Confidence**: 82%
- Problem: `LazyInit` is now a public export of `@mds/bundler-utils` (visible in `src/index.ts` line 11), but the README usage example only shows `createMdsTransformer`, `formatMdsError`, and `shouldTransform`. The README import example on line 26 does not mention `LazyInit`. Consumers browsing the README would not know this utility exists.
- Fix: Add a brief section to the README after the existing "Usage" section:
```markdown
### LazyInit

A generic single-init lazy value holder with dedup and retry semantics,
used internally by the transform pipeline and available for plugin authors:

\```ts
import { LazyInit } from '@mds/bundler-utils';

const lazy = new LazyInit(async () => {
  const mds = await import('@mds/mds');
  return mds;
});

const mds = await lazy.get(); // factory called once, cached thereafter
\```
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**CHANGELOG missing entries for this branch's changes** - `CHANGELOG.md:8-9`
**Confidence**: 85%
- Problem: The `[Unreleased]` section of `CHANGELOG.md` does not document the three notable changes in this PR: (1) `resolve_path`/`resolve_source` signature change from `&Path` to `&str` (breaking API change for Rust library consumers), (2) the new `LazyInit<T>` extraction into `@mds/bundler-utils`, and (3) the non-UTF-8 path rejection behavior. The PR description references closing issues #23, #12, and #32, all of which represent notable changes. The existing `[Unreleased]` section documents `isMdsError()` and the bundler packages but not these changes.
- Note: The PR description states this is a pre-release project with no migration, which reduces urgency. However, the CHANGELOG already exists and follows Keep a Changelog conventions, so maintaining it is expected.
- Fix: Add entries under `[Unreleased]`:
```markdown
### Changed

- **`ModuleCache::resolve_path` and `resolve_source` accept `&str` instead of `&Path`** — eliminates silent UTF-8 corruption via `display()`. Non-UTF-8 paths now fail with an explicit error at the public API boundary. (#23)

### Added

- **`LazyInit<T>` utility** in `@mds/bundler-utils` — single-init lazy value holder with concurrent deduplication, retry-on-reject, and TOCTOU-safe `reset()`. Extracted from inline init logic in `createMdsTransformer` and `webpack-loader`. (#32)
- **API surface tests** for `resolve_path(&str)` and `resolve_source(&str)` signatures, plus non-UTF-8 path rejection tests on Unix. (#12)
```

## Pre-existing Issues (Not Blocking)

No pre-existing documentation issues identified at CRITICAL severity.

## Suggestions (Lower Confidence)

- **`_setTransformerForTesting` signature change undocumented in webpack-loader** - `packages/webpack-loader/src/index.ts:73` (Confidence: 65%) -- The function changed from sync to async (`Promise<void>` return). The JSDoc describes purpose but not the async contract. Callers in tests already adapted (line 133 of `loader.spec.mjs`), but the JSDoc could note the async requirement.

- **`path_to_str` could note the `path_to_str`/`resolve_base_dir` duality** - `crates/mds-core/src/lib.rs:255-258` (Confidence: 60%) -- Both `path_to_str` and `resolve_base_dir` perform the same UTF-8 conversion with nearly identical error messages. A brief cross-reference noting the two conversion points (single-path vs. optional-base-dir) would help maintainers understand the full boundary enforcement surface.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The code documentation within changed Rust files is generally strong -- `path_to_str`, `resolve_base_dir`, and the resolver methods all have clear doc comments explaining "why". The new `LazyInit` class has an excellent class-level JSDoc. The main gaps are: (1) `resolve_path`'s doc comment is slightly stale after the `&Path` to `&str` migration, (2) `LazyInit`'s public methods lack per-method JSDoc, (3) `LazyInit` is not mentioned in the bundler-utils README despite being a public export, and (4) the CHANGELOG does not yet reflect this branch's changes. None of these are blocking for merge, but the first three should be addressed before or shortly after merge.

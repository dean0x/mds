# Performance Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26

## Issues in Your Changes (BLOCKING)

No CRITICAL or HIGH performance issues found.

### MEDIUM

**`_setTransformerForTesting` fire-and-forget promise** - `packages/webpack-loader/src/index.ts:79`
**Confidence**: 82%
- Problem: `void lazy.get()` fires a promise that pre-resolves the LazyInit but the caller has no way to know when initialization completes. If test code calls `_setTransformerForTesting(t)` and immediately invokes the loader, there is a micro-task ordering dependency: `lazy.get()` inside the loader may run before or after the pre-resolve `.then()` callback settles. In practice this is nearly instant (the factory is `async () => t`), but it is a latent race where the factory runs twice if the timing aligns poorly (get() sees `pending !== null` on the second call, so it actually deduplicates correctly). The real cost is that every test setup creates an unnecessary intermediate Promise allocation when the result is already synchronously available.
- Fix: Since the factory is synchronous in effect (`async () => t`), the current approach works correctly thanks to LazyInit's dedup logic. However, for clarity and to avoid the extra microtask, consider awaiting in the caller or adding a `LazyInit.of(value)` static factory that sets the resolved state immediately without going through a promise:

```typescript
// Option: static pre-resolved factory on LazyInit
static of<T>(value: T): LazyInit<T> {
  const lazy = new LazyInit(async () => value);
  lazy.resolved = true;
  lazy.instance = value;
  return lazy;
}
```

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing performance issues found in the changed files.

## Suggestions (Lower Confidence)

- **Repeated `path.to_str()` pattern across 4 public functions** - `crates/mds-core/src/lib.rs:180,295,342,550` (Confidence: 65%) -- The same `path.as_ref().to_str().ok_or_else(...)` 3-line block is duplicated in `check`, `compile_collecting_warnings`, `check_collecting_warnings`, and `compile_with_deps`. While `to_str()` itself is O(1) (it checks a cached UTF-8 validity flag on Unix), the duplicated code is a maintainability concern rather than a performance one. A small helper like `fn path_to_str(p: &Path) -> Result<&str, MdsError>` would consolidate this without any runtime cost.

- **`resolve_base_dir` allocates a `String` on every call even for `Some(d)` path** - `crates/mds-core/src/lib.rs:216-219` (Confidence: 62%) -- The `Some(d)` branch calls `.map(str::to_owned)` which allocates a new String. Since `resolve_source` immediately borrows the result as `&str`, this allocation could theoretically be avoided if the downstream API accepted `Cow<str>`. However, the allocation is per-compilation (not per-file within a compilation), so the absolute cost is negligible. Noted for completeness.

- **`LazyInit.reset()` does not cancel an in-flight promise** - `packages/bundler-utils/src/lazy-init.ts:34-38` (Confidence: 68%) -- If `reset()` is called while a factory promise is in flight, the old promise continues running. When it resolves, it writes to `this.resolved` and `this.instance` on the now-reset LazyInit. This could cause a stale value to appear after reset. The test suite only tests reset after successful resolution, not during in-flight initialization. In practice this is only called in `_resetForTesting`, so production impact is nil, but the semantic gap could bite in future use.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 9/10
**Recommendation**: APPROVED

The changes are performance-neutral to slightly positive:

1. **Rust side**: Replacing `path.display().to_string()` (which lossy-converts non-UTF-8 via replacement characters) with `path.to_str().ok_or(...)` (which is a zero-cost validity check returning a borrow) removes an unconditional heap allocation in `resolve_path` and `resolve_source`. The new `to_str()` calls are O(1) on Unix (the OS string is already UTF-8 validated). The `to_owned()` calls in `resolve_base_dir` are equivalent cost to the previous `to_path_buf()`.

2. **TypeScript side**: Extracting `LazyInit<T>` is a structural refactor with identical runtime behavior to the inlined `ensureInit`/`ensureTransformer` patterns it replaces. No new allocations, no changed async behavior, no additional promise chains. The webpack-loader's `getLazy()` wrapper adds one null check per loader invocation (negligible).

No blocking performance issues. The single MEDIUM finding is a minor test-helper optimization opportunity, not a production concern.

# Performance Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26
**Cycle**: 2 (prior cycle resolved 6/6 issues)

## Issues in Your Changes (BLOCKING)

No blocking performance issues found.

## Issues in Code You Touched (Should Fix)

No should-fix performance issues found.

## Pre-existing Issues (Not Blocking)

No critical pre-existing performance issues found in changed files.

## Suggestions (Lower Confidence)

- **`resolve_base_dir` allocates a `String` on every call via `str::to_owned`** - `lib.rs:217` (Confidence: 65%) -- Each public API entry point (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`, and the `_str_with` family) calls `resolve_base_dir` which allocates a new owned `String` from the base dir. The resolver's `resolve_source` takes `&str`, so a `Cow<str>` return or passing a borrowed `&str` when the path is already UTF-8 would avoid this allocation. However, this is on a cold path (once per compile invocation, not per-file), so the impact is negligible in practice.

- **`LazyInit.get()` creates a new `Promise.resolve()` wrapper on every post-init call** - `lazy-init.ts:20` (Confidence: 70%) -- After the factory resolves, subsequent `get()` calls hit `Promise.resolve(this.instance as T)`, which allocates a new microtask-resolved Promise each time. Storing the resolved promise in `this.pending` (which is already available) would return the same Promise object on repeated calls, avoiding repeated allocation. In bundler plugin hot paths where `transform()` is called per-file, this is called once per module transformation. The impact is minor given that `Promise.resolve` is highly optimized in V8, but it is a missed opportunity.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This PR is a net performance improvement:

1. **Eliminates `path.display().to_string()` in `resolve_path`** (resolver.rs:135). The old code at `resolve_path` called `path.display().to_string()` which always allocates a new `String` and uses the lossy `Display` impl. The new code receives `&str` directly, passing it to `self.fs.normalize()` with zero allocation at the call site. The conversion cost moves to the public API boundary (`path_to_str`) where `Path::to_str()` is a zero-cost borrow check -- it returns `Some(&str)` if the OsStr is already UTF-8 (which it always is on macOS/Linux with UTF-8 locales), with no allocation.

2. **Eliminates `base_dir.display().to_string()` in `resolve_source`** (resolver.rs:245). Same pattern -- the old code allocated a `String` via `display()` on every `resolve_source` call. The new code receives `&str` directly and passes it to `self.fs.canonicalize()`.

3. **`LazyInit<T>` replaces hand-rolled init patterns** with identical runtime characteristics. The fast path (`this.resolved` check) is a single boolean comparison, matching or improving on the old `if (initialized) return` pattern. The generation counter adds a single integer comparison in the `.then()` callback (cold path, runs once). No performance regression.

4. **Type alias `Transformer`** in rollup-plugin and vite-plugin is purely cosmetic -- zero runtime impact.

The two lower-confidence suggestions are micro-optimizations on cold paths that do not warrant blocking.

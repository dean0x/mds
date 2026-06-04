# Reliability Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`findProjectRoot` uses synchronous I/O (`existsSync`) in an async hot path** - `packages/mds/src/util/module-scanner.ts:51`
**Confidence**: 85%
- Problem: `findProjectRoot` calls `existsSync` in a loop (up to `MAX_TRAVERSAL_DEPTH=256` iterations x 2 markers = 512 synchronous filesystem calls). This function is invoked from the async `buildModulesMap`, which is called per-file by the Webpack loader. Although results are cached after the first call per start directory, the first invocation for each unique directory blocks the Node.js event loop for the full traversal. On network filesystems or deep directory trees, this can stall the build pipeline for seconds.
- Fix: Convert to an async function using `fs.promises.access` or `fs.promises.stat` instead of `existsSync`. The cache logic remains identical:
```typescript
async function _findProjectRootUncached(start: string): Promise<string> {
  let dir = start;
  for (let i = 0; i < MAX_TRAVERSAL_DEPTH; i++) {
    for (const marker of PROJECT_ROOT_MARKERS) {
      try {
        await access(resolve(dir, marker));
        return dir;
      } catch {
        // marker not found, continue
      }
    }
    const parent = dirname(dir);
    if (parent === dir) return start;
    dir = parent;
  }
  return start;
}
```

**`projectRootCache` is a module-level `Map` with no eviction** - `packages/mds/src/util/module-scanner.ts:24`
**Confidence**: 82%
- Problem: The `projectRootCache` grows without bound. In long-running processes (Webpack dev server with watch mode), every unique `start` directory creates a new cache entry that is never evicted. For monorepos with many packages, each unique source directory adds an entry. The cache is also module-global, meaning it leaks across separate Webpack compiler instances or test runs.
- Fix: Either (a) add a `clearProjectRootCache()` function exposed for testing and called by `_resetForTesting`, or (b) use a bounded LRU with a reasonable cap (e.g. 128 entries). Option (a) is simpler and sufficient for the intended use case:
```typescript
export function _clearProjectRootCacheForTesting(): void {
  projectRootCache.clear();
}
```
And call it from `_resetForTesting` in the webpack-loader.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`evaluate_if` iterates `elseif_branches` without an independent runtime bound** - `crates/mds-core/src/evaluator.rs:377`
**Confidence**: 80%
- Problem: The `for (cond, body) in &block.elseif_branches` loop in `evaluate_if` relies entirely on the parser's `MAX_ELSEIF_BRANCHES` (256) limit. If the parser limit were bypassed (e.g. via programmatic AST construction in tests or future API consumers), the evaluator would iterate an unbounded number of branches, each evaluating a condition and potentially a full body. The evaluator should have its own defensive assertion.
- Fix: Add a debug assertion at the top of `evaluate_if` (lightweight, stripped in release):
```rust
debug_assert!(
    block.elseif_branches.len() <= MAX_ELSEIF_BRANCHES,
    "invariant: elseif_branches must not exceed MAX_ELSEIF_BRANCHES"
);
```
This follows the assertion density principle (NASA/JPL rule) -- validate invariants at consumption points, not just production points.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`scan` function uses `Promise.all` for parallel child reads without concurrency bound** - `packages/mds/src/util/module-scanner.ts:321`
**Confidence**: 85%
- Problem: When a module has many imports (e.g. a barrel file importing 50+ modules), `Promise.all(importPaths.map(...))` opens all file descriptors simultaneously. Combined with the recursive nature of `scan`, a deep tree with wide fan-out could exhaust file descriptor limits. The `maxModules` guard limits total count but not concurrency at any single level.
- Fix: Use a bounded concurrency utility (e.g. `p-limit` or a simple semaphore) to cap parallel filesystem operations per scan level. Alternatively, process imports sequentially since file I/O is fast for local filesystems and the total count is bounded by `maxModules`.

## Suggestions (Lower Confidence)

- **`_esmImport` lacks a timeout or abort mechanism** - `packages/webpack-loader/src/index.ts:17` (Confidence: 65%) -- The `new Function('id', 'return import(id)')` call produces a promise that could hang if module resolution stalls. A `Promise.race` with a timeout would prevent silent hangs in the lazy init path.

- **`elseif_branches` Vec allocation strategy** - `crates/mds-core/src/parser.rs:275` (Confidence: 62%) -- `Vec::new()` starts with zero capacity. Most real templates use 1-3 `@elseif` branches. A `Vec::with_capacity(4)` would avoid reallocation in the common case while remaining negligible for memory.

- **`findProjectRoot` cache does not normalize paths** - `packages/mds/src/util/module-scanner.ts:37` (Confidence: 70%) -- If the same physical directory is passed with different string representations (e.g. trailing slash vs. no trailing slash, or symlinked paths), cache misses cause redundant traversals. Using `path.resolve()` on `start` before the cache lookup would improve hit rates.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Reliability Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Assessment

The changes introduce significant new language features (negation, equality, `@elseif`) and CJS compatibility for the Webpack loader. The Rust parser and evaluator changes are well-bounded: `MAX_NESTING_DEPTH` reduced from 256 to 64, `MAX_ELSEIF_BRANCHES` enforced before body parsing (applies ADR-001 -- pre-merge quality gate via structured resource limits), and `MAX_DOT_SEGMENTS` reused in the new `parse_dot_path` helper. The `find_unquoted_operator` scanner properly handles escape sequences to avoid premature string termination.

The primary reliability concerns are in the TypeScript module scanner: the synchronous `existsSync` traversal blocking the event loop, and the unbounded cache growth. These are not data-loss risks but can degrade build performance in watch mode or on network filesystems. The evaluator's lack of an independent runtime bound on `elseif_branches` is a defense-in-depth gap that a debug assertion would close cheaply.

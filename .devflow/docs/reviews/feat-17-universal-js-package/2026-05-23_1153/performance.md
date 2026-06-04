# Performance Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Cycle**: 4 (incremental)

## Cross-Cycle Awareness

Cycle 3 resolved the `aggregateSize` non-atomic false-positive (JS is single-threaded). That finding is not re-raised. Deep-freeze of `DEFAULT_COMPILE_OPTS` and `compileOpts()` helper extraction were applied in prior cycles; this review evaluates the current state only.

## Issues in Your Changes (BLOCKING)

### HIGH

**Unnecessary object allocation in compileOpts when vars are present** - `packages/mds/src/backend/wasm.ts:144-147`
**Confidence**: 82%
- Problem: When vars are provided, `compileOpts` spreads both `DEFAULT_COMPILE_OPTS` **and** the `vars` wrapper object:
  ```typescript
  return vars !== undefined ? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS;
  ```
  `varsOpt()` returns `{ vars: ... }` — a fresh wrapper. The spread then creates *another* fresh object merging the frozen defaults with the wrapper. This is two allocations per compile/check call with vars. For high-throughput compilation (e.g., batch processing many templates), this adds up.
- Fix: Construct a single object directly instead of double-spreading:
  ```typescript
  function compileOpts(options?: CompileOptions): { filename: string; modules: Record<string, string>; vars?: Record<string, unknown> } {
    if (options?.vars != null) {
      return { filename: 'input.mds', modules: DEFAULT_COMPILE_OPTS.modules, vars: options.vars };
    }
    return DEFAULT_COMPILE_OPTS;
  }
  ```
  This eliminates the intermediate `{ vars }` wrapper from `varsOpt` and the spread of the frozen defaults, reducing to one allocation.

### MEDIUM

**Sequential candidate loading in WASM init** - `packages/mds/src/backend/wasm.ts:111-117`
**Confidence**: 80%
- Problem: `_init()` loops through WASM candidates sequentially, awaiting `tryLoadCandidate` for each. The first candidate constructs a `new URL(...)` path and calls `createRequire` + `require()`, which involves filesystem access. If the first candidate fails (common in npm-install scenarios where the workspace path does not exist), the full load-and-fail cycle must complete before the second candidate is tried.
- Impact: Init latency increases by the time spent failing on the first candidate. For a one-time init this is minor, but in serverless cold-start scenarios every millisecond matters.
- Fix: This is a low-risk optimization since init is called once. Consider a speculative parallel load with `Promise.any` if cold-start performance becomes a concern:
  ```typescript
  const mod = await Promise.any(
    candidates.map(c => tryLoadCandidate(c, require, options?.wasmUrl)
      .then(m => { if (m === null) throw new Error('not found'); return m; })
    )
  ).catch(() => null);
  ```
  Not blocking — the sequential approach is correct and clear.

## Issues in Code You Touched (Should Fix)

*(none)*

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Dynamic import of node:module on every WASM init attempt** - `packages/mds/src/backend/wasm.ts:101`
**Confidence**: 85%
- Problem: `_init()` uses `await import('node:module')` each time it is called. While Node.js caches dynamic imports, the cache lookup and promise machinery still execute. Since `_init` can be called up to `MAX_INIT_RETRIES` times (3), this runs the dynamic import resolution path multiple times.
- Impact: Negligible in practice since `init` is called once per process and Node.js module cache handles subsequent calls. The frozen `initPromise` pattern ensures `_init` is not re-entered after success.
- Fix (informational): Could hoist `createRequire` to module scope via a top-level `import`, but that would break browser bundleability (the whole point of the dynamic import). Current approach is correct for the universal package goal.

## Suggestions (Lower Confidence)

- **URL construction on every init call** - `packages/mds/src/backend/wasm.ts:106` (Confidence: 65%) — `new URL('../../../../crates/mds-wasm/pkg/mds_wasm.js', import.meta.url).pathname` is constructed every time `_init` runs. Could be hoisted to module scope since `import.meta.url` is stable. Minor since init runs once.

- **Parallel lstat + realpath in statAndValidateModule** - `packages/mds/src/util/module-scanner.ts:139` (Confidence: 70%) — `Promise.all([lstat, realpath])` is good parallelism, but `readFile` at line 208 is a separate await. These three operations could theoretically be batched together (stat + read in one pass). However, the current design intentionally checks size limits *before* reading content to avoid loading oversized files into memory — this is a deliberate tradeoff favoring memory safety over I/O throughput. Current approach is correct.

- **compileFile/checkFile spread varsOpt inline** - `packages/mds/src/backend/wasm.ts:171-172` (Confidence: 62%) — `compileFile` and `checkFile` still use inline `...varsOpt(options)` spread rather than the new `compileOpts()` helper. This is intentional since file operations build a custom options object with the resolved `modules` map, but it means varsOpt still creates the wrapper `{ vars }` object that gets spread. Minor allocation on per-file operations.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates strong performance awareness: frozen default options avoid per-call allocation on the hot path, `Promise.all` parallelizes independent I/O, aggregate size limits prevent memory exhaustion, and the visited-set deduplication is O(1). The blocking HIGH finding (double allocation in `compileOpts`) is a minor optimization opportunity for high-throughput scenarios. The sequential WASM candidate loading is correct and only affects one-time init. Overall, performance characteristics are appropriate for a compiler bindings package.

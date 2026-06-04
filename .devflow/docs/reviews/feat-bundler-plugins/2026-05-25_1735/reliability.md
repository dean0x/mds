# Reliability Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25
**Commits reviewed**: bdbba30..f58fa41 (5 commits)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Promise deduplication in ensureInit does not handle rejection — permanently poisoned singleton** - `packages/bundler-utils/src/transform.ts:31-38`
**Confidence**: 90%
- Problem: `ensureInit()` caches the `initPromise` after the first call. If `mds.init()` rejects (e.g., WASM load failure due to transient network issue), the rejected promise is cached forever. Every subsequent call to `ensureInit()` will `await` the same rejected promise and fail, with no ability to retry. The `initialized` flag never becomes `true`, and `initPromise` is never reset to `null`.
- Impact: A single transient init failure permanently breaks all transform calls for the lifetime of the process. In a long-running dev server (Vite/webpack), this means a restart is required.
- Fix: Reset `initPromise` on rejection so the next call can retry:
```typescript
async function ensureInit(): Promise<void> {
  if (initialized) return;
  if (initPromise === null) {
    initPromise = mds.init().then(() => {
      initialized = true;
    }).catch((err) => {
      initPromise = null;   // allow retry on next call
      throw err;
    });
  }
  return initPromise;
}
```

**Same poisoned-promise pattern in webpack loader singleton** - `packages/webpack-loader/src/index.ts:17-27`
**Confidence**: 90%
- Problem: Identical issue to the above. The module-level `initPromise` in the webpack loader caches the result of `import('@mds/mds')`. If the dynamic import fails (e.g., module resolution error, corrupted node_modules), the rejected promise is cached and `transformer` stays `null` forever.
- Impact: Webpack builds would require a full process restart to recover from any init-time failure.
- Fix: Apply the same catch-and-reset pattern:
```typescript
async function ensureTransformer(options: MdsPluginOptions): Promise<NonNullable<typeof transformer>> {
  if (transformer !== null) return transformer;
  if (initPromise === null) {
    initPromise = import('@mds/mds').then((mds) => {
      transformer = createMdsTransformer(mds, options);
    }).catch((err) => {
      initPromise = null;  // allow retry on next call
      throw err;
    });
  }
  await initPromise;
  return transformer!;
}
```

### MEDIUM

**Webpack loader ensureTransformer ignores options drift after first init** - `packages/webpack-loader/src/index.ts:15-27`
**Confidence**: 85%
- Problem: `ensureTransformer(options)` accepts `options` as a parameter, but only uses it during the very first call (when creating the transformer). If a webpack configuration passes different `vars` for different rules or loaders targeting the same module, the second set of options is silently discarded because `transformer` is already set. The function signature implies per-call options are supported, but the singleton pattern contradicts this.
- Impact: Subtle correctness bug if multiple webpack rules invoke the loader with different options. The `vars` from the first invocation win silently.
- Fix: Either (a) assert that options are identical on subsequent calls, or (b) document explicitly that options are only read once and remove the parameter from subsequent calls:
```typescript
let cachedOptions: MdsPluginOptions | null = null;

async function ensureTransformer(options: MdsPluginOptions): Promise<NonNullable<typeof transformer>> {
  if (transformer !== null) {
    // In debug/dev mode, you could assert deep equality here
    return transformer;
  }
  // ... rest of init
  cachedOptions = options;
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **escapeForJs builds string via concatenation in a character-by-character loop** - `packages/bundler-utils/src/transform.ts:6-22` (Confidence: 60%) — For very large compiled outputs, character-by-character string concatenation with `+=` may cause excessive intermediate string allocations. A regex-based replace or array-join pattern would be more allocation-friendly. However, V8 optimizes string concatenation well in practice, and compiled MDS outputs are unlikely to be megabytes, so this is unlikely to be a real issue.

- **Rollup and Vite plugins silently return null if transform called before buildStart** - `packages/rollup-plugin/src/index.ts:32`, `packages/vite-plugin/src/index.ts:37` (Confidence: 65%) — If a bundler invokes `transform` before `buildStart` (which would be a bundler contract violation), the `transformer === null` guard returns `null` silently. This is arguably correct defensive coding, but an assertion or warning would make debugging easier if bundler lifecycle ordering ever changes.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The overall design is solid: bounded file reads (512 bytes), proper try/finally resource cleanup on file handles, good error propagation through formatMdsError, and appropriate use of async callbacks in the webpack loader. The main reliability concern is the poisoned-promise pattern in both init singletons (bundler-utils transform.ts and webpack-loader index.ts), where a transient init failure permanently breaks the process. These should be fixed before merge since they affect long-running dev server scenarios.

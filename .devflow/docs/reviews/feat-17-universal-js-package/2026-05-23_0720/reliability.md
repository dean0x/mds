# Reliability Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20

## Cross-Cycle Awareness

Prior cycle (Cycle 2) resolved 18/21 issues including key reliability fixes: MAX_IMPORT_DEPTH=64, symlink rejection, path traversal guard, sequential-to-parallel I/O. One deferred item: WASM init retry circuit breaker untested (architectural). This review verifies those fixes landed correctly and checks for new reliability concerns.

## Issues in Your Changes (BLOCKING)

### HIGH

**Mutable `DEFAULT_COMPILE_OPTS.modules` shared across calls** - `packages/mds/src/backend/wasm.ts:106`
**Confidence**: 90%
- Problem: `DEFAULT_COMPILE_OPTS` is `Object.freeze()`-d, but only shallowly. The `modules` property is an empty object literal `{}` that is shared by reference across every `compile()` and `check()` call that takes the no-vars fast path. If the downstream WASM module mutates the `modules` object (e.g., adding entries during compilation), all subsequent calls would see the stale mutation, leading to incorrect compilation results or state leakage between calls.
- Impact: If the WASM `compile()`/`check()` implementations treat `modules` as mutable (common in JS FFI boundaries), this creates cross-call state pollution. Even if current WASM code does not mutate it, this is a latent reliability hazard -- any future change to the WASM side that writes to `modules` silently breaks isolation.
- Fix: Deep-freeze the modules object, or create a fresh empty object per call on the no-vars path:
```typescript
// Option A: deep freeze (safest if WASM respects frozen objects)
const DEFAULT_COMPILE_OPTS = Object.freeze({ 
  filename: 'input.mds', 
  modules: Object.freeze({} as Record<string, string>) 
});

// Option B: fresh object per call (safest if WASM mutates)
return wasm.compile(source, vars !== undefined 
  ? { filename: 'input.mds', modules: {}, ...vars } 
  : { filename: 'input.mds', modules: {} });
```

### MEDIUM

**`aggregateSize` not atomic across parallel scans** - `packages/mds/src/util/module-scanner.ts:188`
**Confidence**: 82%
- Problem: `aggregateSize` is a plain `let` variable incremented inside `scan()`, which runs children in parallel via `Promise.all()`. While JavaScript is single-threaded (no true data race), the check-then-act pattern (`aggregateSize += stats.size; if (aggregateSize > max)`) can be interleaved by `await` points. Multiple parallel children could each read `aggregateSize` before any of them writes, then each adds their size. All N children pass the check individually, but the aggregate exceeds the limit. The pre-reservation comment on line 184-186 acknowledges this exact scenario but the mitigation (using `stats.size` from OS metadata) only reduces the window -- it does not eliminate it.
- Impact: The aggregate size limit could be overshot by up to `(N-1) * maxFileSize` where N is the number of parallel siblings. For the default 10 MiB limit, a fan-out of 10 children each at 1.5 MiB would individually pass but collectively consume 15 MiB. This is bounded by `maxModules` (256) so it cannot grow unboundedly, but the limit becomes advisory rather than strict.
- Fix: Reserve the size atomically before spawning parallel children, or serialize the size check while parallelizing the I/O:
```typescript
// Pre-check all children sizes before reading content
const childStats = await Promise.all(
  importPaths.map(async (importPath) => {
    const childAbsolute = validateImportPath(importPath, absoluteDir);
    return { importPath, childAbsolute, stats: await lstat(childAbsolute) };
  }),
);
// Sequential size reservation (synchronous, no interleaving)
for (const child of childStats) {
  aggregateSize += child.stats.size;
  if (aggregateSize > maxAggregateSize) {
    throw new Error(`resource limit: aggregate module size exceeds maximum of ${maxAggregateSize} bytes`);
  }
}
// Now parallelize the reads
await Promise.all(childStats.map(/* ... */));
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Browser `init()` retry has no caller-visible bound** - `packages/mds/src/browser.ts:37-51`
**Confidence**: 85%
- Problem: The comment on line 27 says "wasm.ts's MAX_INIT_RETRIES enforces a permanent failure bound," and this is structurally correct -- `wasm.ts` has `MAX_INIT_RETRIES=3` with a failure counter. However, `browser.ts`'s `init()` resets `initVoidPromise = null` on every rejection (line 47), allowing unlimited retry attempts from the browser caller's perspective. The `MAX_INIT_RETRIES` guard only fires inside `wasm.ts`'s `init()`, but `browser.ts`'s `createWasmBackend(options)` calls `wasm.ts`'s `init()` internally (line 112 of `wasm.ts`). The retry bound works if and only if the module-level `initFailures` counter in `wasm.ts` persists across calls. Since `initFailures` is module-level state, this does work correctly in practice. The concern is that the comment in `browser.ts` documenting this relationship is the only thing preventing a future refactor from breaking the invariant (e.g., if someone moves `createWasmBackend` to a factory pattern that creates fresh state).
- Impact: Low immediate risk since the current code is correct. This is a fragile cross-module invariant that should be explicitly tested. The prior cycle noted "WASM init retry circuit breaker untested (architectural)" -- this remains unresolved.
- Fix: Add an explicit test that verifies `init()` stops retrying after MAX_INIT_RETRIES failures, or add a local retry guard in `browser.ts` as defense-in-depth:
```typescript
let browserInitFailures = 0;
const MAX_BROWSER_RETRIES = 3;

export function init(options?: InitOptions): Promise<void> {
  if (resolvedBackend !== undefined) return Promise.resolve();
  if (browserInitFailures >= MAX_BROWSER_RETRIES) {
    return Promise.reject(new Error('@mds/mds: init() failed permanently after retries'));
  }
  if (initVoidPromise !== null) return initVoidPromise;
  initVoidPromise = createWasmBackend(options)
    .then((b) => { resolvedBackend = b; })
    .catch((err) => {
      browserInitFailures += 1;
      initVoidPromise = null;
      throw err;
    });
  return initVoidPromise;
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**WASM candidate loading loop has no explicit iteration bound annotation** - `packages/mds/src/backend/wasm.ts:75`
**Confidence**: 80%
- Problem: The `for (const candidate of candidates)` loop on line 75 iterates over the `candidates` array (line 67-72), which is a fixed-length array of 2 elements. This is structurally bounded, but the array is constructed dynamically. If a future developer adds candidates without noticing this is a retry-like pattern, the loop could grow unbounded.
- Impact: Minimal with current code. The array is locally scoped and has 2 elements.
- Fix: Add a `MAX_LOAD_CANDIDATES` assertion or comment documenting the expected bound.

## Suggestions (Lower Confidence)

- **Depth parameter default value on recursive function** - `packages/mds/src/util/module-scanner.ts:133` (Confidence: 70%) -- The `depth` parameter has a default value of `0`, which is correct for the entry point but means any caller could accidentally reset the depth counter by omitting the argument. Making depth required and passing `0` explicitly at the call site (line 215) would prevent accidental misuse.

- **No timeout on WASM `mod.default()` call** - `packages/mds/src/backend/wasm.ts:82` (Confidence: 65%) -- The `await mod.default(options?.wasmUrl)` call has no timeout. In browser environments, if the WASM URL is unreachable but the connection hangs (rather than failing), this could block indefinitely. A `Promise.race` with a timeout would bound the wait.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Reliability Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The branch demonstrates strong reliability engineering -- MAX_IMPORT_DEPTH=64 is well-implemented with clear comments, the depth guard correctly precedes all I/O, and the parallel I/O refactoring is sound. The primary concern is the shared mutable `modules` object in `DEFAULT_COMPILE_OPTS` (HIGH) which could cause cross-call state pollution if the WASM FFI boundary mutates its input. The aggregate size race condition (MEDIUM) is bounded by `maxModules` but makes the size limit advisory. The deferred circuit-breaker test from Cycle 2 remains unresolved.

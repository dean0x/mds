# Reliability Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-24
**PR**: #29

## Issues in Your Changes (BLOCKING)

### HIGH

**initWasmBrowser() has no retry exhaustion guard -- unbounded retry on persistent failure** - `packages/mds/src/backend/wasm.ts:206-215`
**Confidence**: 92%
- Problem: `initWasmBrowser()` clears `cachedBrowserPromise` on every failure and allows unlimited retries. Unlike `initWasmNode()` which has `nodeFailures` counting and `MAX_INIT_RETRIES=3` circuit breaker, the browser path has no failure counter. If a browser environment persistently fails to load the WASM module (wrong URL, CSP policy, missing bundle), every call to `init()` in `browser.ts` will re-attempt the import indefinitely. While each individual call terminates (it is not an unbounded loop), the pattern allows an application to unknowingly hammer a failing resource path with no backoff and no permanent rejection -- the system never "gives up."
- Fix: Add a `browserFailures` counter and circuit breaker matching the Node.js pattern:
```typescript
let browserFailures = 0;
const MAX_BROWSER_RETRIES = 3;

export async function initWasmBrowser(options?: InitOptions): Promise<WasmModule> {
  if (cachedBrowserPromise !== null) {
    return cachedBrowserPromise;
  }
  if (browserFailures >= MAX_BROWSER_RETRIES) {
    throw new Error(
      `@mds/mds: WASM browser backend failed to initialize after ${MAX_BROWSER_RETRIES} attempts.`,
    );
  }
  cachedBrowserPromise = _initBrowser(options).catch((err) => {
    browserFailures += 1;
    cachedBrowserPromise = null;
    throw err;
  });
  return cachedBrowserPromise;
}
```
Also reset `browserFailures` in `_resetForTesting()`.

### MEDIUM

**module-scanner: aggregateSize check occurs after content is already read into memory** - `packages/mds/src/util/module-scanner.ts:224-236`
**Confidence**: 85%
- Problem: The TOCTOU fix changed `statAndValidateModule` to `openAndValidateModule`, which now reads the entire file content before returning. The `aggregateSize` check at line 231-236 happens after `openAndValidateModule` returns, meaning the file bytes are already loaded into memory. The pre-existing comment at line 226-230 says "pre-reserve file size ... before reading content" but that invariant no longer holds -- the content is read inside `openAndValidateModule`. For a single oversized file, memory is consumed before the guard fires. The check still prevents cumulative overshoot across multiple files, but each individual file's content is in memory before the size limit rejects it.
- Fix: Split `openAndValidateModule` into validation (open + stat + realpath check) and reading (readFile), so the size reservation happens between them:
```typescript
async function openAndValidateModule(absolutePath: string): Promise<{ handle: FileHandle; size: number }> {
  // ... open, stat, realpath checks ...
  return { handle, size: stats.size };
}

// In scan():
const { handle, size: fileSize } = await openAndValidateModule(absolutePath);
aggregateSize += fileSize;
if (aggregateSize > maxAggregateSize) {
  await handle.close();
  throw new Error(...);
}
const content = await handle.readFile({ encoding: 'utf-8' });
await handle.close();
```
Alternatively, accept the current design with an updated comment acknowledging that individual file content is read before the aggregate check, and that the 10 MiB default limit is low enough that a single file overshoot is acceptable.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Comment drift in module-scanner aggregateSize logic** - `packages/mds/src/util/module-scanner.ts:226-230` (Confidence: 78%) -- The comment says "pre-reserve file size ... before reading content" but content is now read inside `openAndValidateModule()` before the size check runs. The comment should be updated to reflect the actual ordering.

- **browser.ts getBackend() returns "wasm" unconditionally without assertInitialized()** - `packages/mds/src/browser.ts:89-91` (Confidence: 65%) -- `getBackend()` always returns `'wasm'` without calling `assertInitialized()`. This is intentionally correct for browser (backend is always WASM), but diverges from node.ts where `getBackend()` requires initialization. A consumer porting code between environments may rely on getBackend() as a readiness check. Consider documenting this intentional divergence.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The refactor is well-structured with strong reliability patterns throughout: bounded iteration in module-scanner (MAX_IMPORT_DEPTH=64, MAX_PATH_SEGMENTS=256, DEFAULT_MAX_MODULES=256, DEFAULT_MAX_AGGREGATE_SIZE=10MiB), promise deduplication preventing double-init races, circuit breaker with MAX_INIT_RETRIES=3 on Node.js init, TOCTOU fix using O_NOFOLLOW, and proper resource cleanup via try/finally on file handles. The main reliability gap is the missing circuit breaker on the browser init path, which should be straightforward to add for consistency with the Node.js path.

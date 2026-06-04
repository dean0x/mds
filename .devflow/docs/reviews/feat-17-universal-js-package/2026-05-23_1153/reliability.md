# Reliability Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Cycle**: 4 (prior cycle fixed 19/21 issues; 1 FP aggregateSize atomicity; 1 deferred node.ts/browser.ts LSP tension)

## Issues in Your Changes (BLOCKING)

### HIGH

**browser.ts init() permanently caches rejected promise, defeating wasm.ts retry logic** - `packages/mds/src/browser.ts:43-47`
**Confidence**: 95%
- Problem: The cycle-3 fix "remove browser retry reset" removed the `.catch()` handler that cleared `initVoidPromise` on failure. The intent was to delegate retry logic to wasm.ts. However, browser.ts caches the promise from `createWasmBackend()` in `initVoidPromise` (line 44). On rejection, `initVoidPromise` remains non-null, so all subsequent calls to `browser.init()` hit `if (initVoidPromise !== null) return initVoidPromise` (line 43) and return the stale rejected promise. wasm.ts's `init()` resets its own `initPromise` to null on failure (wasm.ts:69), enabling retries -- but browser.ts never calls it again. The retry mechanism in wasm.ts is unreachable from browser environments.
- Impact: A single transient WASM load failure (network hiccup, CDN timeout) permanently breaks the browser entry point for the lifetime of the page. The user cannot recover without a full page reload, even though wasm.ts was designed to allow up to 3 retries.
- Fix: Re-introduce the `.catch()` handler that clears `initVoidPromise`, or forward retry semantics:
```typescript
export function init(options?: InitOptions): Promise<void> {
  if (resolvedBackend !== undefined) return Promise.resolve();
  if (initVoidPromise !== null) return initVoidPromise;
  initVoidPromise = createWasmBackend(options)
    .then((b) => {
      resolvedBackend = b;
    })
    .catch((err) => {
      // Clear so subsequent calls re-enter wasm.ts's retry logic.
      // wasm.ts's MAX_INIT_RETRIES enforces the permanent failure bound.
      initVoidPromise = null;
      throw err;
    });
  return initVoidPromise;
}
```

**tryLoadCandidate swallows all errors including initialization failures** - `packages/mds/src/backend/wasm.ts:86-95`
**Confidence**: 90%
- Problem: The `tryLoadCandidate` function catches all exceptions and returns null. This is appropriate for "module not found" errors (the candidate path does not exist), but it also silently swallows errors from `mod.default(wasmUrl)` (line 90) -- the WASM initialization call. If the module is found but fails to initialize (corrupt WASM, invalid wasmUrl, out-of-memory during instantiation), the error is discarded. The caller then moves to the next candidate or throws a generic "failed to load WASM module. Build it first" message (line 120), which is misleading when the module exists but cannot initialize.
- Impact: Users debugging WASM initialization failures get a misleading error message ("Build it first") when the real cause is a runtime initialization error. The original error context (previously preserved as `loadError` in the pre-refactor code) is lost.
- Fix: Distinguish "module not found" from "module found but init failed":
```typescript
async function tryLoadCandidate(
  candidate: string,
  require: NodeRequire,
  wasmUrl: InitOptions['wasmUrl'],
): Promise<WasmModule | null> {
  let mod: WasmModule;
  try {
    mod = require(candidate) as WasmModule;
  } catch {
    return null; // candidate not found -- try next
  }
  // Module found -- initialization errors must propagate.
  if (typeof mod.default === 'function') {
    await mod.default(wasmUrl);
  }
  return mod;
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **statAndValidateModule has a TOCTOU window between stat and readFile** - `packages/mds/src/util/module-scanner.ts:138-208` (Confidence: 65%) -- The lstat/realpath checks at lines 139-168 validate the path, but actual file content is read later at line 208. An attacker with write access could swap the file between validation and read. This is an inherent limitation acknowledged by the existing TOCTOU comment (line 150); fully closing it would require O_NOFOLLOW open, which Node.js fs/promises does not directly expose.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The two HIGH findings are both regressions introduced by cycle-3 refactoring:
1. The browser retry path is dead -- a single transient failure permanently breaks init().
2. Error context from WASM initialization failures is silently discarded, producing misleading diagnostics.

Both are straightforward to fix with the suggested code changes. The module-scanner's bounded iteration, depth limits, aggregate size limits, and deep-frozen defaults are well-implemented. The `_resetForTesting` helper and circuit-breaker tests (U-WB1, U-WB2) demonstrate good reliability testing coverage.

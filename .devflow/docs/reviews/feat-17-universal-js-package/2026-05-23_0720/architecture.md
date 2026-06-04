# Architecture Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicate init-state machines in browser.ts and wasm.ts — redundant layering** - `packages/mds/src/browser.ts:24-50`, `packages/mds/src/backend/wasm.ts:25-57`
**Confidence**: 85%
- Problem: Both `browser.ts` and `wasm.ts` independently maintain an init promise cache, a resolved-state guard, and retry-on-failure logic. `browser.ts` has `resolvedBackend` + `initVoidPromise` with its own `.catch()` that clears the cached promise. `wasm.ts` has `wasmModule` + `initPromise` + `initFailures` with its own `.catch()` that clears the cached promise and increments a failure counter. When `browser.ts` calls `createWasmBackend(options)`, which internally calls `init(options)`, both layers perform overlapping idempotency checks and promise caching. The `browser.ts` catch handler resets `initVoidPromise = null` to allow retries, but `wasm.ts` independently tracks `initFailures` and may refuse to retry if its counter has reached `MAX_INIT_RETRIES` — a state that `browser.ts` has no visibility into. This dual state machine creates a confusing ownership boundary: who owns the retry policy? The comment in `browser.ts:26-27` says "wasm.ts's MAX_INIT_RETRIES enforces a permanent failure bound," but `browser.ts` resets its own promise to null on failure (line 47), implying it expects retries to work — which they will until wasm.ts's counter silently blocks them. The layers should have a single source of truth for init lifecycle.
- Fix: Since `createWasmBackend` already calls `init()` internally and returns a fully-configured `MdsBackend`, `browser.ts` should treat it as a black box. Remove the duplicate retry/caching logic in `browser.ts` and let `wasm.ts` own all init lifecycle. `browser.ts` should cache only the resolved `MdsBackend` and the in-flight promise, without adding its own retry semantics:
  ```typescript
  export function init(options?: InitOptions): Promise<void> {
    if (resolvedBackend !== undefined) return Promise.resolve();
    if (initVoidPromise !== null) return initVoidPromise;
    initVoidPromise = createWasmBackend(options).then((b) => {
      resolvedBackend = b;
    });
    // Do NOT add a .catch that resets initVoidPromise — wasm.ts owns retry policy.
    // If wasm init permanently fails, subsequent calls get the same rejected promise.
    return initVoidPromise;
  }
  ```
  Alternatively, if browser.ts needs its own retry-on-failure (e.g., because the wasm fetch URL might become available later), remove the retry logic from wasm.ts and let browser.ts own it entirely. One layer, not both.

**Shared mutable DEFAULT_COMPILE_OPTS modules object** - `packages/mds/src/backend/wasm.ts:106`
**Confidence**: 82%
- Problem: `DEFAULT_COMPILE_OPTS` is `Object.freeze({ filename: 'input.mds', modules: {} as Record<string, string> })`. `Object.freeze` is shallow — it freezes the top-level properties but the nested `modules: {}` object remains mutable. If the WASM binding's `compile()` or `check()` implementation mutates the `modules` object it receives (e.g., adds entries during resolution), the shared default object would be corrupted for all subsequent calls. The code currently passes `DEFAULT_COMPILE_OPTS` directly (line 117: `? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS`) — when no vars are provided, the exact same object reference is passed to every call.
- Fix: Either deep-freeze the nested object or always spread a fresh copy:
  ```typescript
  const DEFAULT_COMPILE_OPTS = Object.freeze({
    filename: 'input.mds',
    modules: Object.freeze({} as Record<string, string>),
  });
  ```
  Deep-freezing the nested `modules` object ensures that any attempt to mutate it throws in strict mode, making the contract explicit. Alternatively, always spread: `{ filename: 'input.mds', modules: {}, ...vars }` — but this loses the allocation-avoidance benefit the comment describes.

### MEDIUM

**node.ts and browser.ts export divergent public APIs for the same package** - `packages/mds/src/node.ts:47-70`, `packages/mds/src/browser.ts:64-107`
**Confidence**: 83%
- Problem: Both entry points export the same function names (`compile`, `check`, `compileFile`, `checkFile`, `getBackend`, `init`) but with significantly different runtime behavior. `compileFile`/`checkFile` throw unconditionally in the browser entry but work normally in the Node entry. `init()` is meaningful in browser but essentially a no-op in Node (re-exported from wasm.ts where it may already be initialized). The `MdsBackend` interface requires `compileFile` and `checkFile`, but the browser adapter never implements them meaningfully — it returns rejected promises. This is an LSP (Liskov Substitution Principle) tension: the browser entry cannot substitute for the node entry despite sharing the same interface. While this is an inherent platform constraint, the architecture does not make this explicit at the type level.
- Fix: Consider splitting `MdsBackend` into a base interface (`MdsCompiler`) with `compile`/`check`, and an extended interface (`MdsFileCompiler extends MdsCompiler`) adding `compileFile`/`checkFile`. The browser entry would implement `MdsCompiler`; the node entry would implement `MdsFileCompiler`. This makes the capability difference visible at compile time rather than surfacing as runtime errors. For the current PR scope, this is not blocking — the approach works and the error messages are clear — but it is worth tracking as architectural debt.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**wasm.ts init() uses module-scoped mutable singletons without reset capability** - `packages/mds/src/backend/wasm.ts:25-29`
**Confidence**: 80%
- Problem: `wasmModule`, `initPromise`, `initFailures` are module-scoped `let` variables. There is no `reset()` or `dispose()` function. This makes integration testing difficult — once `initFailures` reaches `MAX_INIT_RETRIES`, the module is permanently poisoned for the process lifetime. The browser test (`browser.spec.mjs`) works around this by relying on `describe` ordering to call `init()` only once, but any test that needs to verify retry exhaustion would permanently break subsequent tests in the same process.
- Fix: Export a `_resetForTesting()` function (prefixed with underscore to signal internal use) that clears all three singletons. This is a standard pattern for testable singletons:
  ```typescript
  /** @internal — test-only reset. Not part of the public API. */
  export function _resetForTesting(): void {
    wasmModule = undefined;
    initPromise = null;
    initFailures = 0;
  }
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**node.ts uses top-level await with side-effectful import-time initialization** - `packages/mds/src/node.ts:10-45`
**Confidence**: 85%
- Problem: The entire backend selection and initialization (including `console.warn`, dynamic `import()`, and `require()` calls) happens at module load time via top-level await. This means importing the package triggers file I/O and potentially network requests. The consumer has no control over when initialization happens and cannot handle errors gracefully before the module is loaded. This is an intentional design choice (noted in prior resolution as "intentional design"), but architecturally it couples import-time with initialization-time, violating separation of concerns.
- Note: Prior resolutions confirmed this is intentional. Documenting for awareness only.

## Suggestions (Lower Confidence)

- **Consider extracting WASM loading candidates to configuration** - `packages/mds/src/backend/wasm.ts:67-72` (Confidence: 65%) -- The hardcoded candidate paths array couples the WASM adapter to specific filesystem layouts (workspace vs npm install). If a third layout emerges, this function must be modified (OCP tension).

- **CompileOptions and FileOptions are structurally identical** - `packages/mds/src/types.ts:17-27` (Confidence: 70%) -- Both interfaces have only `vars?: Record<string, unknown>`. If they are intended to diverge in the future, the separation is justified forward-looking design. If not, a single `Options` type would reduce surface area.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The overall architecture is well-structured: clean Strategy pattern via `MdsBackend`, proper dependency injection in the native backend, good separation between entry points via the exports map, and thorough resource-bounding in the module scanner. The depth-limit addition (MAX_IMPORT_DEPTH) and parallel I/O hardening from prior review cycles are solid improvements.

The primary concern is the duplicate init-state machine spanning browser.ts and wasm.ts, which creates ambiguous ownership of the retry policy and risks subtle state-desynchronization bugs. The shallow freeze on DEFAULT_COMPILE_OPTS is a correctness risk if the WASM binding mutates the options it receives. Both are addressable without structural rework.

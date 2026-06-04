# TypeScript Review Report

**Branch**: refactor-27-28-unified-backend-architecture -> main
**Date**: 2026-05-24

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing shape validation in `_initBrowser` -- `compile`/`check`/`scanImports` not validated** - `packages/mds/src/backend/wasm.ts:232`
**Confidence**: 90%
- Problem: `_initBrowser()` casts `await import('mds-wasm')` directly to `WasmModule` without validating that `compile`, `check`, and `scanImports` are functions. In contrast, `tryLoadCandidate()` (used by the Node.js path) explicitly validates all three exports at lines 91-94. The browser path only validates `default` (line 241). If the bundled module is corrupt, outdated, or a different version, the code will silently cast it and produce runtime `TypeError`s deep in user calls instead of a clear initialization error.
- Fix: Add the same shape validation used in `tryLoadCandidate`:
  ```typescript
  // After line 233: wasmMod = imported;
  if (
    typeof wasmMod.compile !== 'function' ||
    typeof wasmMod.check !== 'function' ||
    typeof wasmMod.scanImports !== 'function'
  ) {
    throw new Error(
      '@mds/mds: WASM module missing required exports (compile, check, scanImports). ' +
      'Ensure the correct version of mds-wasm is bundled.',
    );
  }
  ```

### MEDIUM

**`browser.ts` `getBackend()` bypasses `assertInitialized()` -- returns `'wasm'` even before `init()`** - `packages/mds/src/browser.ts:89-91`
**Confidence**: 82%
- Problem: In `node.ts`, `getBackend()` calls `assertReady()` and throws if `init()` has not been called (line 199-200). In `browser.ts`, `getBackend()` hardcodes `return 'wasm'` and never calls `assertInitialized()`. This creates an inconsistency: `browser.ts` will happily return `'wasm'` before `init()` has been called, while `node.ts` throws. This is an intentional design choice (the browser always uses WASM), but it diverges from the init-gate pattern established by the other exports and could mislead callers into thinking initialization succeeded.
- Fix: If the intent is to enforce the init-gate consistently, use `assertInitialized()`:
  ```typescript
  export function getBackend(): BackendType {
    assertInitialized();
    return 'wasm';
  }
  ```
  Alternatively, if the current behavior is intentional (since the answer is always `'wasm'`), add a JSDoc note explaining why this does not require init, for consistency documentation.

**`node.ts` does not re-export `MdsNodeBackend` or `MdsBaseBackend` types** - `packages/mds/src/node.ts:204-213`
**Confidence**: 80%
- Problem: `index.ts` exports `MdsBackend`, `MdsBaseBackend`, and `MdsNodeBackend` (lines 10-12), but `node.ts` only re-exports a limited subset (lines 204-213) -- omitting the backend interface types entirely. Consumers importing from the `node` entry point cannot type-annotate variables as `MdsNodeBackend` without importing from `types.js` or the barrel `index.ts`. Since `node.ts` is the primary entry point for Node.js users, these types should be available.
- Fix: Add the backend types to the `node.ts` re-exports:
  ```typescript
  export type {
    BackendType,
    CheckResult,
    CompileOptions,
    CompileResult,
    FileOptions,
    InitOptions,
    MdsBackend,
    MdsBaseBackend,
    MdsError,
    MdsErrorSpan,
    MdsNodeBackend,
  } from './types.js';
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Unused `lstat` import in `module-scanner.ts`** - `packages/mds/src/util/module-scanner.ts:1`
**Confidence**: 95%
- Problem: The import `lstat` from `'node:fs/promises'` is still present but no longer used. The old `statAndValidateModule` called `lstat()` directly, but the new `openAndValidateModule` uses `handle.stat()` (fstat on the fd) and `realpath()`. The `lstat` import is dead code.
- Fix: Remove `lstat` from the import:
  ```typescript
  import { open, realpath } from 'node:fs/promises';
  ```

**CSP error detection uses overly broad `'fetch'` substring match** - `packages/mds/src/backend/wasm.ts:258`
**Confidence**: 80%
- Problem: The CSP detection logic at line 258 checks `msg.includes('fetch')`, which would match any error whose message incidentally contains the word "fetch" -- for example, `"failed to fetch configuration"`, `"prefetch error"`, or any error from a library that happens to use the word. This could cause unrelated errors to be mis-diagnosed as CSP violations, producing a misleading error message that tells users to modify their Content Security Policy.
- Fix: Use a more specific match, such as checking for `'Failed to fetch'` (the exact wording from browser fetch failures) or combining the `fetch` check with a WASM-specific context:
  ```typescript
  if (
    msg.includes('Content Security Policy') ||
    msg.includes('unsafe-eval') ||
    msg.includes('wasm-unsafe-eval') ||
    msg.includes('CompileError') ||
    (msg.includes('fetch') && msg.includes('wasm'))
  ) {
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`initWasmBrowser` has no retry exhaustion limit** - `packages/mds/src/backend/wasm.ts:206-215` (Confidence: 65%) -- Unlike `initWasmNode` which tracks `nodeFailures` and stops after `MAX_INIT_RETRIES`, `initWasmBrowser` has unlimited retries. The comment at line 203-204 says "simpler than Node.js -- no candidate list, so exhaustion means the wasmUrl itself is wrong," but this means a misconfigured browser app will retry indefinitely on every user action that triggers init, potentially flooding the console with errors.

- **`browser.ts` `_initWithModuleForTesting` not guarded by `_resetForTesting` in same call** - `packages/mds/src/browser.ts:44-47` (Confidence: 62%) -- `_initWithModuleForTesting` sets `initVoidPromise = null` but does not call `wasmReset()`. If a test calls `_initWithModuleForTesting` while a previous `initWasmBrowser` promise is in flight (in `cachedBrowserPromise`), the WASM-level state and browser-level state could diverge. This is test-only code, but the coupling could cause test flakiness.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

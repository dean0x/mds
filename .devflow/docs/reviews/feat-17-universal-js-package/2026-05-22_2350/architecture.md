# Architecture Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22T23:50

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicated init race-prevention logic between browser.ts and wasm.ts** - `packages/mds/src/browser.ts:24-51`, `packages/mds/src/backend/wasm.ts:28-60`
**Confidence**: 85%
- Problem: `browser.ts` maintains its own `initPromise` + `backend` singleton state alongside `wasm.ts`'s separate `initPromise` + `initFailures` + `wasmModule` singleton. Both modules independently implement the "cache promise, clear on failure, retry" pattern. This is a layering violation (DIP): `browser.ts` calls `wasmInit(options)` (the wasm.ts `init`), then separately calls `createWasmBackend()` which calls `init()` again internally. The double-init is safe because of idempotency, but the two layers each maintain their own retry/race state, which is fragile. If `wasm.ts` hits `MAX_INIT_RETRIES`, it will throw permanently, but `browser.ts` would still reset its own `initPromise = null` and attempt to retry -- creating a confusing failure mode where `browser.ts` retries but `wasm.ts` refuses.
- Fix: `browser.ts` should delegate entirely to `createWasmBackend()` for init lifecycle, storing only the returned `MdsBackend`. The retry/race logic belongs in one place (wasm.ts). The browser entry's `doInit` could simplify to:
  ```typescript
  async function doInit(options?: InitOptions): Promise<void> {
    const { createWasmBackend } = await import('./backend/wasm.js');
    backend = await createWasmBackend(options);
  }
  ```
  This requires `createWasmBackend` to accept and forward `InitOptions`, but eliminates the duplicate state machines.

### MEDIUM

**Module-level side effects in node.ts make the module untestable and non-lazy** - `packages/mds/src/node.ts:10-45`
**Confidence**: 82%
- Problem: Backend selection runs as top-level `await` at import time (lines 19-44). This means importing the module triggers I/O (loading the napi addon or WASM module), console.warn calls, and environment variable reads. This violates the "explicit over implicit" principle -- consumers cannot import the module without triggering side effects. It also makes unit testing the fallback logic impossible without subprocess spawning (as the U-B5 test demonstrates).
- Fix: Consider a lazy initialization pattern where the backend is resolved on first use:
  ```typescript
  let backendPromise: Promise<MdsBackend> | null = null;
  function getOrInitBackend(): Promise<MdsBackend> {
    if (!backendPromise) backendPromise = resolveBackend();
    return backendPromise;
  }
  ```
  This is a larger refactor and may conflict with the current synchronous API surface (`compile` returns `CompileResult`, not `Promise<CompileResult>`). If synchronous compile is a hard requirement, the current approach is an acceptable trade-off, but document the rationale.

**Stale test:parity script reference in package.json** - `packages/mds/package.json:27`
**Confidence**: 95%
- Problem: `"test:parity": "node --test __test__/parity.spec.mjs"` references the old filename. The file was renamed to `native-backend.spec.mjs` in this PR. Running `npm run test:parity` will fail with file-not-found.
- Fix: Update to `"test:parity": "node --test __test__/native-backend.spec.mjs"` or rename the script key to `test:native`.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**CompileOptions and FileOptions are identical interfaces** - `packages/mds/src/types.ts:17-27`
**Confidence**: 85%
- Problem: `CompileOptions` and `FileOptions` have the exact same shape (`{ vars?: Record<string, unknown> }`). Having two identical interfaces adds surface area without benefit -- ISP is about splitting large interfaces, not duplicating small ones. The `varsOpt` helper already unions them (`CompileOptions | FileOptions`), confirming they're interchangeable. If they diverge in the future, a union type or extension would be cleaner than pre-emptive duplication.
- Fix: Use a single `MdsOptions` type, or have `FileOptions extends CompileOptions` if file-specific options are anticipated. This is low-urgency since both are small, but the duplication is unnecessary complexity.

**getBackend() in browser.ts is hardcoded, not delegated** - `packages/mds/src/browser.ts:68-70`
**Confidence**: 80%
- Problem: `browser.ts` hardcodes `return 'wasm'` in `getBackend()` instead of delegating to `backend.getBackend()`. While this is correct today (browser is always WASM), it breaks the Strategy pattern: if the browser ever supports a second backend, this hardcoded value would silently lie. It also means `getBackend()` returns `'wasm'` even before `init()` is called, when there is no active backend.
- Fix: Either delegate to `assertInitialized().getBackend()` for consistency with compile/check, or document that `getBackend()` in browser context always returns `'wasm'` by design.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No root package.json but package-lock.json references workspace config** - `package-lock.json:1-10`
**Confidence**: 80%
- Problem: The lock file references `"name": "mdl"` and workspace paths (`packages/*`, `crates/mds-napi`) but there is no root `package.json` in the repo (or it is gitignored). This is unusual for npm workspaces -- typically the root `package.json` defines the workspace array and is committed. Without it, `npm install` from the repo root may behave unexpectedly for new contributors.
- Fix: Verify the root `package.json` exists and is tracked. If intentionally untracked, document the workspace setup in the README.

## Suggestions (Lower Confidence)

- **`varsOpt` passes through `null` vars without filtering** - `packages/mds/src/util/options.ts:11` (Confidence: 70%) -- `options?.vars !== undefined` is true when `vars: null`, so `{ vars: null }` is forwarded to backends. The test U-C7 exercises this path and expects no crash, but it may be cleaner to normalize `null` to `undefined` at the boundary.

- **Recursive `scan` has no depth limit** - `packages/mds/src/util/module-scanner.ts:135` (Confidence: 65%) -- `buildModulesMap` limits module count and aggregate size but not recursion depth. A pathological import graph with 256 files in a deep chain would hit the module limit, but the call stack could overflow before that. The module count limit likely provides sufficient protection in practice.

- **WASM candidate paths are hardcoded** - `packages/mds/src/backend/wasm.ts:70-75` (Confidence: 62%) -- The `_init` function tries two hardcoded candidate paths for the WASM module. This is configuration embedded in code rather than externalized. Acceptable for an early-stage package but may need an extension point as deployment scenarios grow.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The overall architecture is well-structured: a clean Strategy pattern via `MdsBackend` interface, proper dependency injection in the native backend adapter, and a sensible conditional-export split between Node and browser entry points. The main concerns are (1) duplicated init/retry state machines between browser.ts and wasm.ts that should be consolidated, and (2) a broken test:parity script reference that will fail. The type system and module boundaries are sound, and the security hardening in the module scanner is thorough.

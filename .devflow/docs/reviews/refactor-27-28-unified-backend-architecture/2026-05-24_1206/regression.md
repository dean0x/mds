# Regression Review Report

**Branch**: PR #29 -> main
**Date**: 2026-05-24T12:06

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Size-limit check now occurs after content is already in memory** - `module-scanner.ts:224-232` (Confidence: 65%) -- The old code separated `statAndValidateModule` (size-only) from `readFile` (content), allowing the aggregate size check to reject before loading content into memory. The new `openAndValidateModule` returns both size and content from the same fd operation, meaning a file exceeding the remaining budget is loaded into memory before the check at line 232 rejects it. This is an intentional trade-off to close the TOCTOU window (as stated in the PR description), and the practical impact is minimal since individual files rarely approach the 10 MiB aggregate limit, but it is a behavioral difference worth documenting.

- **Browser init retry exhaustion test coverage reduced** - `browser.spec.mjs` (Confidence: 60%) -- The old U-BR11 test verified end-to-end that browser `init()` properly clears its cached promise on rejection and retries through `createWasmBackend` -> `wasm.ts` circuit breaker. The new U-BR11 only tests `_resetForTesting()` clears state, and the promise dedup / retry-on-failure path through `initWasmBrowser()` is not exercised in the browser test suite (since `_initWithModuleForTesting` bypasses the real init path). This is understandable given the browser init path cannot run in Node.js, but it represents reduced integration coverage.

- **`initWasmBrowser` lacks circuit breaker / retry exhaustion** - `wasm.ts:206-215` (Confidence: 70%) -- `initWasmNode` has a `nodeFailures` counter with `MAX_INIT_RETRIES` (3), providing permanent failure after exhaustion. `initWasmBrowser` has no equivalent -- every failed call clears `cachedBrowserPromise` and retries indefinitely. The comment says "simpler than Node.js -- no candidate list, so exhaustion means the wasmUrl itself is wrong", which is a reasonable design choice, but it differs from the Node.js path's behavior. If a browser environment has transient init failures (e.g., CSP policy issues that get resolved), unlimited retries are desirable; if the wasmUrl is permanently wrong, unlimited retries are wasteful. Not a regression (the old browser path also had unlimited retries via `wasm.ts init()` -> `createWasmBackend()`), but a behavioral asymmetry.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

## Regression Analysis

### Breaking Changes (Intentional, Documented)

All breaking changes are intentional and acceptable for a pre-release project with zero users:

1. **`init()` now required before `compile`/`check`/`compileFile`/`checkFile`/`getBackend` in Node.js** -- Previously, `node.ts` used top-level await (TLA) to auto-initialize the backend at import time. Now, consumers must call `await init()` explicitly. Clear error messages guide consumers: `@mds/mds: call await init() before using compile/check/compileFile/checkFile/getBackend`. Tests validate this contract (U-B6, U-B7, U-B10, U-B11).

2. **`compileFile`/`checkFile` removed from browser entry** -- Previously exported as always-throwing stubs. Now removed entirely, which is cleaner -- browser consumers who called these would always get an error anyway. Tests U-BR12 and U-BR13 assert their absence.

3. **`FileOptions` type removed from browser entry exports** -- Consistent with removal of file operations from browser entry. Available from `node.ts` and `index.ts` barrel.

4. **`MdsBackend` -> `MdsNodeBackend` (with `MdsBackend` as deprecated alias)** -- `MdsBackend` type alias preserved for backward compatibility: `export type MdsBackend = MdsNodeBackend`. New `MdsBaseBackend` interface added for browser-safe subset.

5. **`createWasmBackend` signature changed** -- Old: `async (options?: InitOptions) => Promise<MdsBackend>` (called `init()` internally). New: `(wasmModule: WasmModule) => MdsBaseBackend` (synchronous, requires pre-initialized module). This is an internal/advanced API.

6. **`init` from `wasm.ts` split into `initWasmNode` and `initWasmBrowser`** -- The old single `init()` export from `wasm.ts` (re-exported by `node.ts`) is replaced by environment-specific init functions.

### Migration Completeness

- All test files updated to call `init()` before backend operations
- All imports updated from old API to new API
- No stale references to old `init` from `wasm.ts`
- No stale references to old `createWasmBackend(options?)` async signature
- `node.ts` re-exports verified: same type set, alphabetically reordered
- `index.ts` barrel exports extended with new types (`MdsBaseBackend`, `MdsNodeBackend`, `WasmModule`)

### Behavioral Preservation

- `compile()`, `check()`, `compileFile()`, `checkFile()`, `getBackend()` function signatures unchanged
- Return types unchanged
- Error types unchanged (`MdsError`, `isMdsError`)
- Backend selection logic preserved (native preferred, WASM fallback, `MDS_BACKEND` env var)
- Circuit breaker behavior preserved for Node.js WASM init
- Promise deduplication preserved for concurrent `init()` calls
- All 95 tests pass

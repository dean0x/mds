# Consistency Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22T13:49

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent type export order between node.ts and browser.ts** - `packages/mds/src/node.ts:76-84`, `packages/mds/src/browser.ts:13-21`
**Confidence**: 82%
- Problem: The type exports in `node.ts` list types in a different order than `browser.ts` (and both differ from `index.ts`). Specifically, `node.ts` lists `MdsError` before `MdsErrorSpan`, while `browser.ts` lists `MdsErrorSpan` before `MdsError` (matching `index.ts`). Furthermore, `node.ts` exports `InitOptions` last but omits `MdsBackend`, while `index.ts` exports `MdsBackend`.
- Fix: Standardize the export order across all entry points. Use the order from `types.ts` as the canonical ordering. The `MdsBackend` type is intentionally omitted from consumer-facing entry points (it is an internal interface), which is acceptable. However, align the type ordering:
  ```typescript
  // Both node.ts and browser.ts should use:
  export type {
    CompileResult,
    CheckResult,
    CompileOptions,
    FileOptions,
    MdsErrorSpan,
    MdsError,
    BackendType,
    InitOptions,
  } from './types.js';
  ```

### MEDIUM

**Inconsistent variable naming: `_backend` vs `backend`, `_initPromise` vs `initPromise`** - `packages/mds/src/browser.ts:24-27`, `packages/mds/src/backend/wasm.ts:27-29`
**Confidence**: 85%
- Problem: `browser.ts` uses underscore-prefixed names for module-level state (`_backend`, `_initPromise`, `_doInit`) while `wasm.ts` uses non-prefixed names for the equivalent pattern (`wasmModule`, `initPromise`, `_init`). The convention within the same package is inconsistent -- one prefixes with `_` to signal "private state", the other does not. Also mixed: `_init` vs `_doInit` for the same conceptual function (the inner init implementation).
- Fix: Choose one convention and apply it uniformly. Since TypeScript module-level variables are already private-by-encapsulation (not exported), dropping the underscore prefix would be idiomatic. If keeping the prefix, apply it consistently in both files:
  ```typescript
  // browser.ts: uses _backend, _initPromise, _doInit (prefixed)
  // wasm.ts:    uses wasmModule, initPromise, _init (mixed)
  // Pick one — recommend no prefix since these are not exported:
  // wasm.ts: wasmModule, initPromise, doInit
  // browser.ts: backend, initPromise, doInit
  ```

**Inconsistent `init()` failure-recovery pattern** - `packages/mds/src/browser.ts:33-38`, `packages/mds/src/backend/wasm.ts:40-50`
**Confidence**: 83%
- Problem: Both `browser.ts` and `wasm.ts` implement the same "idempotent singleton init with retry on failure" pattern, but they do it differently:
  - `wasm.ts` caches the promise synchronously, then uses `.catch()` to reset on failure
  - `browser.ts` checks `_backend !== undefined` first (short-circuit), then caches synchronously, but resets inside `_doInit`'s try/catch
  These are two distinct implementations of the same concept. The `browser.ts` approach has a subtle correctness advantage (it also checks if `_backend` is already set), but the structural divergence makes the code harder to maintain.
- Fix: Extract a shared `createSingletonInit` utility or at minimum align the patterns so both use the same structure (either both use `.catch()` on the outer promise, or both use try/catch in the inner function).

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`CompileOptions` and `FileOptions` have identical shapes** - `packages/mds/src/types.ts:11-17`
**Confidence**: 80%
- Problem: `CompileOptions` and `FileOptions` are defined identically (both have only `vars?: Record<string, unknown>`). While they exist as separate types for future extensibility, the current duplication may signal an intent to add `basePath` to `CompileOptions` (which the napi addon does accept) but was not exposed. The native backend's `NapiAddon` interface shows `compile` accepts `{ basePath?, vars? }` but the public `CompileOptions` only exposes `vars`. This is a potential incomplete API surface.
- Fix: If `basePath` is intentionally hidden from the public API, add a comment documenting why. If it should be exposed, add it to `CompileOptions` only (not `FileOptions`, since file operations infer basePath from the file's directory, matching the napi crate pattern).

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`napi/index.js` uses CommonJS while the rest of the package is ESM** - `crates/mds-napi/index.js:1` (Confidence: 70%) -- The napi platform loader uses `'use strict'` + `require()` (CJS) which is standard for native addon loading. This is likely intentional for maximum compatibility with how `require()` resolves `.node` files, but it creates a style asymmetry with the ESM package. Consider a brief inline comment explaining why CJS is required here.

- **Tests redefine `__dirname` and `FIXTURES` independently** - `packages/mds/__test__/parity.spec.mjs:15-18`, `packages/mds/__test__/perf.spec.mjs:14-16` (Confidence: 72%) -- Several test files redefine `__dirname`, `FIXTURES`, and `SIMPLE_MDS` locally rather than importing from `helpers.mjs`. The `helpers.mjs` file already exports these. This leads to duplicated path resolution logic that could drift out of sync.

- **`browser.ts` imports `MdsBackend` type but never needs it in the type export list** - `packages/mds/src/browser.ts:8` (Confidence: 65%) -- `MdsBackend` is imported for use as a local type annotation but is not re-exported to consumers. This is fine but the import list includes `InitOptions` which is re-exported and `MdsBackend` which is not. A comment or grouping would clarify intent.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The package demonstrates strong internal consistency overall: both backends implement the same `MdsBackend` interface faithfully, the TypeScript types correctly mirror the napi/wasm result shapes (`CompileResult`, `CheckResult`), naming conventions across the API surface are uniformly camelCase matching the napi crate's `js_name` annotations, and the test files follow a consistent structure. The blocking issues are minor naming/ordering inconsistencies that should be addressed for long-term maintainability but do not affect correctness.

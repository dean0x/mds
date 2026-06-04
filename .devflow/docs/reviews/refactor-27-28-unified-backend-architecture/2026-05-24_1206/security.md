# Security Review Report

**Branch**: refactor-27-28-unified-backend-architecture -> main
**Date**: 2026-05-24T12:06
**PR**: #29

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Browser WASM init missing shape validation** - `packages/mds/src/backend/wasm.ts:228-233`
**Confidence**: 90%
- Problem: `_initBrowser()` casts the dynamically imported module directly as `WasmModule` without performing the same shape validation (checking for `compile`, `check`, `scanImports` functions) that `tryLoadCandidate()` applies for the Node.js path. A malicious or corrupted bundled module could provide arbitrary exports that bypass type safety at runtime. The Node.js path validates the shape at lines 91-95, but the browser path trusts the cast on line 232 without verification.
- Impact: In browser environments, a compromised or misconfigured WASM bundle could inject a module with unexpected behavior. The type assertion (`as WasmModule`) provides no runtime safety. This violates the FEATURE_KNOWLEDGE note that the package "validates WASM module shape at boundary."
- Fix:
  ```typescript
  // After line 233 (wasmMod = imported;), add:
  if (
    typeof wasmMod.compile !== 'function' ||
    typeof wasmMod.check !== 'function' ||
    typeof wasmMod.scanImports !== 'function'
  ) {
    throw new Error(
      '@mds/mds: WASM module shape validation failed. ' +
      'Expected compile, check, and scanImports exports.',
    );
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**initWasmBrowser has no retry exhaustion limit** - `packages/mds/src/backend/wasm.ts:206-215`
**Confidence**: 82%
- Problem: Unlike `initWasmNode()` which tracks `nodeFailures` and stops retrying after `MAX_INIT_RETRIES` (3 attempts), `initWasmBrowser()` clears `cachedBrowserPromise` on every failure and allows unlimited retries. While the comment on line 203-204 states "simpler than Node.js -- no candidate list, so exhaustion means the wasmUrl itself is wrong," an unbounded retry loop is still a concern. A caller in a retry loop could repeatedly hit a failing WASM endpoint, causing excessive network requests or CPU cycles in a browser environment.
- Impact: Potential for denial-of-self in browser environments if init is called in an automated retry pattern. This is more of a reliability concern than a direct exploit, but it deviates from the hardening pattern established by the Node.js path.
- Fix: Add a `browserFailures` counter mirroring `nodeFailures`, or document the intentional asymmetry with a comment explaining why browser retries are unbounded.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`_initWithModuleForTesting` bypasses all validation** - `packages/mds/src/browser.ts:44-46` (Confidence: 65%) -- This test-only export injects a WasmModule directly into `resolvedBackend` without shape validation. While marked `@internal` and clearly labeled for testing only, if it were accidentally imported in production code, it would circumvent boundary validation. Consider adding a `process.env.NODE_ENV` guard or shape check.

- **`_resetForTesting` exports are available at runtime** - `packages/mds/src/node.ts:41-44`, `packages/mds/src/browser.ts:31-34`, `packages/mds/src/backend/wasm.ts:56-60` (Confidence: 62%) -- Three modules export `_resetForTesting()` which can clear all singleton state (backend, init promises, failure counters). While the underscore prefix and `@internal` tag signal intent, these are fully exported and callable by any consumer. In a shared-process environment (e.g., SSR), a dependency calling `_resetForTesting()` could silently destroy backend state.

- **`WasmModule` type exported publicly from index.ts** - `packages/mds/src/index.ts:15` (Confidence: 60%) -- Exporting the `WasmModule` interface and `initWasmNode`/`initWasmBrowser`/`createWasmBackend` from the package's public index gives consumers direct access to low-level initialization functions. While useful for advanced use cases, it increases the API attack surface.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The refactoring significantly improves security posture in the module-scanner by replacing the old lstat-then-readFile TOCTOU-vulnerable pattern with O_NOFOLLOW-based atomic open-validate-read. The split of WASM init into Node/Browser paths correctly isolates node:fs and node:module from browser bundles.

The single blocking finding is the missing shape validation in `_initBrowser()` -- the Node.js path validates the WASM module shape at the boundary before trusting it, but the browser path casts without verification. This is a boundary validation gap that should be addressed before merge to maintain the security invariant documented in FEATURE_KNOWLEDGE ("validates WASM module shape at boundary").

# Consistency Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-24
**PR**: #29

## Issues in Your Changes (BLOCKING)

### HIGH

**Assertion guard function naming inconsistency** - `node.ts:169`, `browser.ts:71`
**Confidence**: 92%
- Problem: `node.ts` uses `assertReady()` while `browser.ts` uses `assertInitialized()`. Both functions serve the identical purpose (throw if backend is not initialized, return the backend). The two entry points are parallel modules with the same public API pattern, so internal helpers should use the same naming.
- Fix: Rename both to the same name. Either `assertReady` or `assertInitialized` -- pick one and apply consistently:
```typescript
// browser.ts (if standardizing on assertReady)
function assertReady(): MdsBaseBackend {
  if (resolvedBackend === undefined) {
    throw new Error('@mds/mds: call init() before using compile/check in a browser environment');
  }
  return resolvedBackend;
}
```

---

**JSDoc phrasing inconsistency for init() requirement** - `node.ts:178-198`, `browser.ts:78-83`
**Confidence**: 90%
- Problem: `node.ts` uses `"Requires await init() first."` while `browser.ts` uses `"Requires init() to have been called and awaited first."` for identical semantic. These are user-facing JSDoc strings that appear in IDE tooltips and generated docs -- they should use identical phrasing across entry points.
- Fix: Standardize on one phrasing. The shorter `node.ts` form is preferable:
```typescript
// browser.ts - match node.ts style
/** Compile an MDS source string to Markdown. Requires await init() first. */
export function compile(source: string, options?: CompileOptions): CompileResult {
```

---

**Unused import: `lstat` in module-scanner.ts** - `module-scanner.ts:1`
**Confidence**: 95%
- Problem: `lstat` is imported from `node:fs/promises` but is never called. The refactor replaced `statAndValidateModule` (which called `lstat`) with `openAndValidateModule` (which uses `open` with `O_NOFOLLOW` + `handle.stat()`). The stale import references `lstat` in comments (lines 6, 86, 145) but the function itself is dead code. This contradicts the zero-dead-code principle.
- Fix: Remove `lstat` from the import statement:
```typescript
import { open, realpath } from 'node:fs/promises';
```

### MEDIUM

**Missing shape validation in browser WASM init** - `wasm.ts:223-267`
**Confidence**: 85%
- Problem: `_initBrowser()` does not validate that the loaded WASM module has `compile`, `check`, or `scanImports` functions before returning it as `WasmModule`. The comment on line 227 says "The shape is validated below" but only `default` is checked (line 241). In contrast, `tryLoadCandidate()` (Node.js path, lines 91-94) validates `compile`, `check`, and `scanImports`. A malformed bundler output would cause a confusing runtime error (`TypeError: wasmMod.compile is not a function`) instead of a clear initialization error.
- Fix: Add shape validation after `wasmMod.default()` succeeds, matching `tryLoadCandidate`:
```typescript
// After await wasmMod.default(options?.wasmUrl);
if (
  typeof wasmMod.compile !== 'function' ||
  typeof wasmMod.check !== 'function' ||
  typeof wasmMod.scanImports !== 'function'
) {
  throw new Error(
    '@mds/mds: WASM module missing required exports (compile, check, scanImports). ' +
    'Rebuild with: wasm-pack build crates/mds-wasm --target web --out-dir pkg',
  );
}
```

---

**Test ID numbering gaps across new test suites** - `backend.spec.mjs`, `wasm-backend.spec.mjs`
**Confidence**: 82%
- Problem: Test IDs skip numbers: `backend.spec.mjs` skips U-B9 (goes U-B8 to U-B10); `wasm-backend.spec.mjs` skips U-WB7 (goes U-WB6 to U-WB8). The file headers claim "U-B1 through U-B11" and "U-WB1 through U-WB13" but the sequences have holes. While not functionally broken, this breaks the established convention of sequential numbering and makes it harder to track test coverage.
- Fix: Either renumber to close gaps, or add the missing test IDs with tests for the missing cases (e.g., U-B9 could test `check()` before init, U-WB7 could test the browser init path).

---

**Stale JSDoc comment referencing `lstat` in module-scanner.ts** - `module-scanner.ts:86`
**Confidence**: 88%
- Problem: The `buildModulesMap` function's JSDoc still says "Rejects symlinks (lstat check)" on line 86, but the implementation now uses `O_NOFOLLOW` + `open()` for symlink rejection. The comment on line 6 also references "lstat and open" but lstat is no longer used. Comments should reflect the actual implementation.
- Fix: Update the JSDoc:
```typescript
 * Security checks performed:
 * - Rejects symlinks (O_NOFOLLOW / realpath check)
```
And update line 6:
```
// component. Using it closes the TOCTOU window between open and read.
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`browser.ts` `getBackend()` skips assertion guard** - `browser.ts:89-91`
**Confidence**: 80%
- Problem: `browser.ts`'s `getBackend()` returns a hardcoded `'wasm'` without calling `assertInitialized()`, while `node.ts`'s `getBackend()` calls `assertReady().getBackend()`. The behavior is correct (browser is always WASM), but the pattern inconsistency means `getBackend()` succeeds before `init()` in browser but throws in Node.js. The pre-init test U-BR5 (`getBackend() always returns "wasm"`) confirms this is intentional, but it creates an asymmetric developer experience.
- Fix: This is a documented design choice per U-BR5. If you want consistency, wrap it in the guard. If you want browser's "always works" behavior, document the difference in the JSDoc:
```typescript
/** Returns the active backend type. Always 'wasm' in browser environments. Does NOT require init(). */
export function getBackend(): BackendType {
  return 'wasm';
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Asymmetric retry exhaustion between Node and Browser init** - `wasm.ts:131-147` vs `wasm.ts:206-215` (Confidence: 65%) -- `initWasmNode` has MAX_INIT_RETRIES circuit breaker while `initWasmBrowser` retries infinitely. The JSDoc justifies this ("simpler than Node.js"), but the structural asymmetry means browser users could loop forever on a misconfigured wasmUrl.

- **`createNativeBackend` returns `MdsNodeBackend` directly while `createWasmBackend` returns `MdsBaseBackend`** - `native.ts:29`, `wasm.ts:313` (Confidence: 70%) -- The two factory functions return different interface levels. This is by design (native has file ops built in, WASM needs wrapping via `wrapWithFileOps`), but the FEATURE_KNOWLEDGE stated "createNativeBackend(addon) and createWasmBackend(wasmModule) should both be sync DI factories" -- the return type asymmetry could confuse consumers expecting interchangeable factories.

- **Type re-export ordering differs between browser.ts and node.ts** - `browser.ts:5-13`, `node.ts:204-213` (Confidence: 62%) -- Both files re-export types from `./types.js` but browser.ts places `export type` and `export { isMdsError }` before module-level variables while node.ts places them at the end of the file. Alphabetical ordering within the export blocks is consistent, but file-level placement differs.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 3 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The refactoring achieves its primary goal of splitting the backend interface hierarchy (MdsBaseBackend / MdsNodeBackend) and removing TLA cleanly. The factory pattern split is well-motivated and the promise deduplication is applied consistently across both entry points. The main consistency issues are: (1) the unused `lstat` import and stale comments, (2) the naming divergence between assertion guard functions, (3) the JSDoc phrasing mismatch, and (4) the missing shape validation in browser init that exists in Node.js init. None are critical, but the dead import and stale comments should be fixed before merge.

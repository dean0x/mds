# Regression Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20
**Diff range**: 28829946d5..HEAD (5 commits)
**Prior resolutions**: Cycle 2 fixed 18/21 issues (2 FP, 1 deferred). Stale test:parity script and isMdsError behavioral change both resolved.

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

(none)

## Regression Checklist

- [x] No exports removed without deprecation
- [x] Return types backward compatible
- [x] Default values unchanged (or documented)
- [x] Side effects preserved (events, logging)
- [x] All consumers of changed code updated
- [x] Migration complete across codebase
- [x] CLI options preserved or deprecated
- [x] API endpoints preserved or versioned
- [x] Commit messages match implementation
- [x] Breaking changes documented in CHANGELOG

## Analysis Details

### Export Surface

No exports were removed in this diff. The `BuildModulesMapResult` type import was dropped from `wasm.ts` but remains exported from `module-scanner.ts` -- no public surface affected.

### Signature Changes

| Function | Change | Backward Compatible |
|----------|--------|---------------------|
| `createWasmBackend(options?)` | Added optional `InitOptions` param | Yes -- optional param addition |
| `browser.init(options?)` | Changed from `async function` to plain `function` returning `Promise<void>` | Yes -- callers still `await init()` |
| `scan(path, key, depth?)` | Added optional `depth` param (default 0) | Yes -- internal function, optional param |
| `varsOpt(options?)` | Changed `!== undefined` to `!= null` | Yes -- treats `null` as "no vars" (tested in U-C7) |

### Behavioral Changes

1. **`varsOpt` null handling**: `{ vars: null }` was previously forwarded as `{ vars: null }`, now treated as "no vars". This is an intentional fix -- passing `null` to the WASM backend was incorrect. Test U-C7 validates `compile('...', { vars: null })` does not throw.

2. **`browser.init()` simplification**: Old path called `wasmInit(options)` then `createWasmBackend()` (no args, called `init()` again internally). New path calls `createWasmBackend(options)` directly, which forwards options to `init(options)`. Eliminates redundant double-init. Concurrency semantics preserved: same-promise deduplication via `initVoidPromise` cache.

3. **`DEFAULT_COMPILE_OPTS` frozen shared object**: No-vars compile/check calls now pass a shared frozen object instead of allocating a new one per call. Safe because the WASM backend reads properties via `serde_wasm_bindgen` without mutating the JS object.

4. **`dependencies` -> `optionalDependencies`**: `mds-napi` moved so npm install won't fail if the native addon is unavailable. This is correct for the universal package's WASM-fallback design. The node.ts entry already handles the native-addon-missing case with a try/catch fallback.

5. **`test:parity` -> `test:native`**: Script renamed to match the renamed test file (`native-backend.spec.mjs`). Previous review cycle flagged this as broken; now resolved.

6. **`isMdsError` behavioral tightening**: Already present at diff base (commit `28829946d`). The `startsWith('mds::')` check was added in an earlier branch commit and is documented in CHANGELOG. Not a regression within this diff range.

### Depth Guard Addition

The `scan()` function in `module-scanner.ts` gained a `MAX_IMPORT_DEPTH=64` recursion bound. This is a new safety guard, not a regression. The `visited` set already prevented cycles, but a long linear chain (e.g., A->B->C->...->64th file) would previously consume unbounded stack frames. Test U-SM5 validates the resource-limit path.

### Test Coverage of Changes

| Change | Test Coverage |
|--------|--------------|
| `varsOpt` null handling | U-C7 (compile with null vars) |
| Browser init simplification | U-BR6 (concurrent init), U-BR10 (idempotent) |
| Depth guard | U-SM4 (within limit), U-SM5 (exceeds limit) |
| isMdsError prefix check | U-E5b (non-mds:: code returns false) |
| DEFAULT_COMPILE_OPTS | U-BR7, U-BR8 (compile/check after init) |

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

No regression issues found. All changes are backward compatible with the prior state of this feature branch. Export surface is preserved, function signatures are backward compatible (optional param additions only), and all behavioral changes are tested and intentional. Prior review cycle issues (stale test:parity script, undocumented isMdsError change) have been resolved. The package is not yet published to npm, so the entire `packages/mds/` directory is new code with no external consumers to regress.

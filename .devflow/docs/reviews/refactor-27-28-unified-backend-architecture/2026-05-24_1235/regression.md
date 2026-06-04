# Regression Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24
**Diff range**: c57685c73a1c6c01c12040776659b796eb363827...HEAD (4 commits)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Stale JSDoc on tryLoadCandidate** - `packages/mds/src/backend/wasm.ts:74-75`
**Confidence**: 90%
- Problem: The JSDoc says `tryLoadCandidate` returns "null if the candidate is not found (MODULE_NOT_FOUND) or the loaded module does not match the expected shape." After this change, a shape mismatch causes `validateWasmShape` to throw rather than returning null. The function now only returns null for MODULE_NOT_FOUND.
- Impact: Misleading documentation will confuse future maintainers about error-handling behavior. The caller in `_initNode` (line 198) correctly catches the thrown error, but someone reading the JSDoc would expect `null` on shape failure and might write incorrect handling code.
- Fix:
```typescript
/**
 * Attempt to load a single WASM candidate path (Node.js only).
 *
 * Returns the loaded module on success, or null if the candidate is not found
 * (MODULE_NOT_FOUND). Throws if the loaded module does not match the expected
 * WasmModule shape (missing compile/check/scanImports).
 * Re-throws unexpected errors (OOM, corrupted WASM, init failures) so the
 * caller can surface them rather than silently discarding them.
 */
```

**Stale test name and comment for U-WB13** - `packages/mds/__test__/wasm-backend.spec.mjs:152-154`
**Confidence**: 90%
- Problem: Test name says "tryLoadCandidate returns null for modules missing scanImports" and the comment says "a module without it returns null from tryLoadCandidate." After this change, `validateWasmShape` throws instead of returning null. The test assertion itself (verifying `scanImports` is present on a successful init) is correct, but the name and comment describe the old behavior.
- Impact: Misleading test name will cause confusion when the test appears in CI output or when someone is debugging a shape validation failure. The stated contract does not match the actual code.
- Fix: Rename the test to `'U-WB13: validateWasmShape rejects modules missing scanImports'` and update the comment to say "The shape check now requires scanImports; a module without it causes validateWasmShape to throw from tryLoadCandidate."

## Pre-existing Issues (Not Blocking)

### MEDIUM

**File handle not wrapped in try/finally in scan()** - `packages/mds/src/util/module-scanner.ts:239-259`
**Confidence**: 65%
- This finding is below the 80% threshold for this section. The handle returned by `openAndValidateModule` at line 239 is not wrapped in a single try/finally covering the aggregate size check and the readFile. Currently the code between lines 239 and 254 cannot throw (just arithmetic and a conditional with explicit close), but the pattern is fragile if future changes add awaits or throwable logic between handle acquisition and the try/finally at line 255. Moved to Suggestions below.

## Suggestions (Lower Confidence)

- **Fragile handle ownership in scan()** - `packages/mds/src/util/module-scanner.ts:239` (Confidence: 65%) -- The file handle from `openAndValidateModule` is not wrapped in a single try/finally spanning the aggregate size check and readFile. Currently safe because the intermediate code is synchronous arithmetic, but a future await or throw between lines 239 and 255 could leak the handle. Consider wrapping lines 239-259 in a single try/finally that always closes the handle.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

No functionality regressions detected. All 102 tests pass. The behavioral changes in this diff are intentional improvements:

1. `tryLoadCandidate` now throws on shape mismatch instead of returning null -- the caller correctly catches this and records it as `lastError` for diagnostics. Strictly better error reporting.
2. `_initBrowser` now validates the WASM module shape with `validateWasmShape` instead of an unchecked `as WasmModule` cast. Catches malformed modules that previously would have caused silent runtime failures.
3. `_resetForTesting` signature extended with a defaulted second parameter -- backward compatible with all existing callers.
4. `assertInitialized` renamed to `assertReady` -- fully migrated, no stale references remain.
5. `openNoFollow` extracted as a module-level helper -- same behavior, reduced nesting.
6. Aggregate size check moved before `readFile` -- same limit enforcement, better memory defense.

The two should-fix items are documentation drift: stale JSDoc and a stale test name that describe the old null-return behavior instead of the new throw behavior.

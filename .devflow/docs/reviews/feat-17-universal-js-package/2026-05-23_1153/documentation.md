# Documentation Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**tryLoadCandidate JSDoc contradicts implementation** - `packages/mds/src/backend/wasm.ts:78-79`
**Confidence**: 95%
- Problem: The JSDoc says "Re-throws unexpected errors so the caller can surface them" but the implementation catches ALL errors with a bare `catch {}` and returns `null` unconditionally. The documentation promises re-throw behavior that does not exist. This misleads anyone maintaining this function into believing some errors will propagate when none do.
- Fix: Update the JSDoc to match the actual catch-all behavior:
```typescript
/**
 * Attempt to load a single WASM candidate path.
 *
 * Returns the loaded module on success, or null if the candidate cannot be
 * loaded (module not found, initialization failure, etc.). All errors are
 * caught and treated as "candidate not available".
 */
```

### MEDIUM

**`_init` error message lost diagnostic context** - `packages/mds/src/backend/wasm.ts:119-121`
**Confidence**: 82%
- Problem: The previous `_init` error message included `${String(loadError)}` to surface the underlying error. The new version drops this context entirely, making the error message less actionable for users debugging WASM build issues. While the message still provides the build command hint, the root cause (e.g., "MODULE_NOT_FOUND", WASM init failure details) is now silently discarded. The JSDoc for `_init` is absent, and neither the function nor its error message documents what diagnostic information is available.
- Fix: Either restore the error context by accumulating errors from `tryLoadCandidate`, or document in a comment why the per-candidate error detail was intentionally dropped (e.g., because errors are always "module not found" noise).

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`_init` function lacks JSDoc** - `packages/mds/src/backend/wasm.ts:98`
**Confidence**: 85%
- Problem: The `_init` function is the core internal initialization logic and has no JSDoc. Other functions in this file (`init`, `tryLoadCandidate`, `_resetForTesting`, `assertInitialized`, `compileOpts`, `createWasmBackend`) all have JSDoc comments. `_init` is the only function without one, breaking the consistent documentation pattern in this file.
- Fix: Add a JSDoc comment explaining its role:
```typescript
/**
 * Internal initialization: locate and load the WASM module from known candidate paths.
 *
 * Called by `init()` — callers should not invoke this directly.
 * Throws if no candidate can be loaded.
 */
async function _init(options?: InitOptions): Promise<void> {
```

**`assertInitialized` function lacks JSDoc** - `packages/mds/src/backend/wasm.ts:124`
**Confidence**: 80%
- Problem: `assertInitialized` has no JSDoc, unlike all the other exported or significant functions in this file. A one-liner JSDoc would maintain the file's documentation consistency.
- Fix:
```typescript
/** Return the initialized WASM module, or throw if init() has not completed. */
function assertInitialized(): WasmModule {
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`scanImports` helper in scanner.spec.mjs has outdated inline comments** - `packages/mds/__test__/scanner.spec.mjs:22-26`
**Confidence**: 82%
- Problem: The inline comment says "The napi addon doesn't expose scanImports directly, so we use the compile result to determine imports... Actually we need scan_imports." This reads like a stream-of-consciousness draft that was never cleaned up. It then switches to "For testing buildModulesMap, use a simple regex-based scanner" which is accurate but the preceding confused text is misleading noise.
- Fix: Clean up to a single clear comment: `// Regex-based import scanner for testing buildModulesMap without requiring the napi addon.`

## Suggestions (Lower Confidence)

- **README could document `_resetForTesting`'s existence for contributors** - `packages/mds/src/backend/wasm.ts:41` (Confidence: 60%) -- The `@internal` tag documents this for TypeScript consumers but a note in the README's API table about the testing escape hatch would help contributors. However, `@internal` is the standard convention and may be sufficient.

- **README Node.js example uses `compile` synchronously but the import is top-level await** - `packages/mds/README.md:22` (Confidence: 65%) -- The example shows `compile('Hello {name}', ...)` called synchronously, which is correct, but the module itself uses top-level `await` for backend initialization during import. Users coming from CommonJS might be confused about why a simple `import` takes time. The existing "zero-config" heading partially addresses this, but a brief note that "the import itself performs async backend selection" could prevent confusion.

- **Test file header comment count off by one in scanner.spec.mjs** - `packages/mds/__test__/scanner.spec.mjs:2` (Confidence: 70%) -- The header says "Tests: U-S1 through U-S10" covering `normalizeVirtualKey`, plus `buildModulesMap` tests U-SM1 through U-SM5 that are not mentioned in the header range. This is minor but the pattern in other test files (e.g., error.spec.mjs which was updated to "U-E1 through U-E9") is to list the full range.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Documentation Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The documentation quality across this PR is generally strong: README is well-maintained and was updated to reflect the stricter `isMdsError` semantics, CHANGELOG covers the breaking change clearly, JSDoc coverage is thorough on types and most functions, and test file headers accurately describe their scope. The one blocking HIGH issue is a JSDoc comment on `tryLoadCandidate` that directly contradicts the implementation -- the doc claims errors are re-thrown but the code catches everything silently. This should be corrected before merge to prevent maintenance confusion.

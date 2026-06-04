# Complexity Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**`buildModulesMap` inner `scan` closure has high cyclomatic complexity** - `packages/mds/src/util/module-scanner.ts:104-168`
**Confidence**: 85%
- Problem: The `scan` inner function contains 7 decision points (visited check, symlink check, root escape check, aggregate size check, module count check, null byte check, empty path check) across ~65 lines with 4 levels of nesting (function > if > Promise.all > map callback > if). While each check is individually correct and security-necessary, the combined cognitive load makes this function harder to reason about than ideal.
- Fix: Extract the security validation for child imports into a named helper:
```typescript
function validateImportPath(importPath: string, absoluteDir: string, projectRoot: string): string {
  if (importPath.includes('\0')) {
    throw new Error('security: import path contains null byte');
  }
  if (importPath.trim().length === 0) {
    throw new Error('security: import path is empty');
  }
  const childAbsolute = resolve(absoluteDir, importPath);
  if (!childAbsolute.startsWith(projectRoot + '/') && childAbsolute !== projectRoot) {
    throw new Error(
      `security: import path escapes project root: ${childAbsolute} is outside ${projectRoot}`,
    );
  }
  return childAbsolute;
}
```
This reduces `scan` to ~40 lines with 3 nesting levels and separates security concerns from traversal logic.

### MEDIUM

**`node.ts` top-level backend selection uses nested try/catch with 4 nesting levels** - `packages/mds/src/node.ts:14-39`
**Confidence**: 82%
- Problem: The backend selection logic uses nested `try/catch` blocks (outer try for native, inner try for WASM fallback) which reaches 4 levels of indentation. This is acceptable for initialization code that runs once, but the nesting makes the control flow harder to trace visually.
- Fix: Extract the fallback logic into a named async function to flatten nesting:
```typescript
async function initBackend(): Promise<MdsBackend> {
  if (forceBackend === 'wasm') {
    const { createWasmBackend } = await import('./backend/wasm.js');
    return createWasmBackend();
  }
  const nativeResult = await tryNativeBackend();
  if (nativeResult.ok) return nativeResult.backend;
  if (forceBackend === 'native') {
    throw new Error(`MDS_BACKEND=native but native addon failed to load: ${String(nativeResult.error)}`);
  }
  return tryWasmFallback(nativeResult.error);
}
```

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

_No issues found._

## Suggestions (Lower Confidence)

- **`loadBinding` in napi/index.js mixes platform detection with loading** - `crates/mds-napi/index.js:11-46` (Confidence: 65%) -- The function handles both platform key computation and candidate resolution/loading in one 35-line function. Could be split into `getPlatformKey()` and `loadBinding()` for clarity, though this is a standard pattern for native addon loaders and the file is generated/static.

- **Duplicated `varsOpt` helper in native.ts and wasm.ts** - `packages/mds/src/backend/native.ts:22-24`, `packages/mds/src/backend/wasm.ts:98-100` (Confidence: 62%) -- Same 2-line helper is defined identically in both backends. Could be extracted to a shared utility, but the duplication is minimal and avoids coupling between the two backend implementations.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates good complexity management overall. Files are appropriately sized (all under 175 lines), functions are focused, and responsibilities are well-separated across modules. The backend adapter pattern (native.ts, wasm.ts, browser.ts) keeps each entry point simple with clear delegation. The `normalizeVirtualKey` function at 50 lines is well-structured with early returns. The main complexity concern is the `buildModulesMap` inner `scan` closure which accumulates security checks and traversal logic in one place -- extracting validation into a helper would improve readability without architectural changes.

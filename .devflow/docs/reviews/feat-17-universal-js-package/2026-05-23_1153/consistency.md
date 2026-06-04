# Consistency Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Inconsistent options-building pattern between compile/check and compileFile/checkFile in wasm.ts** - `packages/mds/src/backend/wasm.ts:155-182`
**Confidence**: 85%
- Problem: The new `compileOpts()` helper is used for `compile()` and `check()` (lines 157, 162), but `compileFile()` and `checkFile()` still inline the options-building with `...varsOpt(options)` spread (lines 168-172, 178-182). The refactoring that extracted `compileOpts()` was not applied uniformly across all four methods in the same backend object. While the file-based methods have different shapes (they supply a real `filename` and `modules`), the vars-merging pattern diverges: `compileOpts()` uses `{ ...DEFAULT_COMPILE_OPTS, ...vars }` while `compileFile`/`checkFile` uses `{ filename, modules, ...varsOpt(options) }`. This is intentional (the data differs), but the asymmetry means `compileOpts()` is not a general solution -- it is a two-method-specific helper named generically.
- Fix: Either rename `compileOpts` to something narrower like `defaultCompileOpts` to signal it only covers the string-source path, or extract a parallel helper for file-based paths:
```typescript
function fileOpts(
  entryFilename: string,
  modules: Record<string, string>,
  options?: FileOptions,
): { filename: string; modules: Record<string, string>; vars?: Record<string, unknown> } {
  return { filename: entryFilename, modules, ...varsOpt(options) };
}
```

**JSDoc style inconsistency between node.ts and browser.ts for matching functions** - `packages/mds/src/browser.ts:57-67`
**Confidence**: 82%
- Problem: The diff collapses multi-line JSDoc comments to single-line in browser.ts (e.g., `compile`, `check`, `getBackend`). The resulting single-line style is `/** Compile an MDS source string to Markdown. Requires init() to have been called and awaited first. */`. In node.ts, the equivalent function uses a shorter version `/** Compile an MDS source string to Markdown. */` without the `Requires init()` clause. While adding the init requirement note is correct for browser.ts, the formatting shift (from multi-line to single-line) is inconsistent with the multi-line style used for `init()`, `compileFile()`, and `checkFile()` in the same file. The file now mixes both styles.
- Fix: This is minor -- the single-line form is fine and the content is correct. If you want full consistency within browser.ts, either make all comments single-line (for the short ones) or keep the multi-line form for all. The current mixed state is acceptable but not ideal.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**tryLoadCandidate swallows all errors, JSDoc claims "re-throws unexpected errors"** - `packages/mds/src/backend/wasm.ts:75-96`
**Confidence**: 88%
- Problem: The JSDoc for `tryLoadCandidate` says "Re-throws unexpected errors so the caller can surface them" but the implementation uses a bare `catch` that returns `null` for every error, not just "not found" errors. This is a documentation-to-implementation mismatch. The previous inline version also swallowed all errors (catching into `loadError`), so this is not a regression, but the new extracted function's JSDoc creates a false expectation.
- Fix: Either update the JSDoc to match reality:
```typescript
/**
 * Attempt to load a single WASM candidate path.
 *
 * Returns the loaded module on success, or null on any failure.
 */
```
Or implement the documented behavior by filtering errors:
```typescript
} catch (e) {
  if (isModuleNotFoundError(e)) return null;
  throw e;
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Error message lost in _init fallback** - `packages/mds/src/backend/wasm.ts:119-121` (Confidence: 72%) -- The previous version appended `${String(loadError)}` to the WASM-load failure message; the new version omits it. This was likely intentional (the `tryLoadCandidate` helper swallows errors so there is no `loadError` to append), but losing the last error's context makes debugging harder for users when WASM fails to load.

- **Test ID numbering gap: U-E5b renamed to U-E9 but U-E5b through U-E8 comment range is misleading** - `packages/mds/__test__/error.spec.mjs:3,49` (Confidence: 65%) -- The file header says "Tests: U-E1 through U-E9" and the test was renamed from `U-E5b` to `U-E9`, which is good. However, `U-E9` appears between `U-E5` (isMdsError false for non-errors) and `U-E6` (check syntax error) in the file ordering. Reordering U-E9 to the end of the file would match the sequential numbering convention used in all other test files (U-C1-C9, U-S1-S10, U-SM1-SM5, etc.).

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase shows strong consistency overall. Naming conventions, error handling patterns (throw-based for this package, appropriate since it is a binding layer), import organization, and test structure are well-aligned between node.ts, browser.ts, and the backends. The `compileOpts()` extraction is a good deduplication move; the main gap is that it was not extended to cover the file-based methods (or renamed to reflect its limited scope). The JSDoc mismatch on `tryLoadCandidate` should be corrected to avoid misleading future readers.

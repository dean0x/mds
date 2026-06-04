# Consistency Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20

## Issues in Your Changes (BLOCKING)

### MEDIUM

**varsOpt JSDoc says "present" but implementation checks null/undefined** - `packages/mds/src/util/options.ts:4-11`
**Confidence**: 85%
- Problem: The JSDoc comment says "Build the `{ vars }` sub-object only when `options.vars` is **present**." The implementation was changed from `!== undefined` to `!= null` (loose equality), which now also filters out `null`. The word "present" is ambiguous -- it could mean "not undefined" or "not nullish". The comment does not reflect the intentional behavioral change that `null` is now treated as absent. A previous review cycle (cycle 2) already addressed the null passthrough behavior, but the JSDoc was not updated to match.
- Fix: Update the JSDoc to explicitly document the null-coalescing behavior:
```typescript
/**
 * Build the `{ vars }` sub-object only when `options.vars` is defined and non-null.
 *
 * Both native and WASM backends forward vars as a nested object. When the
 * caller passes no vars (undefined or null), omitting the key entirely avoids
 * unnecessary object creation and keeps the options shape minimal.
 */
```

---

**Test ID naming inconsistency: `U-E5b` breaks sequential numbering convention** - `packages/mds/__test__/error.spec.mjs:49`
**Confidence**: 82%
- Problem: All other test IDs across the project follow a strict sequential numbering pattern within their prefix (U-C1 through U-C9, U-E1 through U-E8, U-BR1 through U-BR10, U-SM1 through U-SM5, etc.). The new test `U-E5b` uses a suffixed sub-identifier that breaks this convention. The error.spec.mjs file header also says "Tests: U-E1 through U-E8" but the file now contains U-E5b between U-E5 and U-E6, creating a gap in the documented range.
- Fix: Renumber to `U-E9` (next in sequence) and update the file header:
```javascript
/**
 * Error shape tests for @mds/mds universal package.
 * Tests: U-E1 through U-E9
 */
```
```javascript
  test('U-E9: isMdsError returns false for errors with non-mds:: code', () => {
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**JSDoc style inconsistency between browser.ts and node.ts for parallel API functions** - `packages/mds/src/browser.ts:60-99`
**Confidence**: 80%
- Problem: The newly added JSDoc comments in browser.ts use multi-line format with extra detail (e.g., "Requires init() to have been called and awaited first.") while the parallel functions in node.ts use single-line JSDoc (e.g., `/** Compile an MDS source string to Markdown. */`). Since these are the two entry points exporting the same public API, their documentation style should be consistent.
- Fix: Either align browser.ts to use single-line JSDoc like node.ts, or expand node.ts to match browser.ts. Given that browser.ts adds genuinely useful context (the init() prerequisite), the browser.ts style is appropriate for browser-specific caveats. However, the base descriptions should match. For `compile`:
  - node.ts: `/** Compile an MDS source string to Markdown. */`
  - browser.ts: First line should match: `/** Compile an MDS source string to Markdown. Requires init() to have been called first. */`

## Pre-existing Issues (Not Blocking)

No pre-existing consistency issues above the confidence threshold.

## Suggestions (Lower Confidence)

- **Naming asymmetry between wasm.ts and browser.ts state variables** - `packages/mds/src/browser.ts:24,28` vs `packages/mds/src/backend/wasm.ts:25,27` (Confidence: 65%) -- browser.ts uses `resolvedBackend`/`initVoidPromise` while wasm.ts uses `wasmModule`/`initPromise`. The naming patterns differ in both word choice and specificity. This is minor since they are in different modules with different scopes, but `initVoidPromise` in particular is unusually specific compared to the simpler `initPromise` in wasm.ts.

- **DEFAULT_COMPILE_OPTS frozen object contains mutable nested object** - `packages/mds/src/backend/wasm.ts:106` (Confidence: 70%) -- `Object.freeze` is shallow; the `modules: {}` inner object is not frozen. The WASM backend spreads it into a new object when vars are present, but passes it directly when vars are absent. If the WASM module ever mutates the `modules` property, this shared frozen object could be corrupted. This is more of a defensive consistency concern than a known bug.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase is well-organized with strong internal consistency. The changes are clean and follow established patterns. The two blocking MEDIUM findings are documentation/naming hygiene issues:
1. The `varsOpt` JSDoc should be updated to reflect the null-handling behavioral change.
2. The `U-E5b` test ID should be renumbered to follow the sequential convention.

The JSDoc style difference between browser.ts and node.ts is a should-fix that would improve API surface consistency across entry points.

Prior cycle 2 resolutions (varsOpt null passthrough, README col/column fix) are confirmed correctly applied in the current code.

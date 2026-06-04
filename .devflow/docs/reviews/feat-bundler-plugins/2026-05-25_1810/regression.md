# Regression Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10
**Cycle**: 2 (prior cycle resolved 18/20 issues across 7 commits)

## Issues in Your Changes (BLOCKING)

No blocking regression issues found.

## Issues in Code You Touched (Should Fix)

No should-fix regression issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing regression issues found.

## Suggestions (Lower Confidence)

No suggestions.

## Regression Checklist

- [x] No exports removed without deprecation -- `isMdsError` removed from `MdsApi` interface but was never called via the interface by any bundler package; `errors.ts` uses its own duck-typed `isMdsErrorLike` instead. Safe removal.
- [x] Return types backward compatible -- all public function signatures unchanged (`createMdsTransformer`, `formatMdsError`, `shouldTransform`, `cleanId`, `isMdsExtension`).
- [x] Default values unchanged -- no defaults modified.
- [x] Side effects preserved -- warning emission, dependency registration, and HMR reload behavior all intact.
- [x] All consumers of changed code updated -- both `vite-plugin` and `rollup-plugin` now correctly pass `clean` (not raw `id`) to `transformer.transform()`, matching the intent of `cleanId()` usage.
- [x] Migration complete across codebase -- no stale references to old patterns remain.
- [x] `.gitignore` change safe -- `packages/mds/dist/` (already gitignored) generalized to `packages/*/dist/` to cover all new packages. `packages/mds/dist/` was never tracked.
- [x] Poisoned promise fix consistent -- `bundler-utils/transform.ts` resets `initPromise` on rejection; `webpack-loader/index.ts` applies the same pattern for its `initPromise`. Both retryable.
- [x] `escapeForJs` rewrite behavioral parity -- new regex-based version handles all 6 characters from the old `switch`-based version (backslash, double-quote, newline, carriage return, U+2028, U+2029) plus adds null byte escaping. Strictly additive, no characters lost.
- [x] Commit messages match implementation -- "simplify plugin code" matches Object.assign refactor and shouldTransform shorthand; "fix tautological assertions" matches the `assert.ok(typeof x, 'string')` -> `assert.equal(typeof x, 'string')` fix; "poisoned-promise, escapeForJs" matches both the init rejection handler and the regex rewrite.
- [x] Dist artifacts removed from tracking -- all deleted files are `dist/` build outputs, correctly untracked via gitignore.
- [x] All 4 test suites pass (74 tests total, 0 failures).

## Key Behavioral Changes Verified

| Change | Before | After | Regression Risk |
|--------|--------|-------|-----------------|
| `transformer.transform(id)` -> `transform(clean)` in vite/rollup | Raw id with query params passed to compiler | Clean path passed | **Bug fix** -- eliminates double-cleaning since `transform()` internally calls `cleanId()` again, but the raw id could have carried query/hash fragments the second `cleanId()` would strip. Correct change. |
| `escapeForJs` imperative -> regex | `switch(true)` loop, 6 cases | `String.replace` with map, 7 cases | **Additive** -- null byte (`\0`) now escaped. All prior escapes preserved. |
| `ensureInit` poisoned-promise reset | Rejected promise cached forever | `initPromise = null` on rejection | **Bug fix** -- transient init failures no longer permanently break the transformer. |
| `MdsApi.isMdsError` removed | Interface required `isMdsError` | Interface omits it | **Safe** -- no bundler code called `mds.isMdsError()`; duck-typed `isMdsErrorLike` in `errors.ts` handles error detection independently. |
| `_resetForTesting` production guard | No guard | Throws in `NODE_ENV=production` | **Additive** -- prevents accidental misuse; existing tests run without `NODE_ENV=production`. |

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 10/10
**Recommendation**: APPROVED

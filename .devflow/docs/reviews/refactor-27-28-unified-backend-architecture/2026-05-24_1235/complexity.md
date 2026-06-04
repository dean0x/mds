# Complexity Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24
**Diff**: c57685c73a1c6c01c12040776659b796eb363827...HEAD (4 commits)

## Issues in Your Changes (BLOCKING)

No blocking complexity issues found.

## Issues in Code You Touched (Should Fix)

No should-fix complexity issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing complexity issues found at CRITICAL severity.

## Suggestions (Lower Confidence)

- **`_initBrowser` has 3-level nesting with CSP string matching** - `wasm.ts:279-295` (Confidence: 65%) -- The CSP error detection block (4 string-match conditions inside a catch inside a try) reaches nesting depth 3. Currently manageable but approaching the warning threshold; if more CSP patterns are added this will become harder to follow. Consider extracting a `isCspError(msg)` predicate if more conditions are added.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

The changes in this incremental diff consistently **reduce** complexity rather than introduce it. Specific improvements observed:

1. **`openNoFollow` extraction (module-scanner.ts:24-34)**: Extracting the ELOOP/ENOTDIR translation from `openAndValidateModule` into a standalone module-level helper removes one nested try/catch, reducing `openAndValidateModule`'s nesting from 4 levels to 3. The JSDoc clearly states the motivation ("keeping nesting shallow"). Well-scoped single-responsibility helper.

2. **`validateWasmShape` extraction (wasm.ts:123-133)**: Replaces a 6-line inline conditional check in `tryLoadCandidate` with a reusable, tested function. The loop-over-names pattern is more maintainable than repeating three `typeof` checks. The `asserts mod is WasmModule` return type gives callers narrowing without explicit casts.

3. **`_initBrowser` restructured (wasm.ts:251-299)**: The previous version cast `imported` as `WasmModule` inline and had a fragile prefix-matching re-throw. The new version uses `validateWasmShape` for a clean boundary check with no catch-and-rethrow needed, removing one nesting level.

4. **`openAndValidateModule` phase split (module-scanner.ts:172-214)**: Splitting into open+validate and read phases is a net-neutral or slight improvement for complexity -- it adds a second try/finally in `scan()` but removes the combined read-inside-validate pattern. The caller (`scan`) now has clearer resource lifecycle management: check aggregate size on metadata first, then read.

5. **Test simplifications**: Removing `assert.doesNotThrow` wrapper (U-WB9), empty destructure `[, ]` (U-B8), and adding try/finally for cleanup (U-B6) all reduce noise without adding complexity.

All functions remain well within complexity thresholds: no function exceeds 50 lines, nesting stays at 3 or below in changed code, parameter counts are 1-3, and cyclomatic complexity per function is below 5 in all modified functions.

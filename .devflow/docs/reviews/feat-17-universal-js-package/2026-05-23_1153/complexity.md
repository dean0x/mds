# Complexity Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Cycle**: 4 (incremental after 3 prior resolution cycles)

## Cross-Cycle Awareness

Prior cycles resolved 19/21 issues including: extracting `statAndValidateModule()` to reduce `scan()` complexity, extracting `compileOpts()` helper, extracting `tryLoadCandidate()`, deep-freezing `DEFAULT_COMPILE_OPTS`, and adding `_resetForTesting()`. One FP dismissed (aggregateSize non-atomic -- JS single-threaded). One deferred (node.ts/browser.ts LSP tension). These areas are not re-flagged.

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

No should-fix issues found.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`buildModulesMap` outer function is long (148 lines)** - `module-scanner.ts:84-231`
**Confidence**: 82%
- Problem: `buildModulesMap` spans lines 84-231 (148 lines including nested functions `validateImportPath`, `statAndValidateModule`, and `scan`). The nested closures share state via `modules`, `visited`, `aggregateSize`, and `projectRoot`, which is a reasonable design choice for encapsulation. However, the outer function's total line count exceeds the 50-line warning threshold. Individual nested functions are well-sized (validateImportPath: 20 lines, statAndValidateModule: 32 lines, scan: 56 lines), and the prior cycle already extracted `statAndValidateModule` to reduce `scan()` complexity.
- Impact: The file is readable and each function has a single concern, but the long outer container makes it harder to navigate the module as a whole.
- Fix: This is not blocking -- the nested-closure pattern is the correct design here because the closures share mutable state (`aggregateSize`, `visited`, `modules`). An alternative would be extracting to a class with private state, but that adds indirection without meaningful complexity reduction. Acceptable as-is.

## Suggestions (Lower Confidence)

- **`normalizeVirtualKey` has 7 branch points** - `module-scanner.ts:27-72` (Confidence: 65%) -- Cyclomatic complexity is approximately 7 (empty string check, null byte check, base-length check, segment count check, loop with `.`/`..`/normal branches, final empty check). Each branch is a distinct validation concern and the function is 46 lines, which is within tolerance. Could benefit from extracting the relative-path resolution loop into a helper, but this is a style preference.

- **`compileFile`/`checkFile` near-duplication in wasm backend** - `wasm.ts:165-183` (Confidence: 62%) -- The `compileFile` and `checkFile` methods in `createWasmBackend` share identical module-resolution logic, differing only in the final `wasm.compile()` vs `wasm.check()` call. A shared `resolveAndInvoke(path, options, method)` helper could eliminate the duplication, but at 8 lines each this is borderline and could reduce readability.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED

### Rationale

The codebase demonstrates good complexity management after 3 prior resolution cycles:

1. **Function decomposition is effective** -- `scan()` was reduced from ~90 lines to ~56 lines by extracting `statAndValidateModule()`. The `tryLoadCandidate()` extraction reduced `_init()` from ~30 lines to ~25 lines. The `compileOpts()` helper eliminated inline duplication in `compile()`/`check()`.

2. **File sizes are within tolerance** -- wasm.ts (189 lines), browser.ts (96 lines), module-scanner.ts (231 lines), options.ts (12 lines), types.ts (85 lines). Only module-scanner.ts is in the warning zone (>200 lines) but this is entirely due to the necessarily-nested closure structure.

3. **Nesting depth is well controlled** -- Maximum nesting is 3 levels (buildModulesMap > scan > Promise.all callback), which is within the "good" threshold. No function exceeds 4 levels of nesting.

4. **Cyclomatic complexity per function is low** -- `init()`: 3, `_init()`: 2, `tryLoadCandidate()`: 2, `scan()`: 5, `statAndValidateModule()`: 3, `validateImportPath()`: 3, `normalizeVirtualKey()`: 7. All under the warning threshold of 10.

5. **Parameter counts are minimal** -- No function exceeds 3 parameters. The `ModuleScannerOptions` object pattern is correctly used for optional configuration.

No blocking or should-fix complexity issues remain. The architecture is clean and well-decomposed.

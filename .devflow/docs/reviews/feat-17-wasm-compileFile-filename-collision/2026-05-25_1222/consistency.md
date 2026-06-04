# Consistency Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25T12:22

## Issues in Your Changes (BLOCKING)

_No blocking issues found._

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Novel error prefix `invariant violation:` inconsistent with codebase conventions** - `packages/mds/src/node.ts:75-77`
**Confidence**: 82%
- Problem: The new `prepareFileArgs` function throws with an `invariant violation:` prefix. The codebase uses domain-specific error prefixes consistently: `security:` for security guards, `resource limit:` for bound checks, and `@mds/mds:` for public API misuse. `invariant violation:` is a new pattern not used anywhere else in the codebase (0 other occurrences in `src/`).
- Fix: While the intent is correct (this is a defensive assertion), align the message to match existing conventions. Since this guards an internal contract of `buildModulesMap`, a descriptive error without a novel prefix is more consistent:
```typescript
throw new Error(
  `buildModulesMap did not populate entry file "${entryFilename}" in modules map`,
);
```
Alternatively, if the team wants a dedicated prefix for internal assertions, it should be adopted across the codebase — not introduced in a single location.

## Pre-existing Issues (Not Blocking)

_No pre-existing issues found._

## Suggestions (Lower Confidence)

- **Mixed assertion styles in test file** - `packages/mds/__test__/wasm-compileFile.spec.mjs:57,140,176` (Confidence: 65%) — The file predominantly uses `assert.ok()` (13 calls) but also uses `assert.equal()` at lines 140 and 176 (in parity tests). The sibling `compileFile.spec.mjs` uses `assert.ok` exclusively (19 calls, 0 `assert.equal`). This is acceptable since `assert.equal` is semantically appropriate for exact value comparison in parity tests, making it an intentional choice rather than drift. Noting for awareness only.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | 0 |
| Pre-existing | - | - | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The changes show strong consistency overall:
- The DRY refactor of duplicated `compileFile`/`checkFile` logic into `prepareFileArgs` is well-executed and consistent with the codebase's extraction patterns.
- The test refactor from `runWasm`/`runNative` into `runScript` + `wasmEnv()`/`nativeEnv()` is a clear improvement, eliminating duplicated subprocess configuration and making the env-switching explicit.
- The `delete modules[entryFilename]` mutation pattern is documented in both the JSDoc on `BuildModulesMapResult.modules` and the `prepareFileArgs` docstring, making the contract clear.
- Import consolidation to `helpers.mjs` is consistent with how other test files (`compileFile.spec.mjs`, `check.spec.mjs`) import shared fixtures.
- New tests U-WCF9 through U-WCF11 follow the exact same structure as existing tests U-WCF7 and U-WCF8, demonstrating good pattern adherence.

The single should-fix item is a minor error message prefix inconsistency that does not affect correctness.

# Regression Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25T12:22
**Commit range**: db99f70...HEAD (1 commit: 687315c)

## Issues in Your Changes (BLOCKING)

No blocking regression issues found.

## Issues in Code You Touched (Should Fix)

No should-fix regression issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing regression issues found.

## Suggestions (Lower Confidence)

_No suggestions._

## Analysis Notes

### Regression Checklist Evaluation

**Exports preserved**: All 9 public exports from `packages/mds/src/node.ts` are identical before and after: `_resetForTesting`, `init`, `compile`, `check`, `compileFile`, `checkFile`, `getBackend`, `isMdsError`, and the type re-exports. No exports were removed, renamed, or had their signatures changed.

**Return types preserved**: `compileFile` still returns `Promise<CompileResult>`, `checkFile` still returns `Promise<CheckResult>`. No widening or narrowing.

**No files removed**: All three changed files (`node.ts`, `module-scanner.ts`, `wasm-compileFile.spec.mjs`) are modifications only. No deletions.

**Error behavior change -- intentional and correct**: The previous code used `modules[entryFilename] ?? ''` which silently fell back to an empty string if the entry filename was somehow missing from the modules map. The new code throws an explicit invariant violation error. This is a deliberate behavioral change that surfaces bugs earlier. The fallback to `''` was a silent-failure anti-pattern -- `buildModulesMap()` is contractually guaranteed to include the entry file (it is the first file scanned at line 287 of `module-scanner.ts`), so `undefined` would indicate a bug in `buildModulesMap`, not a user error. This change was flagged and resolved in cycle 1. No regression risk.

**DRY refactor -- no semantic change**: The extract of `prepareFileArgs()` consolidates the duplicated extract-and-delete logic from `compileFile` and `checkFile` into a single helper. Both call sites now invoke `prepareFileArgs(path, options)` and use the returned `{ source, opts }`. The logic is identical to the previous inline versions, just deduplicated. No regression risk.

**Test helper refactor -- no behavioral change**: `runWasm(script, extraEnv)` and `runNative(script)` were replaced by a unified `runScript(script, env)` with helper functions `wasmEnv()` and `nativeEnv()`. The subprocess invocation is identical: same `execFile` call, same timeout, same cwd. The only additions are (a) the empty-stdout guard (`if (!stdout.trim()) throw ...`) and (b) explicit env parameter instead of merging into process.env. All existing test IDs (U-WCF1 through U-WCF8) are preserved with equivalent logic.

**New tests are additive only**: U-WCF9, U-WCF10, and U-WCF11 are new tests. They do not modify or replace existing test cases. U-WCF9/U-WCF10 add parity checks for the import scenario (IMPORT_CONSUMER_MDS). U-WCF11 adds the checkFile error-path test mirroring U-WCF6.

**U-WCF1 assertion weakened (cosmetic)**: Line 57 changed from `assert.equal(typeof result.output, 'string', ...)` to `assert.ok(typeof result.output === 'string', ...)`. Both are functionally equivalent (both fail if output is not a string), but `assert.equal` provides better diagnostic messages on failure (shows expected vs actual). This is a cosmetic concern, not a regression.

**U-WCF4 assertion tightened**: The assertion changed from `result.output.includes('99')` to `result.output.includes('You have 99 items')`. Given the fixture content (`Hello {name}! You have {count} items.`), the tighter assertion is more precise and less prone to false passes. This is a strict improvement.

**Commit message matches implementation**: The commit message describes 6 changes. All 6 are present in the diff: (1) invariant assertion, (2) JSDoc on BuildModulesMapResult.modules, (3) empty stdout guard, (4) tightened U-WCF4 assertion, (5) new parity tests U-WCF9/U-WCF10, (6) new error path test U-WCF11. No intent-vs-reality mismatch.

**Fixture paths via shared helpers**: Test constants (`SIMPLE_MDS`, `IMPORT_CONSUMER_MDS`, `ENTRY_MDS`, `__dirname`) are now imported from `./helpers.mjs` instead of being defined inline. The helpers module resolves identical paths. No regression risk.

**Prior cycle resolution cross-reference**: The silent empty-source fallback issue (now an explicit throw) and the fragile U-WCF4 assertion were both resolved in cycle 1. This review confirms those fixes are correctly implemented. No regressions reintroduced.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

The score is 9/10 rather than 10/10 due to the minor U-WCF1 assertion weakening (`assert.equal` to `assert.ok`) which reduces diagnostic quality on failure but does not affect correctness. All public API surfaces, return types, error behaviors, and test coverage are preserved or improved. The changes are well-scoped, additive, and carry minimal regression risk.

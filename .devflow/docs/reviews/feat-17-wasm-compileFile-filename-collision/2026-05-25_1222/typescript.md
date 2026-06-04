# TypeScript Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25
**Diff**: `git diff db99f70...HEAD`
**Prior Resolutions**: Cycle 1 resolved 6 issues, 4 false positives (delete mutation safety, asymmetric backend architectures, subprocess test overhead, parity test concern). These are not re-flagged.

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

No should-fix issues found.

## Pre-existing Issues (Not Blocking)

No critical pre-existing issues found in the changed files.

## Suggestions (Lower Confidence)

No suggestions.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**TypeScript Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

### What was reviewed

- `packages/mds/src/node.ts` -- New `prepareFileArgs` helper function extracted from duplicated logic in `compileFile`/`checkFile`.
- `packages/mds/src/util/module-scanner.ts` -- JSDoc additions to `BuildModulesMapResult.modules` documenting the filename collision contract.
- `packages/mds/__test__/wasm-compileFile.spec.mjs` -- Test refactoring (unified `runScript`, new parity and error tests U-WCF9 through U-WCF11).

### Positive observations

1. **Proper `noUncheckedIndexedAccess` handling** -- `prepareFileArgs` in `node.ts:73` correctly accounts for `noUncheckedIndexedAccess` by checking `source === undefined` before using the value. This replaces the prior `?? ''` silent fallback with an explicit invariant violation, which is the correct approach.

2. **Explicit return type annotation** -- `prepareFileArgs` declares `Promise<{ source: string; opts: ReturnType<typeof fileOpts> }>`, providing clear type documentation. Using `ReturnType<typeof fileOpts>` avoids duplicating the options shape.

3. **DRY extraction** -- The duplicated `buildModulesMap` + `delete` + `fileOpts` sequence in `compileFile` and `checkFile` is correctly consolidated into `prepareFileArgs`. Both callers are now one-liners with identical type safety.

4. **Strict tsconfig** -- The project has `strict: true` and `noUncheckedIndexedAccess: true` in `tsconfig.base.json`, which is the recommended configuration.

5. **Interface documentation** -- The JSDoc on `BuildModulesMapResult.modules` clearly documents the filename collision contract and what callers MUST do, establishing a documented invariant at the type level.

6. **No `any` types** -- All new code uses proper types. No `any` escape hatches introduced.

7. **Test file is `.mjs`** -- Not TypeScript, so TypeScript-specific patterns (generics, branded types, etc.) do not apply. The refactoring from `runWasm`/`runNative` to `runScript(script, env)` is clean and reduces code duplication.

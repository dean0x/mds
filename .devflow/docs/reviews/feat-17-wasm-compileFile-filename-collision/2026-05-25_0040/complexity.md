# Complexity Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-25
**PR**: #30 -- Fix WASM compileFile/checkFile filename collision

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Duplicated extract-and-delete logic in compileFile/checkFile** - `packages/mds/src/node.ts:67-73, 78-81`
**Confidence**: 90%
- Problem: The same 3-line pattern (extract source from modules map, delete entry key, call WASM function) is copy-pasted identically between `compileFile` and `checkFile`. Both methods perform `const source = modules[entryFilename] ?? ''`, `delete modules[entryFilename]`, then pass `source` and `fileOpts(entryFilename, modules, options)` to the respective WASM method. This is a minor duplication (only 2 occurrences), but the logic is non-obvious (the comment explains a subtle WASM API contract), making it easy for a future contributor to update one callsite and forget the other.
- Fix: Extract a shared helper function that encapsulates the modules-map preparation:
```typescript
async function prepareFileArgs(
  path: string,
  options: FileOptions | undefined,
): Promise<{ source: string; opts: ReturnType<typeof fileOpts> }> {
  const { entryFilename, modules } = await buildModulesMap(path, (src) => wasmModule.scanImports(src));
  const source = modules[entryFilename] ?? '';
  delete modules[entryFilename];
  return { source, opts: fileOpts(entryFilename, modules, options) };
}
```
Then both methods become single-line delegations. (Note: the working tree already contains this refactoring but it has not been committed.)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Repetitive inline script strings in tests** - `packages/mds/__test__/wasm-compileFile.spec.mjs:65-69, 78-82, 95-99, 106-110` (Confidence: 65%) -- Tests U-WCF1 through U-WCF5 each construct a nearly identical inline ESM script string (import, init, call, write JSON). A helper like `compileFileScript(path, opts?)` could reduce the boilerplate and make the test intent clearer. However, for subprocess-isolated tests, inline scripts are a reasonable and explicit approach, and the repetition is not severe enough to constitute a maintainability risk.

- **U-WCF6 bypasses the runWasm helper** - `packages/mds/__test__/wasm-compileFile.spec.mjs:140-149` (Confidence: 70%) -- Test U-WCF6 manually calls `exec()` with inline environment setup instead of using the `runWasm` helper. This creates a minor inconsistency in the test file: 7 tests use `runWasm`/`runNative`, while 1 duplicates the subprocess logic inline. The test works correctly as-is, but using `runWasm` would be more consistent.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The production code change is minimal (13 lines) and well-scoped: a targeted bug fix in `wrapWithFileOps`. The only blocking finding is a duplicated 3-line pattern between `compileFile` and `checkFile`, which carries moderate risk of divergence during future maintenance. The working tree already contains the refactoring that addresses this (a `prepareFileArgs` helper), so the condition is: commit the refactoring before merge, or accept the minor duplication as-is. The test file (188 lines, 8 tests) is well-structured with clear subprocess isolation, appropriate timeouts (30s), and good coverage of both happy paths and error cases. Overall complexity is low and the code is easy to reason about.

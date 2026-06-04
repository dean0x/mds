# TypeScript Review Report

**Branch**: feat-17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25

## Issues in Your Changes (BLOCKING)

### HIGH

**Code duplication in compileFile/checkFile — extract/delete pattern repeated verbatim** - `packages/mds/src/node.ts:67-73` and `packages/mds/src/node.ts:78-81`
**Confidence**: 85%
- Problem: The three-line sequence (`const source = modules[entryFilename] ?? ''`, `delete modules[entryFilename]`, call WASM) is copy-pasted between `compileFile` and `checkFile`. This is a DRY violation — if the entry-extraction logic ever needs adjustment (e.g., adding validation that `entryFilename` actually exists in the map, or logging), both sites must be updated in lockstep. The comment on `checkFile` even says "Same fix as compileFile", acknowledging the duplication.
- Fix: Extract a shared helper function (the working tree already has `prepareFileArgs` which is the correct refactor — this should be in the committed code):
```typescript
async function prepareFileArgs(
  path: string,
  wasmModule: WasmModule,
  options: FileOptions | undefined,
): Promise<{ source: string; opts: ReturnType<typeof fileOpts> }> {
  const { entryFilename, modules } = await buildModulesMap(path, (src) => wasmModule.scanImports(src));
  const source = modules[entryFilename] ?? '';
  delete modules[entryFilename];
  return { source, opts: fileOpts(entryFilename, modules, options) };
}
```

### MEDIUM

**Test file uses untyped `.mjs` instead of `.ts` — no TypeScript coverage for test code** - `packages/mds/__test__/wasm-compileFile.spec.mjs`
**Confidence**: 82%
- Problem: The test file is plain JavaScript (`.mjs`). While the `tsconfig.json` excludes `__test__/` so this is consistent with project convention, the test code contains untyped patterns: `JSON.parse(stdout)` returns `any`, and all `result.*` property accesses are unchecked. The `runWasm` and `runNative` helpers return `any` from `JSON.parse`. This means typos in property names (e.g., `result.dependecies`) would silently pass at compile time.
- Fix: If project convention is `.mjs` tests, this is acceptable. However, consider adding a `@ts-check` comment or a JSDoc `@returns` annotation to `runWasm`/`runNative` to at least document the expected return shape:
```javascript
/**
 * @param {string} script
 * @param {Record<string,string>} [extraEnv]
 * @returns {Promise<Record<string, unknown>>}
 */
async function runWasm(script, extraEnv = {}) { ... }
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`modules` object mutation via `delete` — violates immutability principle** - `packages/mds/src/node.ts:72,80`
**Confidence**: 82%
- Problem: The fix uses `delete modules[entryFilename]` which mutates the object returned by `buildModulesMap`. While the comment in the commit message correctly notes "the modules object is a fresh allocation per buildModulesMap() call so the mutation is safe," the `delete` operator in strict TypeScript is a code smell — it changes the shape of the object at runtime, defeating the structural type system. With `noUncheckedIndexedAccess: true` enabled, `modules[entryFilename]` already returns `string | undefined`, so the `?? ''` fallback is correct, but `delete` is still a mutation of an object whose type (`Record<string, string>`) does not communicate that keys may be removed.
- Note: This is pre-existing design — the fix correctly works within the existing mutable pattern. Not blocking.

## Suggestions (Lower Confidence)

- **U-WCF6 breaks helper pattern to use raw exec** - `packages/mds/__test__/wasm-compileFile.spec.mjs:140-148` (Confidence: 70%) — Test U-WCF6 duplicates the subprocess-spawning logic instead of using the `runWasm` helper. The test catches errors inside the subprocess script and serializes them, so `runWasm` would work here. The likely reason is that `runWasm` would throw on a non-zero exit code, but the subprocess in U-WCF6 always exits 0 (the catch writes JSON). Consider unifying through `runWasm` for consistency.

- **No error message content assertion in U-WCF6** - `packages/mds/__test__/wasm-compileFile.spec.mjs:150` (Confidence: 65%) — The test only checks `result.threw === true` but does not validate the error message content. A future regression could throw a different error (e.g., a generic JS error instead of the expected filesystem error) and U-WCF6 would still pass. Consider asserting that `result.message` contains expected text like "ENOENT" or "no such file".

- **Parity tests (U-WCF7, U-WCF8) may be flaky if native backend unavailable** - `packages/mds/__test__/wasm-compileFile.spec.mjs:160,178` (Confidence: 62%) — `runNative` removes `MDS_BACKEND` from env, letting the package auto-detect. If the native addon is not built in a CI environment, this falls back to WASM silently, making the "parity" test compare WASM-to-WASM rather than WASM-to-native. The test would pass but provide no value.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**TypeScript Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions: The HIGH-severity code duplication should be addressed (the working tree already contains the `prepareFileArgs` refactor, which resolves it -- ensure that refactor is committed as part of this PR). The fix itself is correct and well-commented. Type safety in the production TypeScript code is strong -- `strict: true`, `noUncheckedIndexedAccess: true`, proper `type` imports, discriminated union return types. The test file being `.mjs` is consistent with project convention.

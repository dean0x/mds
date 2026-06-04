# TypeScript Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27T01:00
**Commits**: 0c0c3fe fix(webpack-loader): add CJS build for Webpack 5 compatibility

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Type assertion on `_esmImport` return bypasses runtime type safety** - `packages/webpack-loader/src/index.ts:40`
**Confidence**: 82%
- Problem: `_esmImport('@mds/mds') as typeof import('@mds/mds')` is a type assertion (`as`) on the return of a `new Function`-based dynamic import. The `_esmImport` helper correctly returns `Promise<unknown>`, but the call site immediately widens it with a type assertion rather than narrowing. If `@mds/mds` changes its export shape, this assertion will silently mask the mismatch at compile time since `as` casts are unchecked.
- Fix: This is a pragmatic tradeoff acknowledged by the PR -- the `new Function` trick intentionally escapes the compiler to preserve `import()` in CJS output. The `as typeof import(...)` assertion is the idiomatic approach for this pattern, and adding a runtime type guard for every property of the MDS API would be disproportionate overhead. However, consider adding a minimal runtime sanity check after the import to fail fast if the module shape is wrong:
  ```typescript
  const mds = await _esmImport('@mds/mds') as typeof import('@mds/mds');
  if (typeof mds.compileFile !== 'function') {
    throw new Error('Incompatible @mds/mds module — expected compileFile export');
  }
  ```
  This provides a single assertion as a canary without full validation overhead. Not blocking given the controlled dependency relationship (`peerDependencies` pins the version range).

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **CJS test files re-resolve the same path on every test case** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs:21-64`, `packages/webpack-loader/__test__/cjs-compat.spec.mjs:21-51` (Confidence: 65%) -- Each test independently calls `require(resolve(__dirname, '../dist-cjs/index.js'))`. Node caches `require()` results so there is no correctness or performance issue, but extracting the resolved path to a `const cjsEntry = resolve(__dirname, '../dist-cjs/index.js')` at module scope would reduce repetition and make the require target a single source of truth.

- **Async function detection in webpack-loader CJS test is fragile** - `packages/webpack-loader/__test__/cjs-compat.spec.mjs:31-33` (Confidence: 72%) -- The test checks `mdsLoader.constructor.name === 'AsyncFunction' || mdsLoader.toString().includes('async')` to verify the loader is async. The `toString()` fallback is brittle because minifiers or CJS compilation may strip the `async` keyword text. Since the CJS build is not minified (raw tsc output), this works today, but a more robust check would be to verify the function returns a thenable: `const result = mdsLoader.call(mockCtx); assert.equal(typeof result?.then, 'function')`.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

The TypeScript changes in this PR are well-executed:

1. **Type safety**: The `_esmImport` helper is correctly typed as `(id: string) => Promise<unknown>`, avoiding `any`. The `as` cast at the call site is the standard approach for this `new Function` CJS-escape pattern and is well-documented with a GitHub issue reference.

2. **Strict mode compliance**: Both `tsconfig.cjs.json` files extend `tsconfig.base.json` which has `strict: true` and `noUncheckedIndexedAccess: true`. The CJS configs correctly override only `module`, `moduleResolution`, and `outDir` while disabling declarations (since the ESM build provides the canonical `.d.ts` files).

3. **Type-only imports**: `import type { MdsPluginOptions }` correctly uses the `type` modifier, ensuring it is erased in both ESM and CJS output.

4. **Export conditions**: The `package.json` exports map correctly separates `types`, `import`, and `require` conditions, with types pointing to the ESM declaration files. This is the correct dual-build pattern.

5. **No `any` usage**: Grep confirms zero `any` types across both packages.

The single MEDIUM finding (type assertion on dynamic import) is a known tradeoff of the `new Function` CJS-escape-hatch pattern rather than a design flaw. The condition for approval is to consider adding a minimal runtime sanity check on the imported module shape.

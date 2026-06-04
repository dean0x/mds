# TypeScript Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**PR**: #34

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Runtime shape check bypasses the type assertion it follows** - `packages/webpack-loader/src/index.ts:47-48`
**Confidence**: 85%
- Problem: Line 47 uses `as typeof import('@mds/mds')` to assert the module's type, then line 48 immediately re-casts the typed value with `as Record<string, unknown>` to perform a runtime shape check. The runtime check validates only `compileFile` but the `typeof import('@mds/mds')` assertion has already told the compiler the full module shape is present. If the module were actually missing `init()` (which `createMdsTransformer` calls internally via the `MdsApi` interface), the type assertion would suppress the compiler diagnostic and the runtime check would not catch it.
- Fix: Either widen the runtime validation to cover the full `MdsApi` interface (at minimum `compileFile` and `init`), or replace the type assertion with a type guard that narrows from `unknown`:

```typescript
const mds: unknown = await _esmImport('@mds/mds');
if (
  typeof mds !== 'object' || mds === null ||
  typeof (mds as Record<string, unknown>)['compileFile'] !== 'function' ||
  typeof (mds as Record<string, unknown>)['init'] !== 'function'
) {
  throw new Error(
    '@mds/mds module shape is unexpected: expected compileFile and init functions. ' +
    'Check that the installed version is compatible.',
  );
}
return createMdsTransformer(mds as MdsApi, options);
```

This keeps the variable typed as `unknown` until the guard narrows it, which is the idiomatic TypeScript pattern (avoids the anti-pattern of asserting then re-asserting). The `MdsApi` type from `@mds/bundler-utils` is the correct narrowing target since that is what `createMdsTransformer` actually accepts.

### MEDIUM

**Module-level `projectRootCache` has no eviction and no `_resetForTesting` export** - `packages/mds/src/util/module-scanner.ts:25`
**Confidence**: 82%
- Problem: The `projectRootCache` is a module-level `Map` that grows monotonically. In a long-running process (e.g., Webpack watch mode), every unique entry-file directory adds an entry that is never evicted. The `findProjectRoot` function is exported and tested, but the cache is not clearable from tests. The `findProjectRoot` tests work around this by creating fresh `mkdtemp` directories for each test (so cache keys never collide), but this is fragile -- any test that re-uses a directory path after filesystem changes will get stale cached results.
- Fix: Export a `_resetProjectRootCacheForTesting` function gated on `NODE_ENV=test` (matching the existing pattern in `webpack-loader/src/index.ts`), or accept the current workaround as sufficient since tests already use unique temp dirs.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`existsSync` for marker detection is synchronous I/O in an otherwise async module** - `packages/mds/src/util/module-scanner.ts:52`
**Confidence**: 80%
- Problem: `findProjectRoot` uses `existsSync` (synchronous I/O) to check for `.git` and `.mdsroot` markers. The comment at line 22-23 acknowledges this: "Each traversal performs up to MAX_TRAVERSAL_DEPTH x |markers| synchronous I/O calls, which can block the event loop on deep trees or network FSes." The function is called from `buildModulesMap` which is otherwise fully async. The caching mitigates repeat calls, but the first call for a given start directory will perform up to 512 synchronous stat calls.
- Fix: This is an intentional design choice (the function is synchronous to simplify the caller contract, and caching limits the impact). No change required, but document the tradeoff in the JSDoc more prominently or consider an async variant for the initial uncached call if Webpack watch-mode performance on network filesystems becomes an issue.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`O_NOFOLLOW` fallback uses `as Record<string, number>` type assertion** - `packages/mds/src/util/module-scanner.ts:9`
**Confidence**: 80%
- Problem: Line 9 casts `constants` to `Record<string, number>` to access `O_NOFOLLOW` which may not exist on all platforms. This is a valid workaround for the platform gap but uses `as` assertion rather than a proper type narrowing. Pre-existing -- not introduced in this PR.

## Suggestions (Lower Confidence)

- **Repeated `require(resolve(__dirname, '../dist-cjs/index.js'))` in CJS tests** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs` and `packages/webpack-loader/__test__/cjs-compat.spec.mjs` (Confidence: 65%) -- Each test case independently requires the same path. A shared `const mod = require(...)` at the `describe` scope would reduce repetition and make path changes a single-line edit. Minor style concern in .mjs test files (not TypeScript source), so low priority.

- **`typeof import('@mds/mds')` resolves to the module namespace type at compile time but runtime shape may differ** - `packages/webpack-loader/src/index.ts:47` (Confidence: 70%) -- The `as typeof import(...)` assertion is correct for the ESM path but when loaded via `new Function('id', 'return import(id)')` from a CJS context, the module namespace may have its exports wrapped differently (e.g., `default` wrapper). The existing runtime check on `compileFile` partially addresses this, but named exports from ESM modules loaded dynamically can appear under a `.default` property in some bundler/runtime combinations. Worth adding a debug log or a more comprehensive shape check if CJS interop issues are reported.

- **Test file U-PR4 assertion is weak** - `packages/mds/__test__/scanner.spec.mjs:268-269` (Confidence: 65%) -- The test for the filesystem-root sentinel case asserts only `typeof result === 'string' && result.length > 0`, which would pass for any non-empty string. The comment explains why -- `os.tmpdir()` may be inside a git repo -- but this means the test does not meaningfully validate the fallback behavior. Consider using a known path that is guaranteed to have no markers, or skip the test on platforms where the assertion cannot be meaningful.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The TypeScript changes are well-structured with strong type safety overall. The project correctly uses `strict: true` and `noUncheckedIndexedAccess: true`. The `findProjectRoot` function is properly typed with bounded loops and caching. The main concern is the type assertion pattern in the webpack loader's ESM import wrapper -- the `as typeof import(...)` assertion followed by a partial runtime check is a minor type-safety gap. Both packages compile cleanly with no TypeScript errors. The CJS compatibility test files are `.mjs` (not TypeScript) and appropriately test the behavioral contracts of the compiled output. Applies ADR-001 (pre-merge quality gate): all TypeScript compilation checks pass.

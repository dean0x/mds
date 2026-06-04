# TypeScript Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24T12:35

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Redundant type assertion after assertion function** - `packages/mds/src/backend/wasm.ts:97`
**Confidence**: 95%
- Problem: `validateWasmShape(mod)` is an `asserts mod is WasmModule` function. After it returns, TypeScript already narrows `mod` to `WasmModule`. The subsequent `const wasmMod = mod as WasmModule` is a redundant type assertion that adds noise and undermines the assertion function's purpose.
- Fix: Remove the `as WasmModule` cast:
  ```typescript
  validateWasmShape(mod);
  const wasmMod = mod;
  // Or simply use `mod` directly since it is now narrowed to WasmModule.
  ```

**File handle not wrapped in a single try/finally in scan()** - `packages/mds/src/util/module-scanner.ts:239-259`
**Confidence**: 82%
- Problem: After `openAndValidateModule` returns the open handle, the caller has two separate cleanup paths: an explicit `handle.close()` in the aggregate-size-exceeded branch (line 248) and a `try/finally` block for `readFile` (lines 255-259). While currently safe because no `await` or throw can occur between them, this pattern is fragile -- any future modification that introduces an `await` or fallible operation between lines 239 and 255 (outside the try/finally) would leak the file descriptor.
- Fix: Wrap the entire handle usage in a single try/finally:
  ```typescript
  const { handle, size: fileSize } = await openAndValidateModule(absolutePath);
  try {
    aggregateSize += fileSize;
    if (aggregateSize > maxAggregateSize) {
      throw new Error(
        `resource limit: aggregate module size exceeds maximum of ${maxAggregateSize} bytes`,
      );
    }
    content = await handle.readFile({ encoding: 'utf-8' });
  } finally {
    await handle.close();
  }
  ```

## Issues in Code You Touched (Should Fix)

### LOW

**`Awaited<ReturnType<typeof open>>` instead of named `FileHandle` type** - `packages/mds/src/util/module-scanner.ts:24`, `packages/mds/src/util/module-scanner.ts:174`
**Confidence**: 85%
- Problem: The return type `Promise<Awaited<ReturnType<typeof open>>>` in `openNoFollow` (line 24) and `openAndValidateModule` (line 174) is verbose and harder to read than the named `FileHandle` type from `node:fs/promises`. `Awaited<ReturnType<typeof open>>` resolves to `FileHandle` -- using the named type communicates intent more clearly and is consistent with idiomatic Node.js TypeScript.
- Fix: Import and use `FileHandle` directly:
  ```typescript
  import { open, realpath, type FileHandle } from 'node:fs/promises';
  // ...
  async function openNoFollow(absolutePath: string): Promise<FileHandle> {
  // ...
  async function openAndValidateModule(
    absolutePath: string,
  ): Promise<{ handle: FileHandle; size: number }> {
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing TypeScript issues found in reviewed files.

## Suggestions (Lower Confidence)

- **`default` check in `_initBrowser` may be unreachable after `validateWasmShape`** - `packages/mds/src/backend/wasm.ts:272` (Confidence: 65%) -- `validateWasmShape` asserts the module has `compile`, `check`, `scanImports`, but does not check `default`. The `WasmModule` interface marks `default` as optional. The runtime check on line 272 is therefore correct and necessary, but the comment on line 267 ("no need to catch here") could be misleading if a reader assumes `validateWasmShape` covers `default` as well. Consider adding a brief note that `default` is intentionally checked separately because it is browser-required but not part of the base shape.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The TypeScript usage is solid overall. Strict mode and `noUncheckedIndexedAccess` are enabled. The `asserts mod is WasmModule` pattern is a good application of assertion functions for runtime validation with type narrowing. The `unknown` type is used correctly for dynamic imports and `require()` results. The two MEDIUM blocking items are a redundant type assertion and a fragile file handle cleanup pattern -- both low-risk but worth addressing for maintainability.

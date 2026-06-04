# TypeScript Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Unsafe type assertion on environment variable** - `packages/mds/src/node.ts:10`
**Confidence**: 92%
- Problem: `process.env['MDS_BACKEND'] as BackendType | undefined` casts an arbitrary string to `BackendType | undefined` without validation. If a user sets `MDS_BACKEND=foo`, the value passes the `=== 'wasm'` check silently and falls into the native-first path with no indication the env var was invalid.
- Fix: Validate the env var at the boundary:
```typescript
const rawBackend = process.env['MDS_BACKEND'];
const forceBackend: BackendType | undefined =
  rawBackend === 'native' || rawBackend === 'wasm' ? rawBackend : undefined;
if (rawBackend !== undefined && forceBackend === undefined) {
  console.warn(`@mds/mds: ignoring invalid MDS_BACKEND="${rawBackend}" (expected "native" or "wasm")`);
}
```

**Missing explicit return type annotation on `buildFileModules`** - `packages/mds/src/backend/wasm.ts:102`
**Confidence**: 82%
- Problem: The helper `async function buildFileModules(wasm: WasmModule, path: string)` has no explicit return type. While TypeScript infers it, this is a module-internal function whose return shape (`BuildModulesMapResult`) is critical to both `compileFile` and `checkFile`. An explicit annotation documents intent and catches regressions when the imported `buildModulesMap` signature evolves.
- Fix:
```typescript
async function buildFileModules(wasm: WasmModule, path: string): Promise<BuildModulesMapResult> {
```
(Import `BuildModulesMapResult` from `'../util/module-scanner.js'`.)

### MEDIUM

**Module-level mutable state without encapsulation** - `packages/mds/src/backend/wasm.ts:27-29`
**Confidence**: 83%
- Problem: `wasmModule` and `initPromise` are module-level `let` bindings mutated by `init()` and read by `assertInitialized()`. While this works for the singleton pattern, it makes testing difficult (no way to reset state between tests without dynamic re-import) and prevents running multiple isolated WASM instances.
- Fix: Consider encapsulating in a class or a factory-with-closure that exposes a `reset()` for testing:
```typescript
// Minimal: export a _resetForTesting() guarded by NODE_ENV
export function _resetForTesting(): void {
  if (process.env['NODE_ENV'] !== 'test') return;
  wasmModule = undefined;
  initPromise = null;
}
```
This is a design suggestion rather than a strict type-safety issue.

**`isMdsError` type guard relies on unsafe `as` cast internally** - `packages/mds/src/types.ts:47`
**Confidence**: 85%
- Problem: `(err as MdsError).code` uses a type assertion inside the guard. While the guard overall is correct (returns `err is MdsError`), the internal cast means the guard does not actually verify `help` or `span` fields. If callers rely on the narrowed type to access `.span` without a null check, they get `undefined` at runtime despite types suggesting it exists. However, `span` and `help` are already optional (`?`), so this is safe in practice.
- Fix: Strengthen the guard to verify the discriminant more robustly (minor improvement):
```typescript
export function isMdsError(err: unknown): err is MdsError {
  return (
    err instanceof Error &&
    'code' in err &&
    typeof (err as { code: unknown }).code === 'string'
  );
}
```
Using `'code' in err` before the cast is more idiomatic TypeScript narrowing.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Duplicated `varsOpt` helper across two files** - `packages/mds/src/backend/native.ts:22`, `packages/mds/src/backend/wasm.ts:98`
**Confidence**: 80%
- Problem: The identical `varsOpt` function is defined in both `native.ts` and `wasm.ts`. While small, this violates DRY and creates a drift risk if one is modified independently (e.g., adding field transformations).
- Fix: Extract to a shared utility:
```typescript
// src/util/options.ts
import type { CompileOptions, FileOptions } from '../types.js';
export function varsOpt(options?: CompileOptions | FileOptions): { vars: Record<string, unknown> } | undefined {
  return options?.vars !== undefined ? { vars: options.vars } : undefined;
}
```

## Pre-existing Issues (Not Blocking)

No pre-existing TypeScript issues found (all code is new on this branch).

## Suggestions (Lower Confidence)

- **Consider branded type for file paths** - `packages/mds/src/util/module-scanner.ts:88` (Confidence: 65%) -- `entryPath: string` could be a branded `AbsolutePath` or `FilePath` type to prevent accidentally passing relative paths at call sites, though `resolve()` normalizes it internally.

- **`_init` uses `node:module` unconditionally** - `packages/mds/src/backend/wasm.ts:55` (Confidence: 70%) -- The `_init` function always imports `node:module` and uses `createRequire`, making it Node-specific. The PR description states this file serves browser environments too (via `browser.ts` calling `wasmInit`). In browser bundles, this import path would fail. This may be intentional (browser path uses different init), but the types allow calling `_init` from any context.

- **No `readonly` on interface fields that should be immutable** - `packages/mds/src/types.ts:1-9` (Confidence: 62%) -- `CompileResult.output`, `CompileResult.warnings`, etc. could be `readonly` to signal immutability, aligning with the project's "immutable by default" principle.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The TypeScript implementation is solid overall: strict tsconfig with `noUncheckedIndexedAccess`, proper `type` imports, good use of interfaces for dependency injection, and well-typed discriminated unions. The main issues are the unsafe env-var cast (should validate at boundary) and a missing return type annotation. No `any` types are used anywhere, which is excellent. The singleton/mutable-state pattern is a pragmatic trade-off documented with clear comments. Addressing the HIGH-severity env-var validation issue before merge would bring this to production quality.

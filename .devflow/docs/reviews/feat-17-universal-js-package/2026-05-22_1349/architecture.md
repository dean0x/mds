# Architecture Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22T13:49

## Issues in Your Changes (BLOCKING)

### HIGH

**Module-level mutable singletons in WASM backend violate DIP and testability** - `packages/mds/src/backend/wasm.ts:27-29`
**Confidence**: 85%
- Problem: The WASM backend uses module-level `let wasmModule` and `let initPromise` singletons. This means: (1) the module cannot be tested in isolation without resetting global state, (2) two independent consumers in the same process cannot use different WASM configurations, and (3) there is a hidden coupling between `init()` and `createWasmBackend()` through shared mutable state. The native backend (`createNativeBackend`) correctly uses pure dependency injection — the WASM backend does not follow the same pattern.
- Fix: Move WASM state into the returned backend object or accept the WASM module as a parameter to `createWasmBackend()`, mirroring the native backend's injection pattern:
```typescript
export async function createWasmBackend(options?: InitOptions): Promise<MdsBackend> {
  const wasmModule = await loadWasmModule(options);
  return {
    compile(source, opts) { return wasmModule.compile(source, { ... }); },
    // ...
  };
}
```

**Duplicated initialization logic between browser.ts and wasm.ts** - `packages/mds/src/browser.ts:24-51`, `packages/mds/src/backend/wasm.ts:27-49`
**Confidence**: 82%
- Problem: Both `browser.ts` and `backend/wasm.ts` independently manage init state with their own `_initPromise`/`initPromise` variables and retry-on-failure logic. `browser.ts` calls `wasmInit(options)` then `createWasmBackend()`, but `createWasmBackend()` also calls `init()` internally (line 115). This double-layered init is fragile — the browser entry point manages its own promise while the underlying backend also manages its own. The two layers duplicate the "cache promise, reset on failure" pattern.
- Fix: Consolidate init management into one layer. Either `browser.ts` fully owns the lifecycle and `createWasmBackend` accepts an already-loaded module, or `createWasmBackend` fully owns init and `browser.ts` simply delegates. The current split ownership is a source of bugs.

### MEDIUM

**node.ts uses top-level await with backend selection — module import becomes fallible** - `packages/mds/src/node.ts:14-39`
**Confidence**: 85%
- Problem: Backend selection happens at module import time via top-level await. If both native and WASM fail, the module import itself throws. Consumers have no way to gracefully handle this — a failed `import('@mds/mds')` is not retryable in most bundlers and runtimes. This is a "fail-closed" design at the wrong boundary. The initialization concern is mixed with the module definition concern.
- Fix: Consider a lazy initialization pattern where the backend is resolved on first use, or provide an explicit `init()` that returns a Result-like value. This would separate "loading the API surface" from "connecting to a backend." The browser.ts already uses this pattern — node.ts could follow suit for consistency.

**`isMdsError` type guard is too broad — any Error with a `code` string matches** - `packages/mds/src/types.ts:46-48`
**Confidence**: 83%
- Problem: The guard `err instanceof Error && typeof (err as MdsError).code === 'string'` will match any Node.js system error (which all have `.code` as string, e.g., `ENOENT`, `EACCES`). This means `isMdsError(new Error('file not found'))` returns false, but `fsError` (which has `.code = 'ENOENT'`) would return true. The type guard does not discriminate MDS errors from system errors.
- Fix: Add a discriminant field (e.g., `__mds: true` or a specific code prefix check) to ensure the guard only matches errors actually produced by the MDS backends:
```typescript
export function isMdsError(err: unknown): err is MdsError {
  return err instanceof Error 
    && typeof (err as MdsError).code === 'string'
    && (err as MdsError).code.startsWith('mds::');
}
```

**WASM `_init` uses `node:module` import unconditionally — breaks browser usage** - `packages/mds/src/backend/wasm.ts:55-56`
**Confidence**: 80%
- Problem: The `_init` function always does `await import('node:module')` and `createRequire(import.meta.url)`. This code path will fail in browser environments because `node:module` does not exist. The comment says "In Node.js: load the built WASM module" but the browser entry point also imports from `./backend/wasm.js`. The browser.ts calls `wasmInit(options)` which hits this same `_init` path.
- Fix: The WASM init should either accept an environment parameter, use conditional platform detection, or the browser entry should use a separate browser-specific WASM loader that does not go through `_init`. The current design assumes Node.js in the shared module.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Constants duplicated between wasm.ts and module-scanner.ts with only a comment as link** - `packages/mds/src/backend/wasm.ts:12-13`, `packages/mds/src/util/module-scanner.ts:5-6`
**Confidence**: 85%
- Problem: `WASM_MAX_MODULES = 256` and `WASM_MAX_AGGREGATE_SIZE = 10 * 1024 * 1024` are defined in wasm.ts with a comment "Must match DEFAULT_MAX_MODULES and DEFAULT_MAX_AGGREGATE_SIZE in module-scanner.ts". Comments are not enforced — if one changes without the other, behavior silently diverges.
- Fix: Export the constants from `module-scanner.ts` and import them in `wasm.ts`, or define them in a shared config module.

**`buildFileModules` re-imports module-scanner on every file operation** - `packages/mds/src/backend/wasm.ts:102-109`
**Confidence**: 80%
- Problem: `buildFileModules` does a dynamic `await import('../util/module-scanner.js')` on every call to `compileFile` or `checkFile`. While JavaScript engines cache dynamic imports, this is an unnecessary indirection that obscures the dependency graph. The static dependency is always needed when the WASM backend is used with file operations.
- Fix: Use a static import at the top of the file:
```typescript
import { buildModulesMap } from '../util/module-scanner.js';
```
The dynamic import pattern is only justified if the module has heavy side effects or is conditionally needed. Here it is always needed when `compileFile`/`checkFile` is called.

## Pre-existing Issues (Not Blocking)

No pre-existing issues identified (this is a new package).

## Suggestions (Lower Confidence)

- **`CompileOptions` and `FileOptions` are identical interfaces** - `packages/mds/src/types.ts:11-17` (Confidence: 70%) — Both have only `vars?: Record<string, unknown>`. Consider whether `FileOptions` should extend `CompileOptions` or if they should be merged until they actually diverge. Separate types that are identical create maintenance burden without benefit.

- **Module scanner `projectRoot` is entry file's parent, not a configurable project root** - `packages/mds/src/util/module-scanner.ts:97` (Confidence: 65%) — Using `dirname(absoluteEntry)` as the project root means imports are sandboxed to the entry file's immediate directory. This may be too restrictive for projects with shared library directories at a higher level.

- **Native backend wraps sync napi calls in async functions that return immediately-resolved promises** - `packages/mds/src/backend/native.ts:42-48` (Confidence: 72%) — `compileFile` and `checkFile` are async but the napi addon calls are synchronous. This could mislead consumers into thinking I/O is non-blocking when it actually blocks the event loop. Consider documenting this or having the interface distinguish sync vs async backends.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 3 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The overall architecture is sound — the Strategy pattern (MdsBackend interface) with backend-specific adapters is the right approach for a universal package. Dependency injection is correctly applied in the native backend. The layering (types -> backends -> entry points) is clean. However, the WASM backend's singleton state management and duplicated init logic between layers undermine testability and create subtle coupling. The `isMdsError` guard being over-broad could lead to incorrect error classification at runtime. These should be addressed before merge.

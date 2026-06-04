# TypeScript Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Cycle**: 4 (incremental after 3 prior cycles; 19/21 resolved)

## Issues in Your Changes (BLOCKING)

### HIGH

**JSDoc contradicts implementation in tryLoadCandidate** - `packages/mds/src/backend/wasm.ts:75-96`
**Confidence**: 95%
- Problem: The JSDoc states "Re-throws unexpected errors so the caller can surface them" but the catch block catches ALL errors and returns `null`. A WASM initialization error (e.g., `mod.default(wasmUrl)` throwing due to corrupt WASM binary, out-of-memory, or incompatible WASM version) is silently swallowed and treated as "candidate not found". This causes the generic "failed to load WASM module. Build it first..." error message to surface instead of the real underlying error.
- Impact: Debugging WASM init failures becomes significantly harder. A user with a correctly located but corrupt WASM binary gets a misleading "build it first" error instead of the actual failure.
- Fix: Distinguish between module-not-found errors (return null) and unexpected errors (re-throw), matching the JSDoc contract:
```typescript
async function tryLoadCandidate(
  candidate: string,
  require: NodeRequire,
  wasmUrl: InitOptions['wasmUrl'],
): Promise<WasmModule | null> {
  try {
    const mod = require(candidate) as WasmModule;
    if (typeof mod.default === 'function') {
      await mod.default(wasmUrl);
    }
    return mod;
  } catch (err: unknown) {
    // Module not found — try next candidate.
    if (err instanceof Error && 'code' in err && (err as NodeJS.ErrnoException).code === 'MODULE_NOT_FOUND') {
      return null;
    }
    // Unexpected error — surface it.
    throw err;
  }
}
```

### MEDIUM

**compileOpts return type is manually spelled out instead of inferred or derived** - `packages/mds/src/backend/wasm.ts:144`
**Confidence**: 82%
- Problem: The return type `{ filename: string; modules: Record<string, string>; vars?: Record<string, unknown> }` is manually written and duplicates structure already defined elsewhere. The WASM `compile` and `check` methods on the `WasmModule` interface (line 19) already define the options shape as `{ filename?: string; modules?: Record<string, string>; vars?: Record<string, unknown> }`. The manual spelling creates a maintenance risk if the two diverge.
- Fix: Either rely on TypeScript inference (remove the explicit return type) or derive the type from the WasmModule interface:
```typescript
function compileOpts(options?: CompileOptions) {
  const vars = varsOpt(options);
  return vars !== undefined ? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS;
}
```
TypeScript will infer the return type correctly from the `DEFAULT_COMPILE_OPTS` shape and the `varsOpt` return type, keeping it in sync automatically.

**`as WasmModule` type assertion in tryLoadCandidate bypasses type safety** - `packages/mds/src/backend/wasm.ts:87`
**Confidence**: 80%
- Problem: `require(candidate) as WasmModule` is a type assertion on an `unknown` return from `require()`. If the loaded module does not conform to the `WasmModule` interface (e.g., missing `compile`, `check`, or `scanImports`), the assertion silently passes and the error surfaces later at call-time with a confusing "not a function" error.
- Fix: Add a minimal runtime shape check after loading:
```typescript
const mod = require(candidate) as Record<string, unknown>;
if (typeof mod.compile !== 'function' || typeof mod.check !== 'function' || typeof mod.scanImports !== 'function') {
  return null; // Not a valid WasmModule — try next candidate
}
```
This pattern follows the project's principle of "parse at boundaries, trust internally" and converts the unsafe assertion to a validated narrowing.

## Issues in Code You Touched (Should Fix)

_None identified._

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`as object` and `as Parameters<...>` assertions in node.ts** - `packages/mds/src/node.ts:27-29`
**Confidence**: 85%
- Problem: The native addon loaded via `require('mds-napi')` is cast with `as object` and then `as Parameters<typeof createNativeBackend>[0]` without runtime validation. This follows the same pattern as the `as WasmModule` assertion in wasm.ts. Neither boundary validates the loaded module's shape at runtime.
- Impact: If `mds-napi` changes its export shape, the error will be a confusing runtime crash rather than a clear validation failure.

## Suggestions (Lower Confidence)

- **Consider `Readonly` annotation on `DEFAULT_COMPILE_OPTS` type** - `packages/mds/src/backend/wasm.ts:138-141` (Confidence: 65%) -- While `Object.freeze` enforces immutability at runtime, the TypeScript type of `DEFAULT_COMPILE_OPTS` is inferred as `Readonly<{...}>` which only makes the top level readonly. The inner `modules` is `Object.freeze`d at runtime but its type after the outer freeze is `Readonly<Record<string, string>>`. The `compileOpts` function then spreads this into a return type that declares `modules: Record<string, string>` (mutable). This is not a bug since the frozen object is still frozen, but using `as const` would make the type-level immutability more explicit.

- **`_resetForTesting` exported without conditional compilation** - `packages/mds/src/backend/wasm.ts:41-45` (Confidence: 70%) -- The `_resetForTesting` function is exported unconditionally and appears in the production `.d.ts` declarations. While the `@internal` tag and underscore prefix signal intent, TypeScript's `@internal` tag has no enforcement. Consumers can import and call it in production code. Consider isolating it behind a test-only import path or at minimum documenting it in the package's public API exclusion list.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The codebase demonstrates strong TypeScript fundamentals: strict mode with `noUncheckedIndexedAccess` enabled, proper `import type` usage, discriminated union patterns, and well-typed interfaces. The main actionable finding is the `tryLoadCandidate` catch-all that contradicts its own JSDoc and silently swallows WASM initialization errors, which should be addressed before merge. The type assertion concerns are lower severity but worth addressing for boundary safety consistency.

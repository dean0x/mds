# TypeScript Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20

## Issues in Your Changes (BLOCKING)

### HIGH

**Shallow-frozen `DEFAULT_COMPILE_OPTS` exposes mutable nested object to FFI boundary** - `packages/mds/src/backend/wasm.ts:106`
**Confidence**: 85%
- Problem: `Object.freeze()` is shallow. The nested `modules: {}` object inside `DEFAULT_COMPILE_OPTS` is NOT frozen. On the no-vars path (lines 117, 123), this frozen object is passed directly to `wasm.compile()`/`wasm.check()` without spreading. If the WASM FFI binding mutates the `modules` property (e.g., populates it during compilation), all subsequent no-vars calls share a corrupted modules map. Additionally, the `as Record<string, string>` cast overrides the narrower type that `Object.freeze` would infer, weakening type safety at the FFI boundary.
- Fix: Either deep-freeze the nested object, or always spread into a fresh object:
```typescript
// Option A: freeze the nested object too
const DEFAULT_COMPILE_OPTS = Object.freeze({
  filename: 'input.mds',
  modules: Object.freeze({} as Record<string, string>),
});

// Option B: always spread (avoids mutation risk entirely)
compile(source: string, options?: CompileOptions): CompileResult {
  const wasm = assertInitialized();
  const vars = varsOpt(options);
  return wasm.compile(source, vars !== undefined
    ? { ...DEFAULT_COMPILE_OPTS, ...vars }
    : { ...DEFAULT_COMPILE_OPTS });
},
```
Option A is preferred -- it maintains the allocation-avoidance intent while preventing mutation at the type level (`Readonly` propagates to nested frozen objects).

### MEDIUM

**Type assertion `as Record<string, string>` on frozen empty object** - `packages/mds/src/backend/wasm.ts:106`
**Confidence**: 82%
- Problem: `{} as Record<string, string>` is a type assertion that bypasses TypeScript's structural type inference. The inferred type of `{}` is `{}` (an empty object literal), and asserting it as `Record<string, string>` tells the compiler "trust me" without validation. While functionally correct here (an empty object does satisfy `Record<string, string>` at runtime), this pattern undermines the type safety principle. Combined with `Object.freeze`, the outer type becomes `Readonly<{ filename: string; modules: Record<string, string> }>` rather than `Readonly<{ filename: string; modules: Readonly<Record<string, string>> }>`, so the compiler won't flag code that mutates `modules`.
- Fix: Use a typed const with satisfies, or annotate the variable:
```typescript
const EMPTY_MODULES: Readonly<Record<string, string>> = {};
const DEFAULT_COMPILE_OPTS = Object.freeze({ filename: 'input.mds', modules: EMPTY_MODULES });
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`WasmModule` interface uses `unknown` in `default?` parameter** - `packages/mds/src/backend/wasm.ts:22` (Confidence: 65%) -- The `default?: (input?: unknown) => Promise<void>` parameter type could be narrowed to match `InitOptions['wasmUrl']` for better type documentation at the FFI boundary, since `options?.wasmUrl` is the only value ever passed.

- **`loadError` typed as `unknown` but stringified without narrowing** - `packages/mds/src/backend/wasm.ts:74,92` (Confidence: 62%) -- `loadError` is `unknown` and used via `String(loadError)` on line 92. While `String()` handles any type at runtime, the pattern of catching as `unknown` and then using it without narrowing is a minor type discipline gap. Consider `err instanceof Error ? err.message : String(err)` for richer error messages.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Notes

**Cross-cycle awareness**: The `varsOpt` null passthrough fix (loose equality `!= null`) from Cycle 2 is confirmed correctly applied. The `normalizeVirtualKey` refactoring is clean. The frozen default options pattern is new in this cycle and introduces the primary finding above.

**Positive observations**:
- Strict tsconfig (`strict: true`, `noUncheckedIndexedAccess: true`) is correctly configured
- Proper `import type` usage throughout all changed files
- Well-typed interfaces (`WasmModule`, `MdsBackend`, `ModuleScannerOptions`)
- Discriminated return types on `varsOpt` (`{ vars: ... } | undefined`) are clean
- The `isMdsError` type guard uses proper narrowing pattern (`err is MdsError`)
- `MdsErrorSpan` additions (line/column docs) are well-typed with optional modifiers
- The `depth` parameter with default value in `scan()` is well-typed
- No `any` types in changed code
- `MAX_IMPORT_DEPTH` as a module-level constant is good practice

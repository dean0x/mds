# TypeScript Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Dead variable reference in test** - `packages/mds/__test__/backend.spec.mjs:46`
**Confidence**: 95%
- Problem: Line 46 assigns `const script = path.join(__dirname, 'backend-wasm-helper.mjs')` but the variable `script` is never used, and the file `backend-wasm-helper.mjs` does not exist on disk. This is dead code that makes the test harder to understand -- it looks like a leftover from a previous iteration that used a helper script instead of inline `--input-type=module`.
- Fix: Remove the dead line:
```javascript
// Remove this line:
const script = path.join(__dirname, 'backend-wasm-helper.mjs');
```

**`varsOpt` passes through `null` vars without validation** - `packages/mds/src/util/options.ts:11`
**Confidence**: 85%
- Problem: The `varsOpt` function checks `options?.vars !== undefined` but does not guard against `null`. The `CompileOptions.vars` type is `Record<string, unknown> | undefined` (no `null` in the union), but at runtime a JS caller can pass `{ vars: null }`. Since `null !== undefined`, `varsOpt` returns `{ vars: null }`, forwarding a `null` where the backend expects `Record<string, unknown>`. Test `U-C7` explicitly validates this case works, but the function should either reject `null` or normalize it to `undefined`. This is a boundary validation gap.
- Fix:
```typescript
export function varsOpt(options?: CompileOptions | FileOptions): { vars: Record<string, unknown> } | undefined {
  return options?.vars != null ? { vars: options.vars } : undefined;
  //                  ^^ loose equality catches both null and undefined
}
```

### MEDIUM

**`normalizeVirtualKey` does not normalize `..` in root-entry path** - `packages/mds/src/util/module-scanner.ts:34-43`
**Confidence**: 82%
- Problem: When `base.length === 0` (root entry point), the function returns `relative` as-is after only counting segments. It does not resolve `.` or `..` segments. If the Rust `VirtualFs::normalize()` does resolve these segments in the empty-base case, the JS and Rust implementations will diverge, causing mismatched virtual keys and failed import resolution. The PR description specifically flags that `normalizeVirtualKey()` must exactly mirror the Rust implementation.
- Fix: Verify against the Rust `VirtualFs::normalize()` implementation. If Rust normalizes `..`/`.` even for root entries, apply the same segment-walking logic here instead of returning `relative` raw. If Rust also returns as-is, add a comment documenting parity.

**Stale npm script reference** - `packages/mds/package.json:28`
**Confidence**: 90%
- Problem: The `test:parity` script still references `__test__/parity.spec.mjs`, but that file was renamed to `native-backend.spec.mjs` in this PR. Running `npm run test:parity` will fail with a "file not found" error.
- Fix:
```json
"test:parity": "node --test __test__/native-backend.spec.mjs"
```
Or rename the script to `test:native` for consistency with the file rename:
```json
"test:native": "node --test __test__/native-backend.spec.mjs"
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**browser.ts `init` does not mirror wasm.ts retry limit** - `packages/mds/src/browser.ts:33-51`
**Confidence**: 80%
- Problem: `wasm.ts` now has a `MAX_INIT_RETRIES` (3) guard that prevents retrying after repeated failures. However, `browser.ts` has its own `initPromise` caching layer with no retry limit. After 3 failures, each call to `browser.ts#init()` still creates a new `doInit()` promise only to be immediately rejected by `wasm.ts`. This is functionally correct but wasteful -- it creates unnecessary promise objects and runs `doInit`'s try/catch on every call after the limit is reached. Consider adding a parallel failure count or caching the terminal error.
- Fix: Add a cached terminal error in browser.ts:
```typescript
let terminalError: Error | null = null;

export async function init(options?: InitOptions): Promise<void> {
  if (backend !== undefined) return;
  if (terminalError !== null) throw terminalError;
  if (initPromise !== null) return initPromise;
  initPromise = doInit(options);
  return initPromise;
}

async function doInit(options?: InitOptions): Promise<void> {
  try {
    await wasmInit(options);
    const { createWasmBackend } = await import('./backend/wasm.js');
    backend = await createWasmBackend();
  } catch (err) {
    initPromise = null;
    if (err instanceof Error && err.message.includes('failed to initialize after')) {
      terminalError = err;
    }
    throw err;
  }
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`isMdsError` type guard uses repeated `as` casts** - `packages/mds/src/types.ts:71-76`
**Confidence**: 82%
- Problem: The type guard casts `err as MdsError` three separate times. While functionally correct, this pattern is fragile -- if `MdsError` gains new required properties, each cast silently lies to the compiler. A single narrowing variable or intermediate object would be cleaner and more maintainable.
- Suggested improvement (non-blocking):
```typescript
export function isMdsError(err: unknown): err is MdsError {
  if (!(err instanceof Error)) return false;
  const record = err as Record<string, unknown>;
  return typeof record.code === 'string' && (record.code as string).startsWith('mds::');
}
```

## Suggestions (Lower Confidence)

- **`CompileOptions` and `FileOptions` are structurally identical** - `packages/mds/src/types.ts:18-27` (Confidence: 65%) -- Both interfaces have the exact same shape (`{ vars?: Record<string, unknown> }`). Consider whether a single `MdsOptions` type with a type alias would reduce duplication. However, having separate types may be intentional for future divergence.

- **`WasmModule.default` typed as `(input?: unknown) => Promise<void>`** - `packages/mds/src/backend/wasm.ts:25` (Confidence: 70%) -- The `input` parameter is actually `InitOptions.wasmUrl` which has a specific union type. Typing it as `unknown` loses information, though since this is an external module interface the imprecision may be unavoidable.

- **`node.ts` uses top-level `await` for backend initialization** - `packages/mds/src/node.ts:20-21` (Confidence: 60%) -- Top-level `await` means any import of this module blocks until the backend is loaded. This is likely intentional for the Node.js entry point, but worth documenting as a deliberate choice since it affects module loading behavior.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | - | 0 | 1 | 0 |
| Pre-existing | - | - | 1 | 0 |

**TypeScript Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The TypeScript code is generally well-structured with proper strict mode, good use of `type`-only imports, no `any` types, and clean discriminated unions. The main concerns are: (1) a boundary validation gap where `null` vars pass through to backends unchecked, (2) a stale npm script that will break after the file rename, (3) a dead variable in the test suite, and (4) potential Rust parity divergence in `normalizeVirtualKey` for the empty-base case that the PR author specifically flagged for review. The tsconfig is properly strict with `noUncheckedIndexedAccess` enabled.

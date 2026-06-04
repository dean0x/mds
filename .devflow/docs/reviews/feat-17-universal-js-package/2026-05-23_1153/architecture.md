# Architecture Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Cycle**: 4 (incremental from cycle 3 — 19/21 fixed, 1 FP, 1 deferred)

## Issues in Your Changes (BLOCKING)

### HIGH

**tryLoadCandidate swallows all errors indiscriminately** - `packages/mds/src/backend/wasm.ts:93`
**Confidence**: 90%
- Problem: The extracted `tryLoadCandidate` function catches *all* exceptions and returns `null`, despite the JSDoc claiming "Re-throws unexpected errors so the caller can surface them." In practice, if `require(candidate)` succeeds but `mod.default(wasmUrl)` throws (e.g., a corrupted WASM binary, an OOM during instantiation, or a network error fetching the wasmUrl), the caller silently moves to the next candidate and ultimately throws a generic "failed to load WASM module" error. This hides the real failure cause, making production debugging very difficult.
- Impact: The JSDoc-behavior mismatch is also an architectural concern: the function's contract says one thing but implements another. Callers reason about a re-throw path that does not exist. This is a Leaky Abstraction / broken contract.
- Fix: Distinguish "not found" (MODULE_NOT_FOUND) from other errors:
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
    // MODULE_NOT_FOUND means this candidate path doesn't exist — try the next.
    if (
      err instanceof Error &&
      'code' in err &&
      (err as NodeJS.ErrnoException).code === 'MODULE_NOT_FOUND'
    ) {
      return null;
    }
    // All other errors (WASM init failure, OOM, etc.) are unexpected — re-throw.
    throw err;
  }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**browser.ts retry semantics partially broken after removing catch-and-reset** - `packages/mds/src/browser.ts:44-46`
**Confidence**: 82%
- Problem: The change removes the `.catch()` handler that resets `initVoidPromise = null` on failure. The comment explains this is intentional — wasm.ts owns retry logic. However, wasm.ts's retry logic works by clearing `initPromise` inside its own `init()` function (wasm.ts:69), allowing subsequent calls to create a new promise. Meanwhile browser.ts caches `initVoidPromise` forever once set. After a transient failure, calling `browser.init()` again returns the same permanently-rejected promise and never re-enters `createWasmBackend()`, so wasm.ts's retry counter is never consulted. The wasm.ts retry mechanism is architecturally bypassed.
- Impact: In browser environments, a single transient WASM load failure (e.g., network timeout) permanently bricks the SDK for the page lifetime. The user would need a full page reload. The prior resolution notes this was deferred as "node.ts/browser.ts LSP tension" — this is likely the same issue.
- Fix: Either (a) restore the catch-and-reset so browser.ts re-enters createWasmBackend on retry, or (b) have browser.ts call wasm.init() directly (which already handles retry) and only wrap the backend-creation on success:
```typescript
export function init(options?: InitOptions): Promise<void> {
  if (resolvedBackend !== undefined) return Promise.resolve();
  if (initVoidPromise !== null) return initVoidPromise;
  initVoidPromise = createWasmBackend(options)
    .then((b) => {
      resolvedBackend = b;
    })
    .catch((err) => {
      initVoidPromise = null; // allow retry — wasm.ts enforces MAX_INIT_RETRIES
      throw err;
    });
  return initVoidPromise;
}
```

### MEDIUM

**Inconsistent options construction between compile/check and compileFile/checkFile** - `packages/mds/src/backend/wasm.ts:155-182`
**Confidence**: 85%
- Problem: `compile()` and `check()` use the new `compileOpts()` helper which deep-freezes defaults and provides consistent object shape. But `compileFile()` and `checkFile()` (lines 168-172, 178-182) manually construct options with `{ filename, modules, ...varsOpt(options) }` — bypassing `compileOpts()` entirely. This creates two divergent code paths for the same "build WASM options" responsibility. The asymmetry means any future change to option construction (e.g., adding new default fields) must be replicated in two places.
- Impact: SRP violation — option construction logic is split across `compileOpts` and inline code in `compileFile/checkFile`. The `compileFile/checkFile` path does not get the benefit of frozen defaults. This is minor today but will cause drift as options evolve.
- Fix: Either extend `compileOpts` to accept optional filename/modules overrides, or extract a shared builder:
```typescript
function buildWasmOpts(
  options?: CompileOptions | FileOptions,
  overrides?: { filename: string; modules: Record<string, string> },
): { filename: string; modules: Record<string, string>; vars?: Record<string, unknown> } {
  const vars = varsOpt(options);
  const base = overrides ?? DEFAULT_COMPILE_OPTS;
  return vars !== undefined ? { ...base, ...vars } : { ...base };
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**node.ts uses top-level await with multi-step imperative fallback logic** - `packages/mds/src/node.ts:19-44`
**Confidence**: 80%
- Problem: The node.ts entry point performs backend selection via deeply nested try/catch with mutable state (`nativeErr`, `backend`) at the module top level. This makes the initialization sequence hard to reason about and test. The native backend adapter uses proper DI (injected addon), but node.ts does the injection imperatively in a catch block.
- Impact: Not blocking since it was not modified in this PR. The pattern works but resists testability — you cannot unit-test node.ts backend selection without actually loading native/WASM modules.

## Suggestions (Lower Confidence)

- **CompileOptions and FileOptions are structurally identical** - `packages/mds/src/types.ts:17-27` (Confidence: 70%) — Both interfaces have exactly the same shape (`{ vars?: Record<string, unknown> }`). Consider a type alias `type FileOptions = CompileOptions` to make the equivalence explicit, or differentiate them if they are expected to diverge.

- **_resetForTesting exported from production module** - `packages/mds/src/backend/wasm.ts:41` (Confidence: 65%) — Exporting test-only functions from production modules creates surface area that consumers could accidentally depend on. Consider conditional export or a separate test-helpers module. The `@internal` tag mitigates but does not prevent misuse.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The overall architecture is sound: clear separation between entry points (node.ts, browser.ts), backend adapters (native.ts, wasm.ts), shared types, and utilities. The `MdsBackend` interface provides a clean abstraction boundary, and DI is used correctly in the native backend. The refactorings in this cycle (deep-freeze, `compileOpts`, `statAndValidateModule`, `tryLoadCandidate`) all improve modularity.

The blocking issue is the silent error swallowing in `tryLoadCandidate` which contradicts its documented contract and will hide real initialization failures in production. The browser.ts retry bypass is the most architecturally concerning should-fix — it renders the wasm.ts retry/circuit-breaker mechanism unreachable from browser environments.

# Regression Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T11:53

## Issues in Your Changes (BLOCKING)

### HIGH

**tryLoadCandidate swallows all errors -- JSDoc/implementation mismatch and lost diagnostics** - `packages/mds/src/backend/wasm.ts:75-96`
**Confidence**: 90%
- Problem: The JSDoc on `tryLoadCandidate` (line 79) claims "Re-throws unexpected errors so the caller can surface them" but the `catch` block at line 93-94 catches all exceptions and returns `null`, never re-throwing. This is a behavioral regression from the old code which captured `loadError` and included it in the final error message (`${String(loadError)}`). If a WASM module is found but fails to initialize (e.g., `mod.default(wasmUrl)` throws due to a corrupted WASM binary, incompatible version, or invalid wasmUrl), the error is silently swallowed. The user receives only the generic message "failed to load WASM module. Build it first with: wasm-pack build..." with no root cause information.
- Impact: Debugging WASM initialization failures becomes significantly harder. Previously, the error message included the underlying cause (e.g., "CompileError: WebAssembly.instantiate(): ..." or "TypeError: ..."). Now, all failures produce identical generic messages regardless of whether the module was missing or broken.
- Fix: Either (a) re-add error capture to include the last error in the final throw message, or (b) make the catch clause discriminate between "module not found" errors and unexpected errors:
```typescript
async function tryLoadCandidate(
  candidate: string,
  require: NodeRequire,
  wasmUrl: InitOptions['wasmUrl'],
): Promise<WasmModule | null> {
  let mod: WasmModule;
  try {
    mod = require(candidate) as WasmModule;
  } catch {
    // Module not found -- expected for fallback candidates.
    return null;
  }
  // Module found but initialization may still fail -- let errors propagate.
  if (typeof mod.default === 'function') {
    await mod.default(wasmUrl);
  }
  return mod;
}
```
  Also update the error message in `_init` to restore the diagnostic:
```typescript
  // Track last error for diagnostics
  let lastError: unknown;
  for (const candidate of candidates) {
    try {
      const mod = await tryLoadCandidate(candidate, require, options?.wasmUrl);
      if (mod !== null) { wasmModule = mod; return; }
    } catch (e) { lastError = e; }
  }
  throw new Error(
    `@mds/mds: failed to load WASM module. Build it first with: wasm-pack build crates/mds-wasm --target nodejs --out-dir pkg.${lastError ? ' ' + String(lastError) : ''}`,
  );
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **browser.ts init() no longer resets on failure -- changes retry semantics** - `packages/mds/src/browser.ts:44-46` (Confidence: 65%) -- The old code reset `initVoidPromise = null` on rejection, allowing callers to retry with potentially different options (e.g., a corrected wasmUrl). The new code permanently caches the rejected promise, so even if the transient condition is resolved (e.g., network becomes available), `init()` will always return the same rejected promise until page reload. This was explicitly acknowledged in prior resolutions as intentional (wasm.ts owns retry logic), but browser environments may benefit from the ability to retry init with new options since there is no module-level re-import mechanism in browsers. Flagged as suggestion only since this was a deliberate design choice in cycle 3.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The branch is well-structured with no lost exports, no deleted files, no signature changes, and all 78 tests pass. The only actionable regression is the `tryLoadCandidate` error swallowing, which degrades debuggability when WASM initialization fails for reasons other than "module not found." The JSDoc also contradicts the implementation (claims re-throw, does not re-throw). The browser.ts retry semantics change was an intentional design decision from cycle 3 and is noted as a lower-confidence suggestion only.

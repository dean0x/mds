# Security Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T11:53

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**tryLoadCandidate silently swallows all errors, masking security-relevant failures** - `packages/mds/src/backend/wasm.ts:93`
**Confidence**: 85%
- Problem: The `tryLoadCandidate` function catches all exceptions and returns `null`, treating every failure identically to "module not found." The JSDoc says "Re-throws unexpected errors so the caller can surface them" but the implementation does not — it catches everything. If the WASM module is found but fails to initialize due to a corrupted/tampered WASM binary, a supply-chain attack (modified module export), or a permissions error, the error is silently discarded. The caller moves on to the next candidate, potentially loading a less-trusted fallback, or throws a generic "failed to load" message that hides the root cause.
- Fix: Distinguish "not found" from other errors. Only swallow resolution/not-found errors; re-throw anything else:
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
    // Only swallow MODULE_NOT_FOUND — re-throw everything else
    // (corrupt WASM, permission errors, initialization failures).
    if (
      err instanceof Error &&
      'code' in err &&
      (err as NodeJS.ErrnoException).code === 'MODULE_NOT_FOUND'
    ) {
      return null;
    }
    throw err;
  }
}
```

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**TOCTOU window between statAndValidateModule and readFile** - `packages/mds/src/util/module-scanner.ts:138-208`
**Confidence**: 82%
- Problem: There is a time-of-check-to-time-of-use gap between `statAndValidateModule` (which runs `lstat` + `realpath` at line 139-142) and the subsequent `readFile` at line 208. An attacker with local filesystem access could swap a validated file for a symlink or different file between these calls. The code already documents awareness of this (TOCTOU comments at line 150-155), and the existing mitigations (lstat + realpath comparison) are the standard best-effort approach for Node.js. This is an inherent limitation of the Node.js filesystem API (no `O_NOFOLLOW` + `fstat` on the opened fd). The current defense is the practical maximum for this runtime.
- Note: Pre-existing, not introduced in this PR. The refactoring into `statAndValidateModule` does not change the TOCTOU window size. Documented for completeness.

## Suggestions (Lower Confidence)

- **`_resetForTesting` exported from production module** - `packages/mds/src/backend/wasm.ts:41` (Confidence: 65%) -- The function resets all singleton security state (init promise, failure counter). While clearly marked `@internal` and named with underscore convention, it is a public export that could be called by any consumer to bypass the circuit breaker (MAX_INIT_RETRIES). Consider gating behind `process.env.NODE_ENV === 'test'` or moving to a test-only module. Low risk for a pre-release package with zero users.

- **Error messages include filesystem paths** - `packages/mds/src/util/module-scanner.ts:124,154,161` (Confidence: 60%) -- Security error messages include absolute filesystem paths (e.g., `import path escapes project root: {childAbsolute} is outside {projectRoot}`). In a server-side context, these could leak internal directory structure to end users. Pre-existing pattern, not introduced in this PR.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Security Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The codebase demonstrates strong security practices: path traversal guards, symlink detection, null byte rejection, resource limits (module count, aggregate size, recursion depth), deep-frozen shared state, and a circuit breaker on WASM init retries. The single blocking issue is the overly broad catch in `tryLoadCandidate` which could mask supply-chain or integrity failures during WASM module loading. The fix is straightforward (discriminate MODULE_NOT_FOUND from other errors).

### Cross-Cycle Notes

Prior resolution cycles addressed 19 issues including deep-freezing `DEFAULT_COMPILE_OPTS`, extracting `compileOpts()` helper, adding `_resetForTesting()`, extracting `tryLoadCandidate()` and `statAndValidateModule()`, and removing browser retry reset. The `tryLoadCandidate` error-swallowing issue is new — it was introduced as part of the extraction refactoring and was not flagged in prior cycles.

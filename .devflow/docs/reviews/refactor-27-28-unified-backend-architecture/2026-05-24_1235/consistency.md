# Consistency Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24T12:35
**Diff Range**: c57685c73a1c6c01c12040776659b796eb363827...HEAD (4 commits)

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Test name U-WB13 contradicts actual behavior after refactor** - `packages/mds/__test__/wasm-backend.spec.mjs:152`
**Confidence**: 95%
- Problem: The test was renamed from "tryLoadCandidate rejects modules missing scanImports" to "tryLoadCandidate returns null for modules missing scanImports". However, the implementation change in this same PR replaced the `return null` shape-check in `tryLoadCandidate` with a call to `validateWasmShape(mod)` which *throws*. The new test name claims "returns null" when the code now throws on shape mismatch. The comment at line 154 ("a module without it returns null from tryLoadCandidate") is also inaccurate.
- Fix: Rename the test to reflect the throwing behavior:
```javascript
test('U-WB13: validateWasmShape rejects modules missing scanImports', async () => {
    // This test verifies the shape validation at the boundary.
    // The shape check now requires scanImports; a module without it causes
    // validateWasmShape to throw. We test this indirectly by verifying that a successful
    // initWasmNode() always yields a module with scanImports (the built WASM has it).
```

**Test file header comment range is stale** - `packages/mds/__test__/wasm-backend.spec.mjs:3`
**Confidence**: 95%
- Problem: The file header says "Tests: U-WB1 through U-WB13" but this PR adds tests U-WB14 through U-WB20. Every other test file in this codebase (e.g., `backend.spec.mjs` line 3: "Tests: U-B1 through U-B11") maintains an accurate range comment at the top. Leaving this stale breaks the documentation pattern.
- Fix:
```javascript
/**
 * WASM backend unit tests for @mds/mds universal package.
 * Tests: U-WB1 through U-WB20
 *
 * Imports dist/backend/wasm.js directly to exercise internal state
 * without going through the full node.ts entry point.
 */
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**assertReady error message phrasing inconsistency between node.ts and browser.ts** - `packages/mds/src/browser.ts:73`, `packages/mds/src/node.ts:172`
**Confidence**: 85%
- Problem: The PR standardized the JSDoc phrasing to "Requires init() to have been called and awaited first" across both entry points. However, the runtime error messages that users actually see still diverge:
  - browser.ts: `@mds/mds: call init() before using compile/check in a browser environment`
  - node.ts: `@mds/mds: call await init() before using compile/check/compileFile/checkFile/getBackend`
  
  The node.ts message says `await init()` while browser.ts says just `init()`. The JSDoc was explicitly standardized in this PR to say "init() to have been called and awaited" so the runtime messages should match the same guidance pattern (both should mention `await` or neither should).
- Fix: Update browser.ts error message to match the pattern:
```typescript
function assertReady(): MdsBaseBackend {
  if (resolvedBackend === undefined) {
    throw new Error('@mds/mds: call await init() before using compile/check in a browser environment');
  }
  return resolvedBackend;
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Inconsistent handle cleanup patterns in scan()** - `packages/mds/src/util/module-scanner.ts:247-258`
**Confidence**: 80%
- Problem: The `scan()` function uses two different cleanup patterns for the file handle returned by `openAndValidateModule`: (1) an explicit `await handle.close()` at line 248 for the aggregate size error path, and (2) a `try/finally` block at lines 255-259 for the read path. These two cleanup styles in the same function are inconsistent. While functionally safe (the code between handle acquisition at line 239 and the first close path is synchronous), the mixed idiom makes the cleanup contract harder to follow.
- Note: Fixing this in this PR would mean wrapping the entire post-`openAndValidateModule` block in a single `try/finally`, which is a reasonable cleanup but not introduced by these changes.

## Suggestions (Lower Confidence)

- **Circuit breaker error message verbs differ** - `packages/mds/src/backend/wasm.ts:157,232` (Confidence: 60%) -- `initWasmNode` says "Check that the WASM module is built and accessible" while `initWasmBrowser` says "Ensure 'mds-wasm' is bundled...". The structural pattern is consistent but verb choice ("Check" vs "Ensure") diverges slightly. Minor style observation.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The changes demonstrate strong consistency overall: the guard function rename from `assertInitialized` to `assertReady` correctly matches the existing `assertReady` in node.ts; the JSDoc phrasing was standardized across all exports; the `_resetForTesting` API extension is backward-compatible; the circuit breaker pattern in `initWasmBrowser` mirrors `initWasmNode`; and the `validateWasmShape` extraction follows the project's pattern of dedicated validation helpers. The two blocking MEDIUM issues are documentation/naming drift that should be corrected before merge to avoid confusion in future maintenance.

# Testing Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24T12:35

## Issues in Your Changes (BLOCKING)

### HIGH

**U-WB13 test name is misleading after behavioral change** - `packages/mds/__test__/wasm-backend.spec.mjs:152`
**Confidence**: 90%
- Problem: The test is named "tryLoadCandidate returns null for modules missing scanImports" but `tryLoadCandidate` no longer returns null for shape validation failures -- it now throws via `validateWasmShape`. The inline `validateWasmShape` call replaced the conditional null-return. The test body only checks that a successful `initWasmNode()` yields a module with `scanImports`, which is an indirect happy-path assertion that does not exercise the error path at all. The test name describes behavior that no longer exists.
- Fix: Rename the test to reflect what it actually validates (e.g., "initWasmNode() only succeeds when scanImports is present") or, better, add a direct test that calls `validateWasmShape` with a module missing `scanImports` and asserts the throw (this is already covered by U-WB20, making U-WB13 redundant -- consider removing it or narrowing its name to "initWasmNode() returns module with scanImports").

### MEDIUM

**U-WB14/U-WB15/U-WB16 browser circuit breaker tests verify the gate but not the failure-counting path** - `packages/mds/__test__/wasm-backend.spec.mjs:178-230`
**Confidence**: 82%
- Problem: These tests pre-seed the failure counter via `_resetForTesting` and verify the circuit-breaker gate fires or does not fire. However, there is no test that verifies `browserFailures` is actually incremented when `initWasmBrowser()` fails (i.e., the `.catch` handler at wasm.ts:237 that does `browserFailures += 1`). The Node.js side has analogous coverage because `initWasmNode` retries with real candidate paths, but the browser path short-circuits with a module-not-found error so the increment is never exercised end-to-end. If the `+= 1` line were deleted, all existing tests would still pass.
- Fix: Add a test that calls `initWasmBrowser()` (which will fail because `mds-wasm` is not available in Node.js test environment), then calls it again, and verifies the failure count incremented (the second call should still not hit the circuit breaker since count would be 1 < 3). This would confirm the catch handler integrates correctly with the counter.

**File header comment lists wrong test range** - `packages/mds/__test__/wasm-backend.spec.mjs:3`
**Confidence**: 95%
- Problem: The JSDoc header says "Tests: U-WB1 through U-WB13" but the file now contains tests U-WB1 through U-WB20. Stale documentation creates confusion about test coverage.
- Fix: Update to "Tests: U-WB1 through U-WB20".

## Issues in Code You Touched (Should Fix)

### MEDIUM

**No tests for the scanner's refactored aggregate-size-before-read path** - `packages/mds/src/util/module-scanner.ts:239-259`
**Confidence**: 85%
- Problem: The scanner was refactored to split `openAndValidateModule` into returning a handle+size (without reading content) so that the aggregate size check happens before `readFile`. This is a security-relevant behavioral change (prevents memory allocation of content that will be rejected), but `scanner.spec.mjs` has no test for `maxAggregateSize` at all. The existing `U-SM5` test only covers `maxModules`. The new two-phase open-then-read pattern, and particularly the handle cleanup on aggregate-size rejection (line 248: `await handle.close()`), is untested.
- Fix: Add a test in `scanner.spec.mjs` that uses `{ maxAggregateSize: 1 }` (or a similarly small value) and verifies the `resource limit: aggregate module size` error is thrown. This would exercise the new pre-read size guard path and confirm the handle is properly closed.

**No tests for `openNoFollow` extracted helper** - `packages/mds/src/util/module-scanner.ts:24-34`
**Confidence**: 80%
- Problem: `openNoFollow` was extracted from the inline try/catch in `openAndValidateModule` as a module-level helper. This is a refactoring of security-critical code (symlink detection). While the function is exercised indirectly through `buildModulesMap`, there are no tests that directly exercise the ELOOP/ENOTDIR error translation. A regression in the error-code matching (e.g., accidentally removing `ENOTDIR`) would not be caught.
- Fix: Add a test in `scanner.spec.mjs` that creates a symlink in a temp directory, calls `buildModulesMap` with it, and verifies the "symlinks are not allowed" error message. This would exercise `openNoFollow` through the public API.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**U-WB2 and U-WB4 are near-duplicates** - `packages/mds/__test__/wasm-backend.spec.mjs:32-84`
**Confidence**: 85%
- Problem: U-WB2 and U-WB4 both pre-seed `MAX_INIT_RETRIES` failures and assert the same circuit-breaker error message pattern ("failed to initialize after" + retry count). U-WB4 adds one extra assertion (`err.message.length > 0`) which is redundant when `includes()` already passed. These could be consolidated into a single test.

**Browser shape validation `describe` block has unnecessary `afterEach(_resetForTesting)`** - `packages/mds/__test__/wasm-backend.spec.mjs:234-236`
**Confidence**: 82%
- Problem: The "browser shape validation" tests (U-WB17 through U-WB20) call `validateWasmShape` which is a pure function -- it only inspects the passed object and throws or returns. It does not touch any module-level singleton state. The `afterEach(() => _resetForTesting(0))` is unnecessary and misleading, suggesting the tests modify global state when they do not.

## Suggestions (Lower Confidence)

- **Consider testing `_initBrowser` CSP error wrapping** - `packages/mds/src/backend/wasm.ts:279-296` (Confidence: 65%) -- The CSP/fetch error detection logic (lines 284-294) has multiple string-match branches but no test coverage. These error paths produce user-facing messages.

- **U-PF0 test name includes threshold value** - `packages/mds/__test__/perf.spec.mjs:21` (Confidence: 62%) -- The test name embeds "< 5000ms" but the threshold is already in the assertion. If the threshold changes, the name becomes stale. Minor readability concern.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The new tests (U-WB14 through U-WB20) are well-structured with clear AAA patterns, good use of `afterEach` cleanup, and descriptive assertion messages. The `validateWasmShape` unit tests (U-WB17-20) are a textbook example of boundary validation testing. The try/finally fix in U-B6 correctly prevents state leaks. The main concerns are: (1) the misleading U-WB13 test name that describes removed behavior, (2) missing test coverage for the security-critical scanner refactoring (aggregate size guard and symlink detection), and (3) the browser circuit breaker counter increment being untested. None are critical, but the scanner coverage gap is notable given the security sensitivity of the changes.

# Testing Review Report

**Branch**: refactor-27-28-unified-backend-architecture -> main
**Date**: 2026-05-24T12:06

## Issues in Your Changes (BLOCKING)

### MEDIUM

**U-PF0: subprocess-based timing test is non-deterministic and measures subprocess overhead, not import time** - `packages/mds/__test__/perf.spec.mjs:21-35`
**Confidence**: 85%
- Problem: The test spawns a subprocess via `execFileSync`, then measures `Date.now()` elapsed time around it. This includes Node.js process startup, V8 warmup, and ESM module graph resolution -- none of which are the import time of `node.js` itself. The threshold of 5000ms is so generous (100x the stated goal of "< 100ms") that TLA could regress significantly without tripping this test. Furthermore, the test name says "< 100ms" but the assertion checks 5000ms, creating a misleading contract.
- Fix: Either (a) rename the test to accurately describe what it measures ("subprocess import completes within 5s") and document that this is a TLA regression guard (not a true import-time benchmark), or (b) measure import time inside the subprocess itself using `performance.now()` around the dynamic `import()` and print the value, then assert on that inner measurement from the parent process. Option (b) provides a meaningful measurement:
```javascript
test('U-PF0: module import completes without blocking I/O (no TLA)', () => {
  const output = execFileSync(process.execPath, ['--input-type=module'], {
    input: `
const { performance } = await import('node:perf_hooks');
const t0 = performance.now();
await import('../dist/node.js');
const elapsed = performance.now() - t0;
console.log(elapsed.toFixed(2));
`,
    cwd: new URL('.', import.meta.url).pathname,
    encoding: 'utf8',
  });
  const importMs = parseFloat(output.trim());
  assert.ok(importMs < 100, `module import took ${importMs}ms — TLA may be present`);
});
```

**U-WB13: test title claims to verify rejection of modules missing scanImports, but only tests the positive path** - `packages/mds/__test__/wasm-backend.spec.mjs:157-168`
**Confidence**: 88%
- Problem: The test name says "tryLoadCandidate rejects modules missing scanImports" but the test body calls `initWasmNode()` and asserts that the returned module has `scanImports`. This verifies the happy path (a valid module includes scanImports), not the rejection behavior. It never constructs a module missing `scanImports` to confirm it is rejected. The comment acknowledges this ("We test this indirectly...") but the test name creates a false sense of coverage for the negative path.
- Fix: Either (a) rename to accurately describe the positive assertion: `U-WB13: initWasmNode() only succeeds when WasmModule includes scanImports`, or (b) if `tryLoadCandidate` were exported or testable, provide a true negative test with a mock module missing `scanImports`. Option (a) is the pragmatic fix:
```javascript
test('U-WB13: initWasmNode() only succeeds when WasmModule includes scanImports', async () => {
  const mod = await initWasmNode();
  assert.equal(typeof mod.scanImports, 'function',
    'initWasmNode() must only succeed when scanImports is present');
});
```

**U-B6: in-process state mutation creates ordering dependency between tests** - `packages/mds/__test__/backend.spec.mjs:57-74`
**Confidence**: 82%
- Problem: U-B6 calls `_resetForTesting()` mid-suite to clear state, then calls `await init()` at the end to re-initialize for subsequent tests. This creates an implicit ordering dependency: if the test runner ever parallelizes tests within a `describe` block, or if U-B6 throws before reaching `await init()`, subsequent tests (U-B7, U-B8, etc.) will fail with confusing "not initialized" errors. The test is also doing double duty (verifying no-TLA AND verifying pre-init throw behavior), which muddies the assertion purpose.
- Fix: Either use `afterEach` to guarantee state restoration regardless of assertion failures, or isolate the state mutation in a subprocess like U-B7 through U-B11 already do:
```javascript
test('U-B6: module import completes without I/O (no top-level await)', () => {
  // Subprocess approach: import without init(), then verify compile throws
  const output = execFileSync(process.execPath, ['--input-type=module'], {
    input: `import { compile } from '../dist/node.js';
try { compile('Hello!\\n'); console.log('no-throw'); }
catch (e) { console.log(e.message.includes('init()') ? 'correct' : 'wrong'); }
`,
    cwd: __dirname,
    env: { ...process.env },
    encoding: 'utf8',
  });
  assert.equal(output.trim(), 'correct');
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing test IDs U-B9 and U-WB7 in numbering sequence create confusion**
**Confidence**: 90%
- Problem: backend.spec.mjs jumps from U-B8 to U-B10. wasm-backend.spec.mjs jumps from U-WB6 to U-WB8. The gaps suggest tests were removed or planned but never written. This is not a functional issue but creates ambiguity about whether coverage was intentionally omitted or accidentally dropped.
- Fix: Either renumber tests to eliminate gaps, or add a comment at the file header explaining the gaps (e.g., "U-B9 reserved for check() pre-init test").

**No negative-path test for initWasmBrowser()** - `packages/mds/src/backend/wasm.ts:206-215`
**Confidence**: 82%
- Problem: `initWasmBrowser()` has substantial new logic (CSP error detection, missing `default()` check, promise dedup with retry-on-failure), but no test exercises the browser init path. The browser tests use `_initWithModuleForTesting()` to bypass it entirely. While this is necessary since Node.js cannot run `import('mds-wasm')`, the consequence is that 60+ lines of new browser-init code (lines 206-268 of wasm.ts) have zero automated test coverage. The CSP detection logic, the `default()` presence check, and the error wrapping are all uncovered.
- Fix: Add unit tests for the browser init error paths by mocking the dynamic import. Since `_initBrowser` is private, this would require either (a) exporting a test helper that injects a mock module loader, or (b) testing through browser.ts `init()` with a mock that simulates the failure modes. At minimum, document the coverage gap with a comment in the test file.

**browser.spec.mjs U-BR6 title is misleading -- test does not verify concurrent behavior** - `packages/mds/__test__/browser.spec.mjs:111-116`
**Confidence**: 85%
- Problem: U-BR6 is titled "concurrent init() cannot double-init an already-initialized backend" but the test body only calls `compile('Hello!\n')` and asserts the output includes "Hello". It does not call `init()` at all, let alone concurrently. The comment explains the intent (idempotency of already-resolved backend), but the test name and the behavior being tested are mismatched. Previously U-BR6 tested `Promise.all([init(), init()])`.
- Fix: Either rename the test to match what it actually verifies (e.g., `U-BR6: compile works after _initWithModuleForTesting`), or add actual concurrent init behavior testing.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**scanner.spec.mjs has no test for the symlink rejection behavior (ELOOP/O_NOFOLLOW)** - `packages/mds/__test__/scanner.spec.mjs`
**Confidence**: 85%
- Problem: The module-scanner's TOCTOU fix (replacing lstat+readFile with O_NOFOLLOW open+fd.stat+fd.readFile) is a significant security improvement, but the scanner test suite has no symlink-related tests. Creating symlinks in test fixtures is platform-sensitive but feasible on macOS/Linux. The `openAndValidateModule` function's ELOOP handling, Windows fallback path, and `isFile()` check are all untested.
- Fix: Consider adding a test that creates a temporary symlink and verifies `buildModulesMap` rejects it:
```javascript
test('U-SM6: rejects symlinks in import chain', async () => {
  const tmp = await mkdtemp(join(tmpdir(), 'mds-'));
  const real = join(tmp, 'real.mds');
  const link = join(tmp, 'link.mds');
  await writeFile(real, 'Hello!\n');
  await symlink(real, link);
  await assert.rejects(
    () => buildModulesMap(link, scanImports),
    /symlink/,
  );
});
```

## Suggestions (Lower Confidence)

- **U-B8 destructuring is a no-op** - `packages/mds/__test__/backend.spec.mjs:95` (Confidence: 75%) -- `const [, ] = await Promise.all([init(), init()])` uses empty destructuring slots, which is syntactically valid but unusual. A simple `await Promise.all([init(), init()])` without destructuring would be clearer.

- **browser.spec.mjs afterEach calls `initWasmNode()` unnecessarily** - `packages/mds/__test__/browser.spec.mjs:157-161` (Confidence: 70%) -- The afterEach in the "init() promise dedup and reset" suite calls `await initWasmNode()` after `wasmReset(0)`. This re-initializes the WASM module for other suites, but since `sharedWasmModule` is already captured at file scope and `_initWithModuleForTesting` bypasses the init path, the re-init may not be necessary. If it is needed, a comment explaining why would help future maintainers.

- **Missing `check()` and `checkFile()` pre-init tests in node.ts test suite** - `packages/mds/__test__/backend.spec.mjs` (Confidence: 72%) -- Tests U-B7, U-B10, U-B11 verify that `compile()`, `getBackend()`, and `compileFile()` throw before `init()`, but `check()` and `checkFile()` are not tested for the same pre-init behavior. All five functions go through the same `assertReady()` gate, so they are implicitly covered, but explicit tests for `check()` and `checkFile()` would complete the contract verification for the breaking change.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 3 | 0 |
| Should Fix | 0 | 0 | 3 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Testing Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The test suite is well-structured with 95 passing tests across 14 suites. The new tests effectively cover the breaking API change (explicit `init()` requirement), concurrent init deduplication, the split MdsBaseBackend/MdsNodeBackend interfaces, and the browser entry point's removal of file operations. The use of `_resetForTesting()` for state isolation and `_initWithModuleForTesting()` for browser-path testing in Node.js are sound patterns.

The conditions for full approval:
1. Fix U-PF0 to either measure the right thing or accurately describe what it tests (the test name says "< 100ms" but asserts 5000ms).
2. Fix U-WB13 naming to match what it actually tests (positive path, not rejection).
3. Address U-B6's in-process state mutation risk (either subprocess isolation or afterEach guard).

The untested browser init code paths (CSP detection, `default()` check, retry logic in `initWasmBrowser`) and symlink rejection behavior are noted as should-fix and pre-existing respectively, but do not block merge since the browser path cannot run in Node.js tests and the symlink fix is a security improvement over the prior TOCTOU-vulnerable code.

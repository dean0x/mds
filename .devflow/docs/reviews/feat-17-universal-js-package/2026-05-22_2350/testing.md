# Testing Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Dead variable `script` in U-B5 test** - `packages/mds/__test__/backend.spec.mjs:46`
**Confidence**: 95%
- Problem: Line 46 declares `const script = path.join(__dirname, 'backend-wasm-helper.mjs');` but this variable is never used -- the test uses inline `--input-type=module` with piped `input` instead. Furthermore, the file `backend-wasm-helper.mjs` does not exist on disk, so this would fail at runtime if it were ever used. This is dead code that suggests an incomplete refactor from a helper-file approach to an inline approach.
- Fix: Remove the dead variable:
```js
test('U-B5: MDS_BACKEND=wasm forces WASM backend', () => {
    // Spawn a subprocess with MDS_BACKEND=wasm to test backend selection
    // without affecting the current process's already-resolved backend.
    const output = execFileSync(process.execPath, ['--input-type=module'], {
```

**Stale `test:parity` npm script references deleted file** - `packages/mds/package.json:27`
**Confidence**: 95%
- Problem: The `test:parity` script still points to `__test__/parity.spec.mjs`, which was renamed to `native-backend.spec.mjs` in this PR. Running `npm run test:parity` will fail with `file not found`.
- Fix: Update the script name and target:
```json
"test:parity": "node --test __test__/native-backend.spec.mjs",
```
Or rename to `test:native` to match the new file name.

### MEDIUM

**Test U-C4 has misleading comment -- does not test what it describes** - `packages/mds/__test__/compile.spec.mjs:32-38`
**Confidence**: 90%
- Problem: The test is named "compile returns warnings for empty @include" and the comment says "Empty include does not fail, but emits a warning." However, the source under test is `'---\nname: Test\n---\nHello!\n'` -- plain frontmatter with no `@include` directive at all. The test merely asserts `typeof result.output === 'string'` and `Array.isArray(result.warnings)` -- trivially true for any valid input. It neither triggers an `@include` warning nor asserts any warnings were produced.
- Fix: Either rename the test to reflect what it actually tests (e.g., "U-C4: compile frontmatter with no body issues"), or actually test an empty `@include`:
```js
test('U-C4: compile returns warnings for empty @include', () => {
    const source = '@import ""\nHello!\n';
    const result = compile(source);
    assert.ok(result.warnings.length > 0, 'expected warning for empty import');
});
```

**Test U-C7 passes `null` to typed `vars` parameter -- tests undefined contract** - `packages/mds/__test__/compile.spec.mjs:56-58`
**Confidence**: 85%
- Problem: `compile('Hello World!\n', { vars: null })` passes `null` where the TypeScript type declares `vars?: Record<string, unknown>`. The `varsOpt` function sees `null !== undefined` and forwards `{ vars: null }` to the native addon. The test asserts "does not throw" but this is testing an undocumented implicit contract -- it depends on the Rust napi addon gracefully ignoring a null vars object. If the addon ever validates its input strictly, this test would break. This is an implementation-coupling test rather than a behavioral contract test.
- Fix: The test is acceptable as a robustness check but should explicitly document that `null` is outside the typed contract:
```js
test('U-C7: compile tolerates null vars (defensive robustness)', () => {
    // null is outside the typed contract (vars?: Record<string, unknown>)
    // but callers in plain JS may pass it. Verify no crash.
    assert.doesNotThrow(() => compile('Hello World!\n', { vars: null }));
});
```

**`isMdsError` tightened to require `mds::` prefix but no test covers the boundary** - `packages/mds/__test__/error.spec.mjs`
**Confidence**: 88%
- Problem: The source changed `isMdsError` from `typeof code === 'string'` to `typeof code === 'string' && code.startsWith('mds::')` (types.ts:71-76). Test U-E4 only tests a plain `Error('regular error')` which has no `.code` property at all. There is no test that creates an Error with a non-`mds::`-prefixed `.code` string to verify the new stricter check. This is a behavioral change in the public API with insufficient coverage.
- Fix: Add a test case for the new boundary:
```js
test('U-E4b: isMdsError returns false for errors with non-mds code', () => {
    const err = new Error('some error');
    err.code = 'ENOENT'; // has code, but not mds:: prefixed
    assert.equal(isMdsError(err), false);
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**New WASM init retry logic (MAX_INIT_RETRIES) has zero test coverage** - `packages/mds/src/backend/wasm.ts:31-52`
**Confidence**: 85%
- Problem: The PR added a `MAX_INIT_RETRIES = 3` guard with mutable `initFailures` counter. After 3 failed init attempts, the WASM backend permanently refuses to initialize. This is significant behavioral logic (a circuit breaker) with no test coverage. The mutable module-level state (`initFailures`) cannot be reset between tests, making it particularly important to test in isolation.
- Fix: Add a test that verifies the retry limit behavior. This likely requires a subprocess approach (similar to U-B5) or extracting the init state into a testable class.

**New `validateImportPath` function has no direct unit tests** - `packages/mds/src/util/module-scanner.ts:114-133`
**Confidence**: 80%
- Problem: The `validateImportPath` function was extracted from the `scan` closure to reduce nesting. While `buildModulesMap` integration tests exercise it indirectly, there are no direct unit tests for this function's security checks (null byte rejection, empty path rejection, project root escape). The scanner.spec.mjs tests only cover `normalizeVirtualKey` directly and `buildModulesMap` as integration. Since `validateImportPath` is a security-critical function performing path traversal guards, it deserves focused unit tests.
- Fix: Either export `validateImportPath` and test it directly, or add `buildModulesMap` tests that specifically exercise each validation path (null byte import, empty import, escaping import).

**New `projectRoot === '/'` guard has no test** - `packages/mds/src/util/module-scanner.ts:100-102`
**Confidence**: 82%
- Problem: The new security check `if (projectRoot === '/' || projectRoot === '')` that prevents the entry file from being at the filesystem root is untested. The existing `U-SM3` test only covers a nonexistent file, not the root-path case.
- Fix: Add a test:
```js
test('U-SM4: rejects entry at filesystem root', async () => {
    await assert.rejects(
        () => buildModulesMap('/file.mds', scanImports),
        /project root cannot be filesystem root/,
    );
});
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No tests for browser entry point** - `packages/mds/src/browser.ts`
**Confidence**: 85%
- Problem: The browser entry point (`browser.ts`) has significant behavioral logic -- `assertInitialized()` throws before `init()`, `compileFile`/`checkFile` always reject, concurrent init is deduplicated. None of this is tested. All test files import from `../dist/node.js`.

**Performance tests use wall-clock time thresholds** - `packages/mds/__test__/perf.spec.mjs`
**Confidence**: 80%
- Problem: Tests U-PF1, U-PF2, U-PF4 use `Date.now()` wall-clock assertions with generous but arbitrary thresholds (2000ms, 3000ms). These are inherently non-deterministic -- on a loaded CI runner or slow machine they could flake. The thresholds are generous enough to be unlikely to fail, but the pattern is still a flaky test smell.

## Suggestions (Lower Confidence)

- **Missing `MDS_BACKEND=invalid` test** - `packages/mds/src/node.ts:13-15` (Confidence: 75%) -- The new `console.warn` for unknown `MDS_BACKEND` values is untested. Could add a subprocess test similar to U-B5 that captures stderr.

- **No negative test for `varsOpt` utility** - `packages/mds/src/util/options.ts` (Confidence: 65%) -- The extracted `varsOpt` function is a shared utility but has no direct unit tests. It is tested indirectly through compile/check tests, which is acceptable for a 3-line function.

- **Redundant shape assertions across test files** - `compile.spec.mjs`, `compileFile.spec.mjs`, `backend.spec.mjs` (Confidence: 65%) -- U-C1, U-C6, U-B4, U-CF9, U-N1 all assert the `{output, warnings, dependencies}` shape. While not harmful, consolidating shape verification into a shared helper would reduce repetition.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 3 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Testing Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The test suite has solid breadth (63 tests across 10 files covering compile, check, file operations, errors, scanner, backend selection, and performance) but exhibits several quality gaps: a stale npm script that will fail, dead test code, a misleading test name/body mismatch, and most importantly missing coverage for newly-added behavioral logic (WASM init retry circuit breaker, `isMdsError` prefix check, `validateImportPath` security guards). The two HIGH issues (dead variable referencing nonexistent file, broken `test:parity` script) should be fixed before merge.

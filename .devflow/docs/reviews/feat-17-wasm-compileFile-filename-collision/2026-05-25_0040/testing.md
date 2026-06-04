# Testing Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25T00:40

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**U-WCF6: Error path test does not use shared `runWasm` helper — inconsistent pattern** - `packages/mds/__test__/wasm-compileFile.spec.mjs:129-151`
**Confidence**: 85%
- Problem: U-WCF6 manually spawns a subprocess with inline `exec()` call instead of using the `runWasm()` helper that every other test uses. The inline subprocess catches the error inside the script and returns `{ threw: true }`, which means the test never verifies the *error message content* -- only that *something* threw. This is weaker than the existing `compileFile.spec.mjs` (U-CF7) which uses `assert.rejects` and verifies `err instanceof Error`. The manual subprocess approach is also the only test that duplicates the env/cwd/timeout configuration, creating a maintenance burden.
- Fix: Use `runWasm()` with a script that lets the error propagate (no try/catch in subprocess). The subprocess will exit non-zero and `exec()` will reject. Assert on the rejection:
```javascript
test('U-WCF6: WASM compileFile on nonexistent file rejects with error', async () => {
  await assert.rejects(
    () => runWasm(`
      import { init, compileFile } from './dist/node.js';
      await init();
      const r = await compileFile('/nonexistent/path/file.mds');
      process.stdout.write(JSON.stringify(r));
    `),
    (err) => {
      assert.ok(err instanceof Error, 'must reject with Error');
      return true;
    },
  );
});
```

**Note**: The uncommitted on-disk version partially addresses this by routing through `runScript()`, but still uses try/catch inside the subprocess script rather than letting the error propagate naturally.

### MEDIUM

**U-WCF7/U-WCF8 parity tests only cover simple.mds — limited parity signal** - `packages/mds/__test__/wasm-compileFile.spec.mjs:153-187`
**Confidence**: 82%
- Problem: The parity tests (U-WCF7, U-WCF8) compare WASM vs native output only for `simple.mds`. The bug being fixed (filename collision) was specifically triggered when imports are present, yet the parity tests do not verify parity for files with imports. `IMPORT_CONSUMER_MDS` and `ENTRY_MDS` are available in the fixture set and would provide more meaningful parity coverage for the exact scenario this PR fixes.
- Fix: Add parity tests (or extend U-WCF7/U-WCF8) to also compare WASM vs native output for `IMPORT_CONSUMER_MDS`:
```javascript
test('U-WCF7b: WASM compileFile with imports matches native (parity)', async () => {
  const script = `
    import { init, compileFile } from './dist/node.js';
    await init();
    const r = await compileFile(${JSON.stringify(IMPORT_CONSUMER_MDS)});
    process.stdout.write(JSON.stringify({ output: r.output, warnings: r.warnings, dependencies: r.dependencies }));
  `;
  const [wasmResult, nativeResult] = await Promise.all([
    runWasm(script),
    runNative(script),
  ]);
  assert.equal(wasmResult.output, nativeResult.output);
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Subprocess scripts embed absolute paths via JSON.stringify — test portability concern** - `packages/mds/__test__/wasm-compileFile.spec.mjs:68,81,98,109` (4 occurrences)
**Confidence**: 80%
- Problem: Each subprocess script embeds the fixture's absolute path via `${JSON.stringify(SIMPLE_MDS)}` directly into the inline `-e` script string. This works correctly but means the path is hardcoded at template interpolation time in the parent process. If the script string were ever extracted to a file (e.g., for debugging), the absolute path would be baked in and non-portable. More importantly, this pattern relies on `JSON.stringify` to properly escape path characters -- safe for most paths, but a path containing characters like backticks, `${}`, or template literal metacharacters could theoretically break the template literal in the subprocess.
- Fix: This is a minor concern -- `JSON.stringify` handles the escaping correctly for standard filesystem paths. No immediate action needed, but worth noting for future test infrastructure evolution. Passing paths via environment variables would be more robust:
```javascript
const result = await runWasm(`
  import { init, compileFile } from './dist/node.js';
  await init();
  const r = await compileFile(process.env.FIXTURE_PATH);
  process.stdout.write(JSON.stringify({ output: r.output }));
`, { FIXTURE_PATH: SIMPLE_MDS });
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No checkFile error path test exists (WASM or native)** - `packages/mds/__test__/check.spec.mjs:58-66`, `packages/mds/__test__/wasm-compileFile.spec.mjs` (absent)
**Confidence**: 85%
- Problem: The new WASM test suite tests `compileFile` error path (U-WCF6) but not `checkFile` error path. The existing native `check.spec.mjs` has a `checkFile` nonexistent file test (U-KF3) but neither suite tests `checkFile` with an invalid/malformed MDS file. Since the PR fixes both `compileFile` and `checkFile` in `wrapWithFileOps`, a `checkFile` error path test for the WASM backend would strengthen confidence in the fix symmetry.

**Test suite execution time is inherently slow due to subprocess isolation** - `packages/mds/__test__/wasm-compileFile.spec.mjs` (entire file)
**Confidence**: 80%
- Problem: Each of the 8 tests spawns at least one subprocess (U-WCF7/U-WCF8 spawn two each), totaling 10 subprocess invocations. The full suite takes ~520ms. While subprocess isolation is a well-reasoned architectural choice (avoids singleton contamination from the module-level backend), the overhead will compound as more WASM file-operation tests are added.
- Note: This is the correct tradeoff for this test -- subprocess isolation is necessary to test the MDS_BACKEND environment variable behavior. Not blocking.

## Suggestions (Lower Confidence)

- **Missing assertion on error message content in U-WCF6** - `wasm-compileFile.spec.mjs:150` (Confidence: 70%) -- The test only asserts `result.threw === true` but does not verify the error message mentions the nonexistent path or contains a meaningful diagnostic. Verifying the message would catch silent error swallowing.

- **U-WCF4 assertion on `'99'` in output is fragile** - `wasm-compileFile.spec.mjs:114-116` (Confidence: 65%) -- Asserting `result.output.includes('99')` could match unintended content if the fixture format changes (e.g., a line number). A more specific assertion like `includes('You have 99 items')` would be more targeted.

- **Uncommitted improvements should be committed** - `wasm-compileFile.spec.mjs`, `src/node.ts` (Confidence: 75%) -- The on-disk versions contain quality improvements (shared `helpers.mjs` imports, DRY `prepareFileArgs` extraction, unified `runScript`/`wasmEnv`/`nativeEnv` helpers) that address some of the issues noted above. These should be committed before the PR is merged.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Strengths
- Excellent use of subprocess isolation to test the MDS_BACKEND environment variable path without singleton contamination
- Good test coverage breadth: happy path, imports, deep chains, runtime vars, error path, and cross-backend parity
- Tests follow Arrange-Act-Assert structure with clear test names and IDs (U-WCF1 through U-WCF8)
- TDD discipline is evident from the commit history (RED commit with failing tests, then GREEN commit with fix)
- Parity tests (U-WCF7/U-WCF8) are a smart validation strategy for dual-backend correctness

### Why CHANGES_REQUESTED
- The parity tests (U-WCF7/U-WCF8) should include the import scenario since that is the exact bug being fixed -- testing parity only on `simple.mds` (which had no imports and may not have triggered the collision) leaves the core fix unverified by parity comparison
- U-WCF6 error test should verify more than just `threw: true` -- either let the error propagate naturally or assert on the message content
- The uncommitted on-disk improvements (helpers.mjs reuse, DRY refactoring) should be committed as they improve the test quality

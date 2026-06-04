# Consistency Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25T00:40

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicate fixture path declarations instead of reusing helpers.mjs** - `packages/mds/__test__/wasm-compileFile.spec.mjs:13-23`
**Confidence**: 92%
- Problem: The new test file re-declares `__dirname`, `SIMPLE_MDS`, `IMPORT_CONSUMER_MDS`, and `ENTRY_MDS` locally instead of importing them from `helpers.mjs`, where they are already exported. Every other test file in the suite (`compileFile.spec.mjs`, `check.spec.mjs`, `compile.spec.mjs`, `scanner.spec.mjs`) imports fixture paths from `helpers.mjs`. This violates the established pattern and creates a maintenance risk: if fixture paths change, this file must be updated separately.
- Fix: Replace local declarations with imports from `./helpers.mjs`:
```js
import { SIMPLE_MDS, IMPORT_CONSUMER_MDS, ENTRY_MDS, __dirname } from './helpers.mjs';
import path from 'node:path';

const pkgRoot = path.join(__dirname, '..');
```
  Remove the `fileURLToPath` and `path` imports used only for computing `__dirname`.

**Two separate subprocess runner functions with duplicated logic** - `packages/mds/__test__/wasm-compileFile.spec.mjs:31-61`
**Confidence**: 82%
- Problem: `runWasm()` and `runNative()` share nearly identical subprocess-spawning logic, differing only in the `env` object. This duplication is inconsistent with the DRY principle seen elsewhere in the codebase (e.g., `compileOpts`/`fileOpts` helpers in `wasm.ts`, the shared `helpers.mjs` module). The `backend.spec.mjs` file uses `execFileSync` with explicit env on each call rather than splitting into two runners, which is a different pattern but still avoids duplication by parameterizing the env inline.
- Fix: Consolidate into a single `runScript(script, env)` function and create small `wasmEnv()` / `nativeEnv()` helpers:
```js
async function runScript(script, env) {
  const { stdout } = await exec(
    process.execPath,
    ['--input-type=module', '-e', script],
    { cwd: pkgRoot, env, timeout: 30000 },
  );
  return JSON.parse(stdout);
}

function wasmEnv() {
  return { ...process.env, MDS_BACKEND: 'wasm' };
}

function nativeEnv() {
  const env = { ...process.env };
  delete env['MDS_BACKEND'];
  return env;
}
```

### MEDIUM

**Duplicated extract-and-delete logic in compileFile and checkFile** - `packages/mds/src/node.ts:67-73,78-81`
**Confidence**: 85%
- Problem: The fix correctly applies the same 3-line pattern (extract source, delete from modules, call WASM) in both `compileFile` and `checkFile`. However, the duplication is inconsistent with the codebase's pattern of extracting shared logic into helper functions (e.g., `compileOpts()` and `fileOpts()` in `wasm.ts`, `varsOpt()` in `options.ts`, `openNoFollow()` in `module-scanner.ts`). The comment on `checkFile` even acknowledges the duplication: "Same fix as compileFile". When two methods share identical preparation logic, this codebase consolidates it.
- Fix: Extract a `prepareFileArgs` helper within `wrapWithFileOps`:
```typescript
async function prepareFileArgs(
  path: string,
  options: FileOptions | undefined,
): Promise<{ source: string; opts: ReturnType<typeof fileOpts> }> {
  const { entryFilename, modules } = await buildModulesMap(path, (src) => wasmModule.scanImports(src));
  const source = modules[entryFilename] ?? '';
  delete modules[entryFilename];
  return { source, opts: fileOpts(entryFilename, modules, options) };
}
```

**U-WCF6 uses raw subprocess instead of runWasm helper** - `packages/mds/__test__/wasm-compileFile.spec.mjs:129-151`
**Confidence**: 88%
- Problem: Test U-WCF6 manually spawns a subprocess with inline `exec()` and env construction instead of using the `runWasm()` helper that all other tests use. The test needs to catch errors inside the subprocess (try/catch around `compileFile`), but that is already handled by writing JSON to stdout -- the same pattern `runWasm` uses. The divergence is unnecessary and inconsistent with the file's own internal pattern.
- Fix: Use `runWasm()` with the try/catch script:
```js
test('U-WCF6: WASM compileFile on nonexistent file rejects with error', async () => {
  const result = await runWasm(`
    import { init, compileFile } from './dist/node.js';
    await init();
    try {
      await compileFile('/nonexistent/path/file.mds');
      process.stdout.write(JSON.stringify({ threw: false }));
    } catch (e) {
      process.stdout.write(JSON.stringify({ threw: true, message: e.message }));
    }
  `);
  assert.ok(result.threw, 'compileFile on nonexistent path must throw');
});
```

**Inconsistent error assertion pattern between U-WCF6 and U-CF7** - `packages/mds/__test__/wasm-compileFile.spec.mjs:129-151`
**Confidence**: 80%
- Problem: The existing `compileFile.spec.mjs` test U-CF7 (nonexistent file) uses `assert.rejects()` -- the idiomatic Node.js test runner pattern for asserting async rejections. The new U-WCF6 uses a try/catch inside the subprocess to capture the error. While the subprocess isolation pattern makes `assert.rejects` infeasible at the top level, the try/catch pattern inside the subprocess script captures only `threw: true` and `message` but does not assert on the error type (e.g., `err instanceof Error`). The existing U-CF7 asserts `err instanceof Error`. This is a minor behavioral consistency gap.
- Fix: Add an `instanceof` check inside the subprocess script and assert on it:
```js
catch (e) {
  process.stdout.write(JSON.stringify({
    threw: true,
    isError: e instanceof Error,
    message: e.message,
  }));
}
```
  Then assert: `assert.ok(result.isError, 'error must be an Error instance');`

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Test ID numbering comment says "U-WCF1 through U-WCF8" but there are exactly 8 tests** - `packages/mds/__test__/wasm-compileFile.spec.mjs:3` (Confidence: 65%) -- The header comment is accurate today, but the existing `wasm-backend.spec.mjs` has non-contiguous IDs (U-WB1 through U-WB21 with gaps at U-WB7). Consider whether the comment should say "8 tests" rather than "U-WCF1 through U-WCF8" to avoid staleness if tests are later removed.

- **runNative delete pattern uses bracket notation for env var** - `packages/mds/__test__/wasm-compileFile.spec.mjs:50` (Confidence: 62%) -- `delete env['MDS_BACKEND']` uses bracket notation while the env is set via spread with string key (`MDS_BACKEND: 'wasm'`). Both work identically, but the codebase's `node.ts` line 16 uses `process.env['MDS_BACKEND']` with brackets consistently. Minor style observation, not blocking.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

# Consistency Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22T23:50

## Issues in Your Changes (BLOCKING)

### HIGH

**Stale npm script references renamed file** - `packages/mds/package.json:28`
**Confidence**: 95%
- Problem: The file `parity.spec.mjs` was renamed to `native-backend.spec.mjs` in this PR, but the npm script `"test:parity"` still references the old filename `__test__/parity.spec.mjs`. Running `npm run test:parity` will fail with a file-not-found error.
- Fix: Update the script name and path to match the renamed file:
  ```json
  "test:parity": "node --test __test__/parity.spec.mjs",
  ```
  should become:
  ```json
  "test:native-backend": "node --test __test__/native-backend.spec.mjs",
  ```

**Inconsistent private function naming convention across entry points** - `packages/mds/src/backend/wasm.ts:62`
**Confidence**: 85%
- Problem: The PR renamed `_doInit` to `doInit` and `_backend` to `backend` and `_initPromise` to `initPromise` in `browser.ts`, removing the underscore-prefix convention for private module-level symbols. However, `wasm.ts` still uses `_init` (line 62) with the underscore prefix for its private init function. The PR partially applied a naming cleanup but did not carry it through consistently.
- Fix: Rename `_init` to `doInit` (matching `browser.ts`) or another non-underscore name:
  ```typescript
  // wasm.ts:53
  initPromise = doInit(options).catch((err) => {
  // wasm.ts:62
  async function doInit(options?: InitOptions): Promise<void> {
  ```

### MEDIUM

**Missing JSDoc on browser.ts exported functions** - `packages/mds/src/browser.ts:60,64,68,72,81`
**Confidence**: 85%
- Problem: The PR added JSDoc comments to all 5 exported functions in `node.ts` (lines 47, 52, 57, 62, 67) but `browser.ts` only has JSDoc on `init()` (line 29). The other exports -- `compile`, `check`, `getBackend`, `compileFile`, `checkFile` -- have no JSDoc. Both files are public entry points consumed by end users and should have matching documentation quality.
- Fix: Add the same JSDoc comments from `node.ts` to the corresponding functions in `browser.ts`:
  ```typescript
  /** Compile an MDS source string to Markdown. */
  export function compile(source: string, options?: CompileOptions): CompileResult {

  /** Validate an MDS source string without rendering. */
  export function check(source: string, options?: CompileOptions): CheckResult {

  /** Returns `'wasm'` -- browser environments always use the WASM backend. */
  export function getBackend(): BackendType {

  /** Not available in browser environments. Always rejects. */
  export function compileFile(_path: string, _options?: FileOptions): Promise<CompileResult> {

  /** Not available in browser environments. Always rejects. */
  export function checkFile(_path: string, _options?: FileOptions): Promise<CheckResult> {
  ```

**Unused variable in test** - `packages/mds/__test__/backend.spec.mjs:46`
**Confidence**: 90%
- Problem: Test U-B5 computes `const script = path.join(__dirname, 'backend-wasm-helper.mjs')` but never uses the `script` variable. The test uses inline `input` with `execFileSync` instead. This is dead code left from an earlier implementation approach and adds confusion.
- Fix: Remove the unused variable:
  ```javascript
  test('U-B5: MDS_BACKEND=wasm forces WASM backend', () => {
    // Spawn a subprocess with MDS_BACKEND=wasm to test backend selection
    // without affecting the current process's already-resolved backend.
    const output = execFileSync(process.execPath, ['--input-type=module'], {
  ```

**Unused imports in compile.spec.mjs** - `packages/mds/__test__/compile.spec.mjs:7`
**Confidence**: 92%
- Problem: `SIMPLE_MDS` and `FIXTURES` are imported from `./helpers.mjs` but never referenced in the test file body. All compile tests use inline source strings. This was likely inherited from a template or earlier version where file-based tests were planned.
- Fix: Remove the unused imports:
  ```javascript
  import { test, describe } from 'node:test';
  import assert from 'node:assert/strict';
  import { compile, isMdsError } from '../dist/node.js';
  ```

## Issues in Code You Touched (Should Fix)

_No issues found in this category._

## Pre-existing Issues (Not Blocking)

_No issues found in this category._

## Suggestions (Lower Confidence)

- **Init retry cap inconsistency between browser.ts and wasm.ts** - `packages/mds/src/browser.ts:40` (Confidence: 70%) -- `wasm.ts` caps init retries at `MAX_INIT_RETRIES = 3` (added in this PR), but `browser.ts` `doInit` allows unlimited retries by resetting `initPromise = null` without any failure counter. Since `browser.ts` delegates to `wasm.init()` which has its own cap, this may be intentionally layered, but the behavioral difference is not documented.

- **Type safety of varsOpt with null** - `packages/mds/src/util/options.ts:11` (Confidence: 65%) -- Test U-C7 passes `{ vars: null }` which flows through `varsOpt` as `{ vars: null }`. The return type claims `Record<string, unknown>` but `null` is not a `Record`. This works at runtime because downstream backends tolerate null, but the types are technically unsound. Either the `CompileOptions.vars` type should include `null`, or `varsOpt` should coerce null to undefined.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 3 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

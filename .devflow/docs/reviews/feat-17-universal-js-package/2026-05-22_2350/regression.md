# Regression Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22T23:50

## Issues in Your Changes (BLOCKING)

### HIGH

**`isMdsError` guard tightened -- now requires `mds::` prefix** - `packages/mds/src/types.ts:71-76`
**Confidence**: 85%
- Problem: The `isMdsError` type guard was changed from checking `typeof code === 'string'` to also requiring `code.startsWith('mds::')`. While all error codes from the Rust compiler (napi and WASM) currently use the `mds::` prefix, this is a behavioral change in a public API function. Any downstream consumer that creates custom errors with a non-`mds::` `code` property and relies on `isMdsError` returning `true` will now get `false`. More importantly, if a future napi or WASM boundary error is introduced without the `mds::` prefix, `isMdsError` will silently miss it.
- Fix: This change is arguably an improvement (more precise detection), but should be documented as a behavioral change. Verify this is intentional. If so, add a note in the CHANGELOG under [Unreleased]:
  ```markdown
  ### Changed
  - `isMdsError()` now requires the `code` property to start with `"mds::"` for stricter identification
  ```

**Stale `test:parity` script references deleted file** - `packages/mds/package.json:27`
**Confidence**: 95%
- Problem: The npm script `"test:parity": "node --test __test__/parity.spec.mjs"` still references `parity.spec.mjs`, which was renamed to `native-backend.spec.mjs` in this PR. Running `npm run test:parity` will fail with a file-not-found error. While `npm test` (which uses a glob) still works, anyone using the named script directly will hit a broken command.
- Fix: Update the script name and path in `package.json`:
  ```json
  "test:native": "node --test __test__/native-backend.spec.mjs",
  ```

### MEDIUM

**WASM `buildFileModules` no longer passes explicit resource limits** - `packages/mds/src/backend/wasm.ts:108-109`
**Confidence**: 82%
- Problem: Previously, `buildFileModules` passed explicit `{ maxModules: WASM_MAX_MODULES, maxAggregateSize: WASM_MAX_AGGREGATE_SIZE }` (both set to the same values as the defaults). Now it calls `buildModulesMap` without the options argument, relying on the defaults exported from `module-scanner.ts`. While the values are currently identical (256 modules, 10 MiB), this creates a coupling where any future change to module-scanner defaults will silently affect the WASM backend. The old code was explicit about its intent.
- Fix: Since the defaults are exported and identical, this is acceptable if the intent is that all callers share the same limits. However, if the WASM backend should be independently configurable, consider keeping the explicit options pass-through or adding a comment explaining the deliberate reliance on shared defaults.

## Issues in Code You Touched (Should Fix)

### LOW

**Dead variable `script` in backend test** - `packages/mds/__test__/backend.spec.mjs:46`
**Confidence**: 90%
- Problem: `const script = path.join(__dirname, 'backend-wasm-helper.mjs');` is assigned but never used. The test uses `execFileSync` with inline `input` instead, so the variable is dead code. The referenced file `backend-wasm-helper.mjs` also does not exist on disk.
- Fix: Remove the dead variable:
  ```javascript
  test('U-B5: MDS_BACKEND=wasm forces WASM backend', () => {
    // Spawn a subprocess with MDS_BACKEND=wasm to test backend selection
    // without affecting the current process's already-resolved backend.
    const output = execFileSync(process.execPath, ['--input-type=module'], {
  ```

## Pre-existing Issues (Not Blocking)

No critical pre-existing issues identified.

## Suggestions (Lower Confidence)

- **WASM `initFailures` counter is module-level mutable state with no reset mechanism** - `packages/mds/src/backend/wasm.ts:32` (Confidence: 65%) -- The `initFailures` counter increments on each failed init attempt and permanently blocks after 3 failures. In long-lived processes (e.g., SSR servers), a transient failure burst during startup could permanently disable WASM init for the process lifetime with no recovery path. Consider exposing a reset or documenting this behavior.

- **`@types/node` downgraded from `^25.9.1` to `^22.0.0`** - `packages/mds/package.json:34` (Confidence: 60%) -- The `@types/node` version was downgraded from v25 to v22 to match the `engines` requirement of `node >= 22`. This is reasonable alignment, but the range `^22.0.0` is quite broad and may pull in v22 typings that lag behind the actual Node 22 APIs used. No functional regression observed, but worth noting.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | - |
| Should Fix | - | 0 | 0 | 1 |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The PR introduces a well-structured universal JS package with no removed public exports and no broken public API signatures. The core regression risk is the `isMdsError` behavioral change (tighter guard) which, while arguably an improvement, is undocumented. The stale `test:parity` npm script is a clear broken-reference regression that should be fixed before merge. Both issues are straightforward to resolve.

The export surfaces (node.ts, browser.ts) maintain full API compatibility. Type exports are reordered but not removed. The module-scanner changes are additive (new security checks, exported constants). No deleted files contain code that wasn't migrated. All 63 tests pass.

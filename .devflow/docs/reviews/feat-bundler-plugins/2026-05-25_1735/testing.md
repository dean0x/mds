# Testing Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25T17:35
**PR**: #31

## Issues in Your Changes (BLOCKING)

### HIGH

**No-op assertion: `assert.ok(typeof result.code, 'string')` always passes** - `packages/bundler-utils/__test__/integration.spec.mjs:55`
**Confidence**: 95%
- Problem: `assert.ok(typeof result.code, 'string')` does NOT verify the type. `typeof result.code` evaluates to a truthy string (e.g. `"string"` or `"undefined"`), and the second argument to `assert.ok` is the error message, not the expected value. This assertion passes for any value, including `undefined`, `null`, or a `number`. It tests nothing.
- Fix: Replace with `assert.equal(typeof result.code, 'string')`:
```javascript
// Before (line 55)
assert.ok(typeof result.code, 'string');

// After
assert.equal(typeof result.code, 'string');
```

**Tautological assertion: split-then-check-for-newline always passes** - `packages/bundler-utils/__test__/integration.spec.mjs:92`
**Confidence**: 90%
- Problem: `result.code.split('\n')[0]` by definition cannot contain `'\n'` because `String.split('\n')` produces segments without the delimiter. So `assert.ok(!exportLine.includes('\n'), ...)` is always true regardless of the code's content. The test intends to verify that the export default line is a single line with no raw newlines inside the string literal, but the assertion is vacuous.
- Fix: Assert the export line does not contain a literal unescaped newline by checking the full code string more carefully:
```javascript
// Verify the first line is a complete export default statement
const exportLine = result.code.split('\n')[0] ?? '';
assert.ok(exportLine.startsWith('export default "'), 'first line should be export default');
assert.ok(exportLine.endsWith('";'), 'export default should end on same line');
```

**Same tautological assertion pattern in transform.spec.mjs** - `packages/bundler-utils/__test__/transform.spec.mjs:172`
**Confidence**: 90%
- Problem: Identical issue as above. `lines.find(l => l.startsWith('export default'))` returns a line from `split('\n')`, so checking `!exportLine.includes('\n')` is always true. The test named "special chars in output are escaped" does not actually verify that escaping occurred correctly.
- Fix: Instead assert that the special characters appear in their escaped form:
```javascript
assert.ok(exportLine.includes('\\n'), 'should have escaped newline');
assert.ok(exportLine.includes('\\"'), 'should have escaped quote');
assert.ok(exportLine.includes('\\\\'), 'should have escaped backslash');
```

### MEDIUM

**Vite plugin error path (transform throw) is not tested** - `packages/vite-plugin/__test__/plugin.spec.mjs`
**Confidence**: 85%
- Problem: The Vite plugin's `transform` method has a catch block (lines 51-64 of `src/index.ts`) that formats errors with `formatMdsError`, attaches `id` and `loc` properties, and re-throws. The rollup plugin test suite includes an error path test (`transform calls this.error when compile fails`), but the Vite plugin test suite has no equivalent. This is a significant error handling path with Vite-specific behavior (attaching `.loc` and `.id` to the thrown error for Vite's overlay display).
- Fix: Add an error path test:
```javascript
test('transform throws enriched error for nonexistent .mds file', async () => {
  const plugin = mdsPlugin();
  const ctx = createPluginContext();
  await plugin.buildStart.call(ctx);

  await assert.rejects(
    () => plugin.transform.call(ctx, '', '/nonexistent/path/file.mds'),
    (err) => {
      assert.ok(err instanceof Error);
      assert.equal(err.id, '/nonexistent/path/file.mds');
      return true;
    },
  );
});
```

**Webpack loader warning emission test is a no-op** - `packages/webpack-loader/__test__/loader.spec.mjs:120-126`
**Confidence**: 85%
- Problem: The test "emitWarning called for each warning" acknowledges in a comment that it "can't easily inject a mock transformer" and just verifies `simple.mds` produces zero warnings. This does not test the warning emission code path at all -- it only tests the happy path where no warnings exist. The test name is misleading because it never validates that `emitWarning` is actually called when warnings are present.
- Fix: Either rename the test to reflect what it actually verifies (e.g., "no warnings emitted for simple fixture") or note this as a known coverage gap. The webpack loader's module-level singleton makes mock injection difficult by design; the `_resetForTesting` hook was added to address singleton reuse but not mock injection.

**Temp directory not cleaned up in integration tests** - `packages/bundler-utils/__test__/integration.spec.mjs:60-76, 80-96`
**Confidence**: 82%
- Problem: The two integration tests that create temp directories (`mds-integration-{pid}` and `mds-integration-esc-{pid}`) only clean up the temp files with `unlinkSync` in their `finally` blocks, but do not remove the temporary directories themselves. Over many test runs, these empty directories accumulate in the system temp dir.
- Fix: Add `rmdirSync` or `rmSync` for the temp directory in the `finally` block:
```javascript
finally {
  try { unlinkSync(tmpFile); } catch { /* ignore */ }
  try { rmdirSync(tmpDir); } catch { /* ignore */ }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Frontmatter test cleanup uses `unlinkSync(TMP)` on a directory** - `packages/bundler-utils/__test__/frontmatter.spec.mjs:59`
**Confidence**: 88%
- Problem: The `after()` cleanup calls `unlinkSync(TMP)` on the `TMP` directory path. `unlinkSync` is for files, not directories -- on most platforms this will throw `EPERM` or `EISDIR`, which is silently caught. The directory is never actually cleaned up. Should use `rmdirSync(TMP)` or `rmSync(TMP, { recursive: true })`.
- Fix:
```javascript
// Before
try { unlinkSync(TMP); } catch { /* ignore */ }

// After
try { rmdirSync(TMP); } catch { /* ignore */ }
```

## Pre-existing Issues (Not Blocking)

No pre-existing issues found -- all test files are new in this PR.

## Suggestions (Lower Confidence)

- **Plugin tests create fresh transformer per test via `buildStart` calling real `import('@mds/mds')` each time** - `packages/rollup-plugin/__test__/plugin.spec.mjs`, `packages/vite-plugin/__test__/plugin.spec.mjs` (Confidence: 65%) -- Each test that calls `buildStart` dynamically imports the real `@mds/mds` and initializes a new transformer. Consider a `before()` hook that calls `buildStart` once and reuses the plugin instance for tests that need an initialized transformer, reducing test suite runtime.

- **`isMdsExtension` does not handle query params itself, relying on callers to `cleanId` first** - `packages/bundler-utils/__test__/frontmatter.spec.mjs:82-84` (Confidence: 62%) -- The test comment notes "Callers are expected to cleanId first" and tests `isMdsExtension('file.mds')` rather than `isMdsExtension('file.mds?inline')`. A test for the negative case (`isMdsExtension('file.mds?inline')` returning false) would document this contract explicitly.

- **Webpack loader options handling not tested with non-default options** - `packages/webpack-loader/__test__/loader.spec.mjs` (Confidence: 60%) -- `getOptions()` always returns `{}` in the mock. No test verifies that custom `vars` options are forwarded correctly through the loader to the transformer, which is a key feature of the bundler integration.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 3 | 2 | - |
| Should Fix | - | - | 1 | - |
| Pre-existing | - | - | - | - |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The test suite is well-structured overall: it uses `node:test` + `node:assert/strict` consistently, covers happy paths, error formatting, frontmatter edge cases, and plugin lifecycle hooks across all 4 packages. The integration tests using real `@mds/mds` provide valuable end-to-end coverage. However, three assertions are no-ops that silently pass without verifying anything (the `typeof` check and two split-then-check patterns), the Vite error path is untested, and temp directory cleanup has minor issues. The no-op assertions are the most concerning since they give a false sense of coverage for important behaviors (type checking, JS escaping correctness).

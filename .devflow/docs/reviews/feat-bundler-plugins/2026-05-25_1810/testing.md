# Testing Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10

## Issues in Your Changes (BLOCKING)

### HIGH

**Webpack loader warning emission path is untested** - `packages/webpack-loader/__test__/loader.spec.mjs:114-122`
**Confidence**: 85%
- Problem: The test "no warnings emitted for simple fixture" only asserts `emittedWarnings.length === 0`. This means the entire `for (const warning of result.warnings) { this.emitWarning(new Error(warning)); }` code path in `webpack-loader/src/index.ts:49-51` is never exercised. The test file itself acknowledges this limitation in a comment ("we can't easily inject a mock transformer due to the module-level singleton"). The comment claims indirect coverage via bundler-utils transform tests, but those tests only verify that warnings pass through the transformer — they do not exercise the webpack loader's `emitWarning` call with an `Error` wrapper. This is a behavioral gap: if the `new Error(warning)` wrapping were removed or the loop were accidentally deleted, no test would fail.
- Fix: Create or use a fixture `.mds` file that produces compiler warnings, or refactor `ensureTransformer` in the webpack loader to accept an injected transformer factory (the same pattern the vite/rollup plugins use by exposing `buildStart`). Then assert that `ctx.emittedWarnings` has the expected count and each entry is an `Error` instance. Example with a warnings-producing fixture:
```js
test('emitWarning called for each compiler warning', async () => {
  // Use a fixture known to produce warnings (or create one)
  const ctx = createLoaderContext(WARNS_MDS);
  await mdsLoader.call(ctx);
  assert.ok(ctx.emittedWarnings.length >= 1, 'should emit warnings');
  for (const w of ctx.emittedWarnings) {
    assert.ok(w instanceof Error, 'each warning should be wrapped in Error');
  }
});
```

**Vite plugin warning emission path is untested** - `packages/vite-plugin/__test__/plugin.spec.mjs`
**Confidence**: 83%
- Problem: The vite plugin test suite never invokes `plugin.transform.call(ctx, ...)` with a file that produces warnings. The `warnings` array captured in `createPluginContext` is never asserted on. The `this.warn(warning)` call in `vite-plugin/src/index.ts:48-49` is never exercised. Same issue as webpack — if the warning loop were removed, no test would catch it.
- Fix: Add a test that compiles a fixture producing warnings and asserts `ctx.warnings.length >= 1`.

### MEDIUM

**U+2028/U+2029 escaping is untested in escapeForJs** - `packages/bundler-utils/src/transform.ts:11-20`
**Confidence**: 82%
- Problem: The `escapeForJs` function has explicit handling for U+2028 (line separator) and U+2029 (paragraph separator) in `JS_ESCAPE_MAP`, and there is a thorough comment explaining why these must be escaped. However, no test verifies this behavior. The existing escape tests cover `\n`, `\r`, `"`, `\\`, and `\0`, but skip U+2028/U+2029. These are notoriously easy to regress because they are invisible characters, and the regex construction via `new RegExp()` (rather than a regex literal) adds fragility — a typo in the string pattern would silently break escaping.
- Fix: Add a test case to `transform.spec.mjs`:
```js
test('U+2028 and U+2029 are escaped', async () => {
  const mds = createMockMds({
    async compileFile() {
      return { output: 'a b c', warnings: [], dependencies: [] };
    },
  });
  const transformer = createMdsTransformer(mds);
  const result = await transformer.transform('/file.mds');

  const lines = result.code.split('\n');
  const exportLine = lines.find(l => l.startsWith('export default'));
  assert.ok(exportLine, 'should have export default line');
  assert.ok(!exportLine.includes(' '), 'U+2028 must not appear raw');
  assert.ok(!exportLine.includes(' '), 'U+2029 must not appear raw');
  assert.ok(exportLine.includes('\\u2028'), 'U+2028 should be escaped');
  assert.ok(exportLine.includes('\\u2029'), 'U+2029 should be escaped');
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Rollup plugin warning path has no assertions** - `packages/rollup-plugin/__test__/plugin.spec.mjs`
**Confidence**: 82%
- Problem: Same pattern as vite and webpack — the `createPluginContext()` captures `warnings` but no test asserts on them. The `this.warn(warning)` call in `rollup-plugin/src/index.ts:43-44` is never exercised. This is a pattern repeated across all three bundler plugin test suites.
- Fix: Add a test using a fixture that produces warnings, or create a mock that generates warnings, and assert `ctx.warnings.length >= 1`.

**Concurrent `ensureInit` calls not tested for race safety** - `packages/bundler-utils/src/transform.ts:33-42`
**Confidence**: 80%
- Problem: The `ensureInit` function uses a promise-caching pattern to ensure `init()` is called only once even when multiple `transform()` calls arrive concurrently. The existing test "init() called exactly once across multiple transforms" calls transforms sequentially (`await` one at a time), which does not exercise the concurrent path where multiple callers await the same `initPromise` simultaneously. The implementation looks correct, but the concurrency invariant is untested.
- Fix: Add a concurrent test:
```js
test('concurrent transforms call init exactly once', async () => {
  const mds = createMockMds();
  const transformer = createMdsTransformer(mds);

  // Fire three transforms concurrently
  await Promise.all([
    transformer.transform('/a.mds'),
    transformer.transform('/b.mds'),
    transformer.transform('/c.mds'),
  ]);

  assert.equal(mds.initCallCount, 1, 'init should be called exactly once even concurrently');
});
```

## Pre-existing Issues (Not Blocking)

_None at CRITICAL severity._

## Suggestions (Lower Confidence)

- **`handleHotUpdate` does not trigger full-reload for `.md` files with `type: mds` frontmatter** - `packages/vite-plugin/src/index.ts:65-72` (Confidence: 65%) -- The `handleHotUpdate` handler uses `isMdsExtension()` which only checks `.mds` extension, meaning `.md` files with `type: mds` frontmatter will not get HMR full-reload. This may be intentional (frontmatter detection is async and `handleHotUpdate` is synchronous), but no test documents this design choice.

- **Webpack loader `ensureTransformer` poisoned-promise recovery is untested** - `packages/webpack-loader/src/index.ts:28-32` (Confidence: 70%) -- The `.catch()` handler that resets `initPromise = null` on import failure mirrors the pattern tested in `bundler-utils/transform.spec.mjs` ("poisoned promise resets on init rejection"), but the webpack loader's own module-level singleton version of this pattern has no direct test. The comment documents the intent but nothing verifies the retry behavior.

- **`_resetForTesting` production guard is untested** - `packages/webpack-loader/src/index.ts:63-68` (Confidence: 72%) -- The `NODE_ENV === 'production'` guard in `_resetForTesting` is never tested. While minor, this guard was explicitly added in this branch and its behavior is unverified.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The test suite is solid overall — 74 tests across 9 suites with good coverage of happy paths, error paths, edge cases (null bytes, special characters, poisoned promises), and proper resource cleanup (rmSync). The cycle-1 fixes (tautological assertions, no-op asserts, vite error path test) are well-executed. The main gaps are: (1) warning emission paths in all three bundler plugins are exercised in source but never asserted in tests, and (2) the U+2028/U+2029 escape paths are documented in code but untested. These are behavioral gaps that could silently regress.

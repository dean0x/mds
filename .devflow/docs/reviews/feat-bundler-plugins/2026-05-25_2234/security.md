# Security Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25
**Prior Resolutions**: Cycle 2 resolved SEC-1 (safeJsonForJs). Verified fix is in place.

## Issues in Your Changes (BLOCKING)

### HIGH

**SEC-2: _setTransformerForTesting lacks NODE_ENV guard in vite-plugin and rollup-plugin** - `packages/vite-plugin/src/index.ts:40`, `packages/rollup-plugin/src/index.ts:34`
**Confidence**: 92%
- Problem: The `_setTransformerForTesting` export in both vite-plugin and rollup-plugin has no runtime guard preventing it from being called in production. By contrast, the webpack-loader correctly guards both `_resetForTesting` and `_setTransformerForTesting` with `if (process.env['NODE_ENV'] !== 'test') throw`. An attacker or misconfigured downstream code importing `_setTransformerForTesting` in production could inject a malicious transformer that returns arbitrary JavaScript module code, which would then be executed by the bundler in the application context.
- Impact: A compromised or misbehaving dependency importing this export could replace the compiler with one that injects arbitrary code into every `.mds` module output. While the `_` prefix convention signals "internal", it is a public export with no enforcement.
- Fix: Add the same `NODE_ENV` guard used in webpack-loader:
```typescript
// vite-plugin/src/index.ts and rollup-plugin/src/index.ts
export function _setTransformerForTesting(t: ReturnType<typeof createMdsTransformer> | null): void {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  _testTransformer = t;
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Prototype pollution via vars passthrough** - `packages/bundler-utils/src/transform.ts:76` (Confidence: 60%) -- The `options.vars` object is passed through to `mds.compileFile` without sanitization. If a user configures `vars` with `__proto__` or `constructor` keys, behaviour depends on the downstream compiler. Since `vars` originates from the bundler config (not user input at runtime), this is low-risk but worth noting for defense-in-depth.

- **File descriptor leak on partial-read race** - `packages/bundler-utils/src/frontmatter.ts:42-63` (Confidence: 62%) -- The `shouldTransform` function opens a file handle for frontmatter peeking. The `try/finally` correctly closes the handle on success or error within the `.then()` callback. However, if the process is killed between `open()` resolving and the `.then()` callback executing, the handle leaks. This is a normal Node.js lifecycle concern and not actionable in practice.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The `safeJsonForJs` fix from cycle 2 (SEC-1) is correctly in place and well-tested -- `</script>`, U+2028, and U+2029 are all escaped in the metadata export line. The `escapeForJs` helper covers the default export string literal thoroughly (backslash, quote, newline, carriage return, null byte, U+2028, U+2029). The XSS-safe output generation is solid.

The single blocking issue (SEC-2) is the missing `NODE_ENV` guard on `_setTransformerForTesting` in vite-plugin and rollup-plugin, creating an inconsistency with the webpack-loader which correctly guards its test-only exports. This is a straightforward fix -- add the same two-line guard already present in the webpack-loader.

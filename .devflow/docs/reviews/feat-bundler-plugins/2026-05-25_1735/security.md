# Security Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25
**Packages reviewed**: `@mds/bundler-utils`, `@mds/vite-plugin`, `@mds/rollup-plugin`, `@mds/webpack-loader`

## Issues in Your Changes (BLOCKING)

### HIGH

**Incomplete output escaping in `escapeForJs` -- backtick and `${` not escaped** - `packages/bundler-utils/src/transform.ts:6-22`
**Confidence**: 85%
- Problem: The `escapeForJs` function escapes the output into a double-quoted string literal (`"..."`), which is safe for that context. However, if this generated code is ever embedded in a template literal context (backtick strings), or if a downstream consumer naively wraps the output in backticks, the `${}` sequence in compiled MDS output would become a template injection vector. More concretely, the function does not escape the null byte (`\0`), which can cause truncation or unexpected behavior in some JavaScript runtimes when present in a string literal.
- Fix: Add null byte escaping. Template literal escaping is a defense-in-depth consideration for a future iteration:
```typescript
case code === 0x0000: result += '\\0'; break;
```

### MEDIUM

**Path traversal via unvalidated `id` parameter in `transform()`** - `packages/bundler-utils/src/transform.ts:46`, `packages/bundler-utils/src/frontmatter.ts:39`
**Confidence**: 82%
- Problem: The `cleanId` function strips query params and hash fragments but does not validate or normalize the file path before passing it to `fs.open()` and `mds.compileFile()`. In a bundler context, `id` comes from the bundler's module resolution and is generally trusted. However, if any upstream code passes user-influenced paths (e.g., from a CMS or dynamic import expressions), sequences like `../../etc/passwd` would be opened and read. The `shouldTransform` function in `frontmatter.ts` will `open()` any `.md` file path it receives. The `transform()` function will `compileFile()` any path it receives.
- Fix: Since these are bundler plugins and the `id` parameter originates from the bundler's own module graph, this is mitigated by the trust boundary of the bundler. However, document this assumption explicitly:
```typescript
// SECURITY: `id` is trusted — it comes from the bundler's module resolution pipeline.
// Do not call this function with user-supplied paths directly.
```

**`_resetForTesting` exported in production build** - `packages/webpack-loader/src/index.ts:52-55`
**Confidence**: 83%
- Problem: The `_resetForTesting()` function resets module-level singleton state (`transformer` and `initPromise`). It is exported from the package's public API surface (`export function`), meaning any code importing `@mds/webpack-loader` can call it. In a production webpack build, a malicious or buggy loader in the same process could call `_resetForTesting()` to force re-initialization of the MDS compiler, potentially causing race conditions or unexpected behavior.
- Fix: Gate behind `NODE_ENV` check or move to a separate test-only entry point:
```typescript
export function _resetForTesting(): void {
  if (process.env.NODE_ENV === 'production') return;
  transformer = null;
  initPromise = null;
}
```
Or better, do not export it at all and use a separate test helper that reaches into internals via a test-specific import path.

## Issues in Code You Touched (Should Fix)

_No issues in this category._

## Pre-existing Issues (Not Blocking)

_No pre-existing issues identified in files touched by this PR._

## Suggestions (Lower Confidence)

- **ReDoS potential in frontmatter regex** - `packages/bundler-utils/src/frontmatter.ts:55` (Confidence: 65%) -- The regex `/(?:^|\n)\s*type:\s*mds\b/` is applied to up to 512 bytes of user-supplied file content. The pattern is simple and the input is bounded to 512 bytes, so practical exploitation is unlikely. No action needed, but worth noting for awareness.

- **Error message information leakage** - `packages/bundler-utils/src/errors.ts:19` (Confidence: 62%) -- `formatMdsError` passes through full error messages including file paths and `help` text. In a dev-server context (Vite HMR overlay), this is desirable. In a hypothetical production SSR scenario, this could leak internal file paths. The current usage context (bundler plugins for development) makes this acceptable.

- **Webpack loader options not validated at boundary** - `packages/webpack-loader/src/index.ts:33` (Confidence: 70%) -- `this.getOptions()` returns raw options without schema validation. The `MdsPluginOptions` type provides compile-time safety but no runtime boundary validation. Since webpack has its own schema validation system (`schema` property on loader exports), this is best addressed by adding a webpack-native JSON schema to the loader export.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates solid security practices overall:
- Input from file reads is bounded (512-byte peek for frontmatter detection)
- File handles are properly closed in `finally` blocks
- Error handling is comprehensive and avoids swallowing errors silently
- The `escapeForJs` function handles the critical injection vectors for double-quoted string literals
- No hardcoded secrets, no network calls, no shell execution

The HIGH finding (incomplete escaping for null bytes) and the MEDIUM findings (undocumented trust boundary assumption on `id`, production-exposed `_resetForTesting`) should be addressed before merge but are not critical blockers given the bundler-plugin context where inputs are trusted by design.

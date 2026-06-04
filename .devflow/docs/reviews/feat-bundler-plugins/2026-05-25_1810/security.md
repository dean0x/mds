# Security Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25
**Prior Resolutions**: Cycle 1 fixed 18/20 issues including poisoned promise, escapeForJs O(n^2)+null byte, dist committed. This is cycle 2 — focused on residual and new security issues only.

## Threat Model Summary

These are bundler plugins (Vite, Rollup, Webpack) that compile `.mds` template files into JS module strings at build time. The trust boundary is:
- **Input**: File paths from bundler module resolution (trusted), file content from `.mds` files on the local filesystem (semi-trusted — developer-authored), and compiler output from `@mds/mds` (trusted).
- **Output**: Generated JavaScript module source code consumed by the bundler pipeline.
- **Runtime context**: Node.js build tooling, not a server handling user requests.

The primary attack surface is the generated JS code — specifically whether compiler output could produce syntactically invalid or injection-prone JavaScript.

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`JSON.stringify` on metadata line does not escape `</script>` or U+2028/U+2029** - `packages/bundler-utils/src/transform.ts:57`
**Confidence**: 82%
- Problem: The metadata export line uses `JSON.stringify` to serialize warnings and dependencies into inline JS:
  ```typescript
  `export const metadata = ${JSON.stringify({ warnings: result.warnings, dependencies: result.dependencies })};\n`
  ```
  `JSON.stringify` does not escape `</script>`, `<!--`, or U+2028/U+2029 characters. If a compiler warning message or dependency path contains `</script>`, and the bundled output is later placed inside an HTML `<script>` tag (as Vite dev server does with ESM transforms), this could break the script context. Similarly, U+2028/U+2029 in strings will produce raw line separators in the JS source, which is valid in ES2019+ but may cause issues with pre-ES2019 downstream parsers or source-map alignment.
- Impact: In the standard bundler pipeline, warnings/dependencies are build-time metadata that originate from the compiler and filesystem paths, making exploitation unlikely. However, if a malicious `.mds` file produces a crafted warning containing `</script>`, Vite's dev server (which serves modules inline in HTML) could be affected. The risk is low because (a) the developer controls the `.mds` files, and (b) this is a build-time tool not exposed to untrusted users.
- Fix: Apply the same escaping discipline as the default export. Either use a safe serializer or post-process `JSON.stringify` output:
  ```typescript
  function safeJsonForJs(value: unknown): string {
    return JSON.stringify(value)
      .replace(/</g, '\\u003c')
      .replace(/ /g, '\\u2028')
      .replace(/ /g, '\\u2029');
  }
  ```
  Then: `export const metadata = ${safeJsonForJs({ warnings: result.warnings, dependencies: result.dependencies })};\n`

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

_No issues found._

## Suggestions (Lower Confidence)

- **`escapeForJs` does not escape `</script>` sequences** - `packages/bundler-utils/src/transform.ts:22` (Confidence: 65%) — While the generated JS is a standalone module file and not typically embedded in HTML `<script>` tags, defense-in-depth would suggest escaping `<` as `\x3c` or `</` as `<\/` in the default export string literal. This is standard practice in template engines that generate JS (e.g., Next.js, Nuxt). The risk is minimal because bundlers write to separate `.js` files, but Vite dev server serves transformed modules in a way that could be affected.

- **`_resetForTesting` environment gate uses string check on `NODE_ENV`** - `packages/webpack-loader/src/index.ts:64` (Confidence: 62%) — The `process.env['NODE_ENV'] === 'production'` guard only protects against the exact string `"production"`. If `NODE_ENV` is unset (common in some CI environments) or set to another value, the function remains callable. This is a testing utility and the risk is limited to singleton state corruption, not a security vulnerability — but the guard could be strengthened to only allow explicitly when `NODE_ENV` is `"test"`.

- **Compiler output is trusted without sanitization** - `packages/bundler-utils/src/transform.ts:51-54` (Confidence: 60%) — The `result.output`, `result.warnings`, and `result.dependencies` from `mds.compileFile` are embedded into generated JS. The default export goes through `escapeForJs` (good), but warnings and dependencies go through `JSON.stringify` only. If the `@mds/mds` compiler were compromised or buggy, it could inject arbitrary JS through the metadata path. This is defense-in-depth only — the compiler is a trusted first-party dependency.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates good security awareness: the `escapeForJs` rewrite properly handles null bytes, backslashes, quotes, newlines, and Unicode line separators. The poisoned-promise fix from cycle 1 is correctly implemented in both `transform.ts` and `webpack-loader/src/index.ts`. The `cleanId` function correctly strips query/hash parameters before passing IDs to the filesystem.

The single blocking MEDIUM issue (JSON.stringify without HTML-safe escaping on the metadata line) is a defense-in-depth concern for Vite dev server scenarios. It is unlikely to be exploitable in practice since `.mds` files are developer-authored, but applying `<` escaping to the JSON output is a low-effort hardening measure that aligns with industry best practice for generating inline JS.

**Condition for approval**: Address the `JSON.stringify` metadata escaping (MEDIUM) or document the decision to accept the risk given the build-time-only threat model.

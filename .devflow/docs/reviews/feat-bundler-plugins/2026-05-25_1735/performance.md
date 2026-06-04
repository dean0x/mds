# Performance Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25

## Issues in Your Changes (BLOCKING)

### MEDIUM

**String concatenation in escapeForJs builds O(n) intermediate strings** - `packages/bundler-utils/src/transform.ts:7-21`
**Confidence**: 85%
- Problem: The `escapeForJs` function uses `result += ch` (and `result += '\\n'`, etc.) in a character-by-character loop. In JavaScript, string concatenation in a tight loop creates a new string object on every iteration, resulting in O(n^2) total allocation for a string of length n. For typical MDS template output (a few KB), this is negligible. However, for large compiled outputs (tens or hundreds of KB) this becomes a measurable bottleneck during build.
- Fix: Use an array to collect segments and join at the end, or use `String.prototype.replace` with a single regex pass:
```typescript
function escapeForJs(str: string): string {
  return str.replace(/[\\"\n\r  ]/g, (ch) => {
    switch (ch) {
      case '\\': return '\\\\';
      case '"':  return '\\"';
      case '\n': return '\\n';
      case '\r': return '\\r';
      case ' ': return '\\u2028';
      case ' ': return '\\u2029';
      default: return ch;
    }
  });
}
```
This approach lets the engine handle the internal buffering in native code with a single allocation for the result string, and is both more idiomatic and faster for all input sizes.

**Double cleanId call on every transform invocation** - `packages/vite-plugin/src/index.ts:38-43`, `packages/rollup-plugin/src/index.ts:33-38`
**Confidence**: 82%
- Problem: In both the Vite and Rollup plugins, `transform()` calls `cleanId(id)` to get the clean path, passes it to `transformer.shouldTransform(clean)`, and if the file should be transformed, calls `transformer.transform(id)` with the **original** (uncleaned) id. Inside `createMdsTransformer.transform()`, `cleanId(id)` is called again (transform.ts:48). This means `cleanId` runs twice per file that passes the `shouldTransform` check. Additionally, `shouldTransform` itself calls `cleanId` internally (frontmatter.ts:31), so the id is cleaned three times total per transformable file.
- Fix: Pass the already-cleaned id to `transformer.transform(clean)` instead of the raw `id`:
```typescript
// In vite-plugin/src/index.ts and rollup-plugin/src/index.ts:
const result = await transformer.transform(clean);  // was: transformer.transform(id)
```
This is a micro-optimization -- `cleanId` is cheap (indexOf + slice). Severity is MEDIUM only because the redundancy suggests a design issue where the API contract is unclear about whether callers or the transformer is responsible for cleaning IDs. Fixing it improves clarity as much as performance.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Webpack loader ignores first caller's options on subsequent calls** - `packages/webpack-loader/src/index.ts:15-27` (Confidence: 75%) -- The module-level singleton `ensureTransformer` captures options from the first `getOptions()` call. If webpack processes files with different loader options (via `use` rule overrides), only the first caller's options take effect; subsequent calls silently ignore their options. This could cause subtle correctness issues but is primarily an options-stale-cache concern. The PR description calls this "deduplicated via promise singleton" so it may be intentional, but the behavior should be documented.

- **Vite handleHotUpdate triggers full-reload for every .mds change** - `packages/vite-plugin/src/index.ts:67-74` (Confidence: 65%) -- Full page reloads on every `.mds` file change bypasses Vite's HMR granularity. For projects with many `.mds` files, this could noticeably slow down the development feedback loop. A more targeted approach would invalidate only the changed module. This may be intentional given MDS compilation semantics (dependencies could cascade), so flagging as a suggestion only.

- **Buffer.alloc(512) on every .md file check** - `packages/bundler-utils/src/frontmatter.ts:39-43` (Confidence: 62%) -- Each `.md` file encountered by the bundler triggers a 512-byte `Buffer.alloc` + file open + read + close. For projects with many non-MDS `.md` files, this could add up. A pre-allocated reusable buffer or a simple `readFile` with `{ length: 512 }` would avoid repeated allocation. However, the current approach is sound for most real-world project sizes.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The overall performance design is solid. The 512-byte frontmatter peek is a good optimization over reading entire files. The init() singleton pattern correctly deduplicates expensive WASM/native initialization. The two MEDIUM findings are real but low-impact for typical usage -- the escapeForJs string concatenation would only matter for unusually large compiled outputs, and the triple cleanId call is a micro-optimization that matters more for API clarity than raw speed. No blocking performance issues.

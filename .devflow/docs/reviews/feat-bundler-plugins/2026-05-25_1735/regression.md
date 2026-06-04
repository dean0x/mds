# Regression Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25T17:35

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Committed dist/ artifacts for new packages violate project .gitignore conventions** - `packages/bundler-utils/dist/`, `packages/rollup-plugin/dist/`, `packages/vite-plugin/dist/`, `packages/webpack-loader/dist/`
**Confidence**: 85%
- Problem: The existing `packages/mds/dist/` is listed in `.gitignore` (line 8), but all 4 new packages commit their `dist/` directories (24 files total: `.js`, `.d.ts`, `.js.map`, `.d.ts.map`). This is a convention mismatch. If a future `.gitignore` update adds `packages/*/dist/` to match the existing pattern, these committed dist files would become stale and diverge from source. More immediately, any contributor rebuilding the packages will see uncommitted changes in `dist/` from different TypeScript compiler output, causing noisy diffs.
- Fix: Add the new dist directories to `.gitignore` and remove them from tracking:
  ```
  # .gitignore - add:
  packages/bundler-utils/dist/
  packages/rollup-plugin/dist/
  packages/vite-plugin/dist/
  packages/webpack-loader/dist/
  ```
  Then `git rm --cached` the dist files. Alternatively, if the intent is to ship pre-built artifacts (unlike `@mds/mds`), document this divergence explicitly.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Root @types/node changed from dev to peer in package-lock.json** - `package-lock.json` (Confidence: 60%) -- The root-level `@types/node@25.9.1` entry changed from `"dev": true` to `"peer": true`. This is an npm workspace hoisting artifact caused by adding new packages that peer-depend on webpack/vite/rollup (which themselves optionally peer-depend on `@types/node`). The existing `packages/mds` already had its own `@types/node@22.19.19` and is unaffected. No functional regression, but the lock file change is larger than strictly necessary for "only workspace links."

- **MdsApi interface in bundler-utils is a narrower projection of @mds/mds exports** - `packages/bundler-utils/src/types.ts:1-5` (Confidence: 65%) -- The `MdsApi` interface defines only `compileFile`, `init`, and `isMdsError`, while `@mds/mds` exports additional functions (`compile`, `check`, `checkFile`, `getBackend`). If the `@mds/mds` module shape changes (e.g., `compileFile` is renamed or its signature changes), the `MdsApi` type redeclaration would silently mask the incompatibility at compile time since the plugins use `import('@mds/mds')` at runtime. The `./mds` export in `bundler-utils/mds.d.ts` partially addresses this for consumers, but the internal `MdsApi` is hand-written, not derived from `@mds/mds` types.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions

1. Decide on the dist/ artifact strategy: either gitignore them (matching `packages/mds/`) or document why these 4 packages diverge from the existing convention.

### What Went Well

- Zero modifications to existing source code -- all 56 non-lockfile changes are new files in new packages.
- No removed exports, no deleted files, no changed function signatures in existing code.
- The `package-lock.json` changes are purely additive (new workspace links, hoisted peer deps) with the mds-napi link correctly preserved (relocated, not removed).
- The `MdsApi` interface is structurally compatible with the actual `@mds/mds` Node export surface (`compileFile`, `init`, `isMdsError` all match).
- All new packages use `peerDependencies` for `@mds/mds` and their respective bundlers, avoiding forced version resolution on consumers.
- Root `package.json` uses `"packages/*"` glob so no modification was needed to register new workspaces.

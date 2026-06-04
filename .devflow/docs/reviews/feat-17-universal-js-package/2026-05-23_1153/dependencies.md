# Dependencies Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T11:53
**PR**: #26
**Cycle**: 4 (incremental)

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Undeclared `mds-wasm` runtime dependency used as fallback** - `packages/mds/src/backend/wasm.ts:108`
**Confidence**: 85%
- Problem: The WASM backend lists `'mds-wasm'` as a fallback candidate in `_init()` (line 108: `'mds-wasm'`), meaning `require('mds-wasm')` will be attempted at runtime. However, `mds-wasm` is not declared in `package.json` under any dependency field (`dependencies`, `optionalDependencies`, or `peerDependencies`). Unlike `mds-napi` which is properly declared as an `optionalDependency`, this fallback path relies on an undeclared package being resolvable at runtime.
- Impact: In a production npm install scenario (not workspace), this fallback will silently fail since the package is never installed. The code handles the failure gracefully (returns null and continues), but consumers who publish `mds-wasm` separately would need to know to install it manually with no guidance from the dependency manifest. This creates a "phantom dependency" -- code that references a package not declared in the manifest.
- Fix: Either declare `mds-wasm` as an `optionalDependency` alongside `mds-napi`, or document clearly that the second candidate path is only for future use:
```json
{
  "optionalDependencies": {
    "mds-napi": "file:../../crates/mds-napi",
    "mds-wasm": "^0.1.0"
  }
}
```
Alternatively, add a comment in wasm.ts making it explicit this is a forward-looking path:
```typescript
// Future: when mds-wasm is published as a standalone npm package
'mds-wasm',
```

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

### LOW

**`mds-napi` uses `file:` protocol link** - `packages/mds/package.json:31`
**Confidence**: 82%
- Problem: The `mds-napi` optional dependency uses `"file:../../crates/mds-napi"` which is a workspace-local reference. This is fine for development in a monorepo, but this reference will break if `@mds/mds` is published to npm without first updating the version specifier to a registry-hosted version.
- Impact: Low for now since this is a pre-release project. Will become blocking when publishing to npm.
- Note: This is pre-existing (not changed in this PR). The lockfile change (`dependencies` -> `optionalDependencies`) correctly reflects the package.json state.

## Suggestions (Lower Confidence)

- **Consider `peerDependencies` for native addon** - `packages/mds/package.json` (Confidence: 65%) -- For a universal package that auto-detects backends, `peerDependencies` with `peerDependenciesMeta: { optional: true }` may be more semantically correct than `optionalDependencies`, as it signals to consumers that *they* choose which backend to install. This is a design decision that can be deferred.

- **No `engines` field in root `package.json`** - `package.json` (Confidence: 62%) -- Root workspace declares `"node": ">=22.0.0"` in engines, but this is not enforced (no `engine-strict` in `.npmrc`). The sub-package `@mds/mds` also declares the same engine constraint. Consider adding an `.npmrc` with `engine-strict=true` or removing the redundant root-level engines field.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 1 |

**Dependencies Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions
1. Address the undeclared `mds-wasm` phantom dependency -- either declare it as optional or add a clear comment that the fallback path is forward-looking.

### Positive Observations
- Correct move of `mds-napi` from `dependencies` to `optionalDependencies` -- this is the right pattern for a universal package where the native addon may not be available.
- Lockfile is committed and consistent with package.json.
- Zero known CVEs reported by `npm audit`.
- Minimal dependency footprint: only `@types/node` and `typescript` as devDependencies, with the native addon as the sole optional runtime dependency.
- All imports are either relative or Node.js built-ins (`node:fs/promises`, `node:path`, `node:module`) -- no undeclared third-party package imports.
- No unused dependencies detected.

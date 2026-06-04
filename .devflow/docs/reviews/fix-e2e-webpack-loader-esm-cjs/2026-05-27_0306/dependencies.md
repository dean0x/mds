# Dependencies Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

No CRITICAL or HIGH issues found.

### MEDIUM

**Exports map missing `default` fallback condition** - `packages/bundler-utils/package.json:11-16`, `packages/webpack-loader/package.json:11-16`
**Confidence**: 82%
- Problem: The `exports` map specifies `types`, `import`, and `require` conditions but omits a `default` fallback. While Node.js and most bundlers resolve one of `import`/`require`, some edge-case tools (Deno, non-standard bundlers, or future runtimes) that do not send `import` or `require` conditions will get no resolution. The `default` condition acts as a universal catch-all per the Node.js package exports spec.
- Fix: Add a `default` condition as the last entry pointing to the CJS build (the safer fallback):
```json
".": {
  "types": "./dist/index.d.ts",
  "import": "./dist/index.js",
  "require": "./dist-cjs/index.js",
  "default": "./dist-cjs/index.js"
}
```

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing dependency issues found.

## Suggestions (Lower Confidence)

- **Lockfile not updated with package.json changes** - `package-lock.json` (Confidence: 65%) -- The `package.json` files for `bundler-utils` and `webpack-loader` were modified (new `main` field, new `exports` conditions, new `files` entries) but `package-lock.json` shows no diff on this branch. In npm workspaces, changes to `main`/`exports`/`files` fields typically do not require lockfile regeneration (only dependency version changes do), so this is likely fine. However, if consumers install from a registry in the future, running `npm install` once to ensure the lockfile reflects the current workspace state is good hygiene.

- **Sibling bundler plugins (`rollup-plugin`, `vite-plugin`) depend on `@mds/bundler-utils` but lack CJS exports** - `packages/rollup-plugin/package.json`, `packages/vite-plugin/package.json` (Confidence: 62%) -- Both `@mds/rollup-plugin` and `@mds/vite-plugin` consume `@mds/bundler-utils` as a dependency. This PR adds CJS support to `bundler-utils` but does not add it to the sibling plugins. This may be intentional (Rollup and Vite are ESM-native tools), but if any CJS consumer transitively loads one of these plugins, it would still fail. Low priority since those tools are inherently ESM.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Dependencies Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions
1. Consider adding a `default` fallback condition to the exports maps (MEDIUM -- not strictly blocking but improves interoperability).

### Positive Observations

- **Exports map condition ordering is correct**: `types` first, then `import`, then `require` -- matches the Node.js resolution priority (first match wins). This is the canonical ordering recommended by the Node.js docs.
- **`main` field correctly points to CJS build**: The `main` field (`./dist-cjs/index.js`) serves as the legacy fallback for tools that do not understand `exports`, which is exactly right for CJS consumers.
- **`type: "module"` preserved**: Both packages retain `type: "module"`, meaning `.js` files in the root are ESM. The CJS build correctly uses a `dist-cjs/package.json` with `{"type":"commonjs"}` to override this for the CJS output directory -- this is the standard dual-publish pattern.
- **`_esmImport` workaround is well-documented and necessary**: The `new Function('id', 'return import(id)')` trick in `webpack-loader/src/index.ts` correctly preserves native `import()` in CJS output, avoiding the TypeScript CJS transform that would rewrite it to `require()` -- which would break loading the ESM-only `@mds/mds` package.
- **`.gitignore` updated**: `packages/*/dist-cjs/` is gitignored, preventing build artifacts from being committed.
- **`files` field updated**: Both packages include `dist-cjs/` in the `files` array, ensuring the CJS build is published to the registry.
- **No new runtime dependencies added**: This change is purely a build-configuration addition (new tsconfig, updated scripts) with no new `dependencies` or `peerDependencies`.
- **CJS compatibility tests are thorough**: Both packages have new test files that verify `require()` loading, export presence, and behavioral correctness of the CJS build.
- **Build parallelization** (applies ADR-001 -- quality gate): The build scripts run ESM and CJS TypeScript compilations in parallel (`&` + `wait`), which was flagged and fixed in the prior resolution cycle.

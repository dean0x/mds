# Dependencies Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27T01:00

## Issues in Your Changes (BLOCKING)

No blocking dependency issues found.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing `main` field for legacy CJS fallback** - `packages/webpack-loader/package.json:10-15`, `packages/bundler-utils/package.json:10-14`
**Confidence**: 82%
- Problem: Both packages set `"type": "module"` and add a `"require"` export condition, but do not define a `"main"` field. Older Node.js versions (pre-12.11) and some tooling (e.g., older bundler plugin resolvers, Jest with default config) fall back to `"main"` when `"exports"` is not understood. The `"require"` condition alone covers Node 12.11+ but not legacy resolvers that ignore the exports map entirely.
- Impact: CJS consumers using tools that do not support the `"exports"` field will fail to resolve the package. Since the PR's purpose is specifically Webpack 5 CJS compatibility, and Webpack 5 does support `"exports"`, the practical impact is limited to edge-case tooling.
- Fix: Add a `"main"` field pointing to the CJS entry:
  ```json
  "main": "./dist-cjs/index.js",
  ```
  This provides a fallback for resolvers that do not understand `"exports"`. Place it before the `"exports"` field for readability.

## Pre-existing Issues (Not Blocking)

### LOW

**Rollup and Vite plugins lack `require` export condition** - `packages/vite-plugin/package.json`, `packages/rollup-plugin/package.json`
**Confidence**: 85%
- Problem: The PR adds CJS builds to `bundler-utils` and `webpack-loader` but does not extend this to `vite-plugin` or `rollup-plugin`. While Vite and Rollup are primarily ESM consumers so this is not urgent, the inconsistency means `@mds/bundler-utils` now ships a CJS build that its sibling packages do not. If a CJS consumer depends on `@mds/bundler-utils` transitively through rollup/vite plugins, the chain breaks at the plugin level.
- Impact: Low -- Vite and Rollup ecosystems are ESM-native. This is an inconsistency, not a bug.
- Fix: Consider adding CJS builds to rollup-plugin and vite-plugin in a follow-up PR if CJS consumers are expected for those packages.

## Suggestions (Lower Confidence)

- **No lockfile changes despite build script modifications** - `package-lock.json` (Confidence: 65%) -- The build scripts changed in both packages but no `package-lock.json` diff is present. This is expected since no new dependencies were added (confirmed by PR description), but worth verifying the lockfile is regenerated after the tsconfig additions to ensure any transitive resolution changes are captured. Run `npm install` and check for lockfile drift.

- **Export condition ordering: `default` condition recommended last** - `packages/webpack-loader/package.json:10-15`, `packages/bundler-utils/package.json:10-14` (Confidence: 62%) -- Node.js documentation recommends placing more specific conditions (`types`, `require`) before less specific ones, with `default` (if used) always last. The current ordering (`types` -> `import` -> `require`) is acceptable since there is no `default` condition, but if `default` is added in the future, it must come after `require`. The current order is correct for now.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 1 |

**Dependencies Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

**Conditions**:
1. Consider adding `"main"` field for legacy CJS fallback (MEDIUM, should-fix).

**Positive observations**:
- No new runtime dependencies added -- the CJS build is achieved purely through build configuration.
- The `_esmImport` workaround in webpack-loader correctly handles the ESM-in-CJS problem for `@mds/mds` dynamic imports.
- `dist-cjs/` is already in `.gitignore`.
- CJS compatibility tests exist for both packages.
- The `dist-cjs/package.json` with `{"type":"commonjs"}` trick correctly overrides the parent `"type": "module"` for CJS output.
- License (MIT) is consistent across all packages.
- No version range changes, no new supply chain exposure.

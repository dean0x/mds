# Dependencies Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Build script fragility: inline Node.js one-liner for CJS package.json marker** - `packages/bundler-utils/package.json:28`, `packages/webpack-loader/package.json:24`
**Confidence**: 82%
- Problem: The build script chains `tsc && tsc && node -e "require('fs').writeFileSync(...)"` as a single inline command. The deeply escaped JSON string `'{\\\"type\\\":\\\"commonjs\\\"}\\n'` is brittle -- any edit risks breaking the escaping silently, producing an invalid `dist-cjs/package.json`. Additionally, this approach is duplicated verbatim across two packages (DRY violation), and the `writeFileSync` call has no error handling -- if the `dist-cjs` directory does not exist yet (e.g., if the second `tsc` fails silently despite `&&`), the script will throw an unhandled error.
- Fix: Extract the CJS marker generation into a small shared script (e.g., `scripts/write-cjs-marker.js`) or use a static `dist-cjs/package.json` file committed to each package. Example shared script approach:
  ```js
  // scripts/write-cjs-marker.js
  import { writeFileSync, mkdirSync } from 'node:fs';
  const dir = process.argv[2] ?? 'dist-cjs';
  mkdirSync(dir, { recursive: true });
  writeFileSync(`${dir}/package.json`, '{"type":"commonjs"}\n');
  ```
  Then in package.json:
  ```json
  "build": "tsc -p tsconfig.json && tsc -p tsconfig.cjs.json && node ../../scripts/write-cjs-marker.js dist-cjs"
  ```

### MEDIUM

**Redundant `default` export condition duplicates `require`** - `packages/bundler-utils/package.json:16`, `packages/webpack-loader/package.json:16`
**Confidence**: 83%
- Problem: The exports map specifies both `"require": "./dist-cjs/index.js"` and `"default": "./dist-cjs/index.js"` pointing to the same file. The `default` condition in Node.js exports maps acts as a fallback for any condition not matched above. Since `types`, `import`, and `require` already cover all standard Node.js resolution paths, the `default` entry is redundant. While not harmful, it could mask issues -- if a consumer uses a non-standard condition, they silently get CJS instead of an error, potentially causing confusing runtime behavior in ESM-only environments.
- Fix: This was likely added intentionally per PRIOR_RESOLUTIONS ("exports map default fallback added"), so this is informational. If the intent is to ensure bundlers with non-standard condition resolution still work, keep it. If not, consider removing the `default` entry to keep the exports map minimal and explicit.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`./mds` subpath export lacks `import`/`require`/`default` conditions** - `packages/bundler-utils/package.json:18-20`
**Confidence**: 84%
- Problem: The `"./mds"` subpath export only exposes `"types": "./mds.d.ts"`. While this is types-only (no runtime code), this means `import '@mds/bundler-utils/mds'` will fail at runtime in any context because there is no `import` or `require` condition to resolve. If this is purely a TypeScript declaration subpath (consumers only use it for type augmentation), it works with `/// <reference types="..." />` or tsconfig paths but will confuse tooling that tries to resolve it as a real module.
- Fix: If this is intentionally types-only, add a comment or consider using the `typesVersions` field instead of an exports subpath. If runtime resolution is needed, add an `import` condition pointing to the actual module.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`@mds/mds` package lacks CJS exports** - `packages/mds/package.json:8-18`
**Confidence**: 80%
- Problem: The `@mds/mds` package (a peer dependency of both modified packages) only provides ESM exports (`"import"` conditions) and no `"require"` condition. Since `bundler-utils` and `webpack-loader` now offer CJS builds that depend on `@mds/mds`, CJS consumers will fail when trying to `require()` code that internally imports `@mds/mds`. The CJS build output will contain `require("@mds/mds")` calls that cannot be resolved in a pure CJS environment.
- Fix: Either add CJS exports to `@mds/mds` or document that the CJS builds of `bundler-utils` and `webpack-loader` require the consumer's bundler to handle the ESM-to-CJS interop for `@mds/mds`. Since Webpack 5 handles this natively, this may be acceptable for the webpack-loader use case.

### LOW

**Inconsistent CJS support across sibling plugins** - `packages/rollup-plugin/package.json`, `packages/vite-plugin/package.json`
**Confidence**: 80%
- Problem: `webpack-loader` and `bundler-utils` now have dual ESM/CJS builds, but sibling packages `rollup-plugin` and `vite-plugin` do not. While Rollup and Vite are ESM-native bundlers (so CJS is unnecessary for them), the shared `bundler-utils` dependency now ships CJS that those plugins do not use. This is not a bug but creates an inconsistency in the monorepo's build strategy.
- Fix: No action needed -- this is intentional since only Webpack 5 requires CJS loader support. Consider adding a brief comment in `bundler-utils/package.json` or README explaining why CJS was added (Webpack 5 compatibility).

## Suggestions (Lower Confidence)

- **tsconfig.cjs.json `paths` inconsistency** - `packages/webpack-loader/tsconfig.cjs.json:10-13` (Confidence: 72%) -- The webpack-loader's CJS tsconfig includes `paths` overrides for `@mds/bundler-utils` and `@mds/mds`, but the bundler-utils CJS tsconfig does not include any `paths`. If the CJS build resolution depends on these paths, the asymmetry could indicate a missing configuration in bundler-utils, or an unnecessary one in webpack-loader.

- **No `clean` script for `dist-cjs` output** - `packages/bundler-utils/package.json`, `packages/webpack-loader/package.json` (Confidence: 65%) -- The build scripts append to `dist-cjs/` but there is no `clean` or `prebuild` script to remove stale artifacts. If a source file is renamed or deleted, old `.js` files remain in `dist-cjs/`.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 1 |

**Dependencies Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions:
1. Consider extracting the CJS marker write script to reduce duplication and fragility (HIGH -- the current inline approach works but is a maintenance risk)
2. Verify that `@mds/mds` ESM-only exports do not break webpack-loader CJS consumers at runtime (the Webpack 5 module resolution likely handles this, but should be confirmed with an e2e test)

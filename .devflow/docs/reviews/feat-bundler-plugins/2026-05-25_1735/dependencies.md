# Dependencies Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25T17:35

## Issues in Your Changes (BLOCKING)

### HIGH

**Compiled dist/ artifacts committed to version control** - `packages/bundler-utils/dist/`, `packages/vite-plugin/dist/`, `packages/webpack-loader/dist/`, `packages/rollup-plugin/dist/` (32 files)
**Confidence**: 95%
- Problem: All four new packages commit their `dist/` directories (32 compiled `.js`, `.d.ts`, `.js.map`, `.d.ts.map` files). The existing `packages/mds` package explicitly gitignores its `dist/` directory (see root `.gitignore` entry `packages/mds/dist/`). The new packages have no `.gitignore` files and no root `.gitignore` entries for their `dist/` directories. Committed build artifacts bloat the repository, create noisy diffs on every rebuild, and risk merge conflicts on generated files.
- Fix: Add gitignore entries for the new package dist directories. Either add per-package `.gitignore` files or add entries to the root `.gitignore`:
  ```gitignore
  # In root .gitignore, add:
  packages/bundler-utils/dist/
  packages/vite-plugin/dist/
  packages/webpack-loader/dist/
  packages/rollup-plugin/dist/
  ```
  Then remove the tracked dist files: `git rm -r --cached packages/bundler-utils/dist packages/vite-plugin/dist packages/webpack-loader/dist packages/rollup-plugin/dist`

### MEDIUM

**Bundler peer dependencies missing from devDependencies for testing** - `packages/vite-plugin/package.json:13`, `packages/webpack-loader/package.json:13`, `packages/rollup-plugin/package.json:13`
**Confidence**: 82%
- Problem: The vite-plugin, webpack-loader, and rollup-plugin packages declare their respective bundlers (vite, webpack, rollup) only as `peerDependencies`. They do not list them in `devDependencies`. While npm workspace hoisting currently resolves the bundlers at the root `node_modules/` (they appear as peer-installed in the lockfile), this creates an implicit dependency on the hoisting behavior. If a consumer clones the repo and runs `npm install --workspace=packages/vite-plugin`, the peer dep may not be auto-installed depending on npm version and configuration. The tests import `@mds/mds` which dynamically imports the bundler APIs, so tests do work today, but only because of the lockfile state.
- Fix: Add the bundlers as devDependencies in each plugin package so testing is self-contained:
  ```json
  // vite-plugin/package.json
  "devDependencies": {
    "@mds/mds": "file:../mds",
    "@types/node": "^22.0.0",
    "typescript": "^5.4.0",
    "vite": "^6.0.0"
  }
  ```
  Apply the same pattern for webpack-loader (`"webpack": "^5.0.0"`) and rollup-plugin (`"rollup": "^4.0.0"`).

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Consider `peerDependenciesMeta` for optional bundler peer deps** - `packages/vite-plugin/package.json:13` (Confidence: 65%) -- When publishing, consumers who install `@mds/vite-plugin` will get a warning if `vite` is not installed. Adding `"peerDependenciesMeta": { "vite": { "optional": false } }` is not needed since it defaults to required, but documenting this intention explicitly can help. More relevant: if you ever want to support `vite` as optional (e.g., for type-only usage), `peerDependenciesMeta` is the mechanism.

- **`@mds/bundler-utils` uses `file:` link for `@mds/mds` in devDependencies** - `packages/bundler-utils/package.json:17` (Confidence: 70%) -- All four packages use `"@mds/mds": "file:../mds"` in devDependencies. This is fine for workspace development but will not resolve after publishing to npm. Since `@mds/mds` is already declared as a peerDependency with `"^0.1.0"`, the file link in devDependencies is only used locally for development. This is a standard monorepo pattern and works correctly, but worth noting that the publish workflow must ensure `devDependencies` are excluded from the published tarball (which npm does by default).

- **Large lockfile delta from peer-installed bundlers** - `package-lock.json` (Confidence: 60%) -- The lockfile grew by ~2100 lines, primarily from transitive dependencies of webpack (70+ packages including @webassemblyjs, ajv, terser, etc.) and vite (esbuild platform binaries). This is expected for these bundlers but represents significant transitive dependency surface area. All resolved packages come from the npm registry with integrity hashes, which is correct.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Dependencies Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The dependency structure is well-designed: `bundler-utils` correctly peer-depends on `@mds/mds`, plugin packages correctly depend on `bundler-utils` (as a regular dependency via `file:` link) and peer-depend on both `@mds/mds` and their respective bundler. Version ranges are appropriate and not overly wide. The lockfile is committed and has integrity hashes.

The two actionable issues are: (1) dist artifacts should not be committed -- this contradicts the existing `packages/mds` convention which gitignores its dist directory, and (2) bundler devDependencies should be explicit rather than relying on workspace hoisting for test isolation.

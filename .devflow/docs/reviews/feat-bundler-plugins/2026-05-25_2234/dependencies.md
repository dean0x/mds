# Dependencies Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Missing `license` field in all 4 new package.json files** -- Confidence: 85%
- `packages/bundler-utils/package.json`, `packages/rollup-plugin/package.json`, `packages/vite-plugin/package.json`, `packages/webpack-loader/package.json`
- Problem: None of the 4 new packages declare a `"license"` field. While a root `LICENSE` file exists, npm packages intended for publication should include a `license` field in package.json for registry metadata, tooling compatibility (e.g., `license-checker`), and user visibility. Without it, npm will show "UNLICENSED" on the registry page, which may deter adoption.
- Fix: Add `"license": "MIT"` (or the appropriate license matching the root LICENSE file) to each package.json:
  ```json
  {
    "name": "@mds/vite-plugin",
    "version": "0.1.0",
    "license": "MIT",
    ...
  }
  ```

**Missing `repository`, `homepage`, and `bugs` fields in all 4 new packages** -- Confidence: 82%
- `packages/bundler-utils/package.json`, `packages/rollup-plugin/package.json`, `packages/vite-plugin/package.json`, `packages/webpack-loader/package.json`
- Problem: None of the new packages include `repository`, `homepage`, or `bugs` fields. These are standard npm metadata that help users find the source code, documentation, and issue tracker. The npm registry uses these for package pages.
- Fix: Add repository metadata to each package.json:
  ```json
  {
    "repository": {
      "type": "git",
      "url": "https://github.com/<org>/mdl.git",
      "directory": "packages/vite-plugin"
    },
    "bugs": {
      "url": "https://github.com/<org>/mdl/issues"
    }
  }
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### LOW

**`@mds/mds` uses `file:` protocol for `mds-napi` optional dependency** -- Confidence: 80%
- `packages/mds/package.json:30`
- Problem: `"mds-napi": "file:../../crates/mds-napi"` is in `optionalDependencies`, which would be published to npm. When users install `@mds/mds` from the registry, this `file:` reference will fail to resolve. However, since it is `optionalDependencies`, the install will not hard-fail -- it will silently skip it. This is pre-existing and not introduced in this PR.
- Note: This will need resolution before publishing `@mds/mds` to npm (either publish `mds-napi` separately, or use a registry-resolvable version specifier).

## Suggestions (Lower Confidence)

- **Consider adding `keywords` to new packages** - `packages/*/package.json` (Confidence: 65%) -- Keywords improve discoverability on npm. Relevant terms: `mds`, `markdown`, `bundler`, `vite`/`rollup`/`webpack` respectively.

- **Consider `peerDependenciesMeta` for bundler peers** - `packages/rollup-plugin/package.json`, `packages/vite-plugin/package.json`, `packages/webpack-loader/package.json` (Confidence: 60%) -- Marking bundler peer deps with `"optional": true` via `peerDependenciesMeta` could prevent install warnings in edge cases (e.g., monorepos where the bundler is hoisted). However, these are required peers -- users must have the bundler installed -- so marking them optional would be misleading. This was previously flagged and confirmed as a false positive (per PRIOR_RESOLUTIONS). No action needed.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 1 |

**Dependency Health Summary**:
- 0 known CVEs (`npm audit` clean)
- 0 dependency tree errors or warnings (`npm ls` clean)
- Lockfile properly updated with workspace links for all 4 new packages
- `file:` protocol correctly replaced with `^0.1.0` in `dependencies` (confirmed per PRIOR_RESOLUTIONS)
- `file:` protocol correctly retained in `devDependencies` for workspace development
- Version consistency verified: `@types/node@^22.0.0`, `typescript@^5.4.0`, `node>=22.0.0` uniform across all packages
- No circular dependencies detected
- Peer dependency structure is correct: plugins peer-depend on their bundler + `@mds/mds`, and depend on `@mds/bundler-utils` as a regular dependency
- Bundler version ranges are reasonable: `vite@^5.0.0 || ^6.0.0`, `rollup@^3.0.0 || ^4.0.0`, `webpack@^5.0.0`

**Dependencies Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions: Add `license` field to all 4 new package.json files before publishing to npm. The `repository`/`homepage`/`bugs` fields are recommended but not strictly required for merge.

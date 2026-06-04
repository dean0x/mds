# Dependencies Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10
**Cycle**: 2 (incremental)

## Cross-Cycle Awareness

Cycle 1 resolved 18/20 issues including:
- Bundler devDependencies (rollup, vite, webpack) added to all 3 plugin packages -- **verified fixed**
- Package.json formatting normalized across 4 packages -- **verified fixed**
- Dist artifacts removed from tracking (commit 7106cb5) and .gitignore updated -- **verified fixed**

This cycle focuses on remaining dependency concerns not addressed in cycle 1.

## Issues in Your Changes (BLOCKING)

### HIGH

**`file:` protocol in production `dependencies` will break npm publish (3 occurrences)** - `packages/rollup-plugin/package.json:22`, `packages/vite-plugin/package.json:22`, `packages/webpack-loader/package.json:22`
**Confidence**: 95%
- Problem: All three plugin packages declare `@mds/bundler-utils` as a regular `dependency` using `"file:../bundler-utils"`. Unlike `devDependencies` (where `file:` is fine for local development), regular `dependencies` are resolved by consumers when they install the package from npm. The `file:` protocol references a local filesystem path that does not exist on the consumer's machine. Running `npm publish` with `file:` in `dependencies` will either fail validation or produce a broken package.
- Impact: Any consumer installing `@mds/rollup-plugin`, `@mds/vite-plugin`, or `@mds/webpack-loader` from a registry will get an installation error because `file:../bundler-utils` cannot be resolved.
- Fix: Replace `file:` with a version range. Since all packages are at `0.1.0`:
```json
"dependencies": {
  "@mds/bundler-utils": "^0.1.0"
}
```
Note: This is a pre-release project (zero users per project memory), so there is no release tooling (changesets/lerna) that would automatically rewrite `file:` references at publish time. This must be addressed before any publish, but is not a runtime correctness issue during local development.

### MEDIUM

**Lockfile not updated after dependency changes** - `package-lock.json`
**Confidence**: 82%
- Problem: The diff modifies `dependencies`, `devDependencies`, and `peerDependencies` across 4 package.json files and adds new devDependencies (rollup, vite, webpack), but `package-lock.json` shows no changes in the diff. The lockfile on disk (3127 lines) exists but may have been updated outside the branch diff range, or may not reflect the current package.json state.
- Impact: Other developers cloning and running `npm install` may get different dependency versions than intended, leading to inconsistent builds.
- Fix: Run `npm install` and commit the updated lockfile:
```bash
npm install
git add package-lock.json
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**No `peerDependenciesMeta` for optional peer dependencies** - `packages/rollup-plugin/package.json:24`, `packages/vite-plugin/package.json:24`, `packages/webpack-loader/package.json:24`
**Confidence**: 80%
- Problem: The bundler tools (rollup, vite, webpack) are declared as `peerDependencies` without `peerDependenciesMeta` marking them as optional. While this is the correct approach for bundler plugins (the host bundler should always be present), npm 7+ will auto-install peer dependencies by default. This means `npm install @mds/vite-plugin` will attempt to install both `vite` AND `rollup` AND `webpack` if a consumer has multiple plugins. Each plugin independently declares only its own bundler peer, so this is not a cross-contamination issue -- but worth noting the design is correct.
- Note: This is informational. The current peer dependency design is standard for bundler plugins. No action needed.

## Suggestions (Lower Confidence)

- **Missing `description`, `license`, `repository` fields** - all 4 `package.json` files (Confidence: 65%) -- These are conventional for publishable npm packages. Not blocking since this is pre-release, but will be needed before first publish.

- **`@mds/bundler-utils` as dependency vs peerDependency** - `packages/rollup-plugin/package.json:22`, `packages/vite-plugin/package.json:22`, `packages/webpack-loader/package.json:22` (Confidence: 60%) -- Currently `@mds/bundler-utils` is a regular dependency, meaning it ships with each plugin. An alternative pattern is to make it a peer dependency so consumers get a single shared copy. However, since bundler-utils is an internal implementation detail (not directly used by consumers), shipping it as a regular dependency is a defensible choice that simplifies installation.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Dependencies Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The `file:` protocol in production `dependencies` is the primary concern. While it works during local monorepo development, it will produce broken packages on publish. The lockfile should also be verified as current. The overall dependency architecture (peer deps for bundler tools, shared bundler-utils as regular dep, structural typing for @mds/mds) is well-designed.

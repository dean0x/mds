# Dependencies Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing lockfile: package-lock.json is git-ignored** - `.gitignore:8`
**Confidence**: 95%
- Problem: The branch explicitly adds `package-lock.json` to `.gitignore`, preventing the lockfile from being committed. Without a committed lockfile, `npm install` resolves version ranges non-deterministically across environments, leading to dependency drift, unreproducible builds, and potential supply chain vulnerabilities. Caret ranges like `^5.4.0` or `^25.9.1` can resolve to different patch/minor versions across installs.
- Impact: Different contributors, CI runners, and deploy environments may get different dependency trees. A newly published malicious version within the caret range could be pulled silently.
- Fix: Remove `package-lock.json` from `.gitignore` and commit the lockfile:
  ```diff
  # .gitignore
  -package-lock.json
  ```
  Then run `npm install` and commit the resulting `package-lock.json`.

### MEDIUM

**@types/node version misalignment with engines constraint** - `packages/mds/package.json:34`
**Confidence**: 82%
- Problem: `@types/node` is pinned to `^25.9.1` but the `engines` field requires `>=22.0.0`. The `@types/node` major version typically tracks the Node.js major version it describes. Version 25.x provides type definitions for Node 25 APIs, but the project explicitly targets Node 22+. This means type definitions may expose APIs not available on the minimum supported runtime (Node 22), masking compatibility issues at compile time.
- Impact: Code may type-check successfully while using Node 25 APIs that don't exist on Node 22, causing runtime failures for users on older supported versions.
- Fix: Pin `@types/node` to match the minimum supported Node version:
  ```json
  "devDependencies": {
    "@types/node": "^22.0.0",
    "typescript": "^5.4.0"
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**No .npmrc with engine-strict enforcement** - `package.json:4`
**Confidence**: 80%
- Problem: The root `package.json` declares `"engines": { "node": ">=22.0.0" }` but npm does not enforce engine constraints by default. Without an `.npmrc` containing `engine-strict=true`, contributors using Node < 22 will silently proceed with potentially broken installs.
- Impact: Contributors with older Node versions get confusing failures instead of a clear error at install time.
- Fix: Create `.npmrc` at the project root:
  ```ini
  engine-strict=true
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Pre-release dependency: serde_yml 0.0.12** - `Cargo.toml` (workspace)
**Confidence**: 85%
- Problem: `serde_yml = "0.0.12"` is a 0.0.x pre-release version. Such versions have no semver stability guarantees (any release can break). The codebase already has a comment noting this ("track for 0.1.x stability milestone"), so the team is aware.
- Impact: Any new `serde_yml` release could break compilation or change behavior. The `0.0.x` range in Cargo.toml under semver means only exactly `0.0.12` is resolved (Cargo treats 0.0.x specially), so practical risk is limited while the lockfile is committed.

## Suggestions (Lower Confidence)

- **Consider adding `packageManager` field to root package.json** - `package.json` (Confidence: 65%) — Without a `packageManager` field (e.g., `"packageManager": "npm@10.x"`), contributors could use different package managers (pnpm, yarn) that may interpret the workspace config differently. Corepack support via `packageManager` field would enforce consistent tooling.

- **file: dependency portability** - `packages/mds/package.json:31` (Confidence: 70%) — The `"mds-napi": "file:../../crates/mds-napi"` dependency works in the monorepo context but will not resolve for external consumers installing from npm. The `files` array and `exports` map suggest this package may be published. Ensure the publish workflow bundles or replaces the file reference before publishing.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Dependencies Score**: 5/10
**Recommendation**: CHANGES_REQUESTED

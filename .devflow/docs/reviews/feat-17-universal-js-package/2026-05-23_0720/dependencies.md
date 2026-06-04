# Dependencies Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Lockfile out of sync with optionalDependencies change** - `packages/mds/package.json:30`
**Confidence**: 92%
- Problem: `package.json` declares `mds-napi` under `optionalDependencies`, but `package-lock.json` still records it under `dependencies` in the `packages/mds` entry. In lockfile v3, `npm ci` uses the lockfile classification, meaning `mds-napi` will be treated as a hard dependency during CI installs despite the `package.json` intent. This defeats the purpose of the `dependencies` -> `optionalDependencies` change, which is central to the universal package's native/WASM auto-detection design (install should succeed even when the native addon cannot be built).
- Fix: Run `npm install` (with a Node.js version meeting engine requirements) to regenerate the lockfile, then verify the `packages/mds` entry shows `optionalDependencies` instead of `dependencies`. Commit the updated `package-lock.json`.

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

### LOW

**Dev dependency engine requirement exceeds project minimum** - `crates/mds-napi/package.json`
**Confidence**: 80%
- Problem: `@napi-rs/cli@^3.0.0` resolved to v3.6.2, which pulls in `@inquirer/ansi@2.0.5` requiring `node >=23.5.0 || ^22.13.0`. The project's `engines.node` field is `>=22.0.0`, and `engine-strict=true` is set in `.npmrc`. This means `npm install` fails on Node 22.3.0-22.12.x. Since `@napi-rs/cli` is a devDependency of `mds-napi` (a workspace member), this only affects developers building the native addon, not end users.
- Fix: Either bump the project's minimum Node version to `>=22.13.0` to align with the transitive requirement, or pin `@napi-rs/cli` to a version whose transitive dependencies are compatible with the current engine range.

## Suggestions (Lower Confidence)

- **file: protocol portability for published packages** - `packages/mds/package.json:31` (Confidence: 70%) -- The `"mds-napi": "file:../../crates/mds-napi"` specifier works within the monorepo workspace but will break if `@mds/mds` is published to npm as-is. The PR description notes this is pre-release, and the PRIOR_RESOLUTIONS confirm this was previously evaluated as a false positive since the package is not yet published. Flagging as a reminder for publish preparation.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 1 |

**Dependencies Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

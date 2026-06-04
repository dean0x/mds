# Dependencies Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Broken `test:parity` script references renamed file** - `packages/mds/package.json:28`
**Confidence**: 95%
- Problem: The `test:parity` script still references `__test__/parity.spec.mjs`, but this PR renames that file to `__test__/native-backend.spec.mjs`. Running `npm run test:parity` will fail with "no test files found."
- Fix: Update the script name and path to match the rename:
```json
"test:native": "node --test __test__/native-backend.spec.mjs"
```
Or remove the script if it is no longer needed as a standalone entry point (the `test` glob `__test__/*.spec.mjs` already picks it up).

### MEDIUM

**Stale nested `crates/mds-napi/package-lock.json` no longer gitignored** - `crates/mds-napi/package-lock.json`
**Confidence**: 85%
- Problem: This PR removes `package-lock.json` from `.gitignore` to commit the root workspace lockfile. A side effect is that the pre-existing nested `crates/mds-napi/package-lock.json` (1843 lines, from before the workspace setup) is now visible to git as untracked. It is stale and conflicts with the root lockfile which is the single source of truth for the npm workspace. If accidentally committed, it would cause confusion about which lockfile governs `mds-napi` dependencies.
- Fix: Either delete the nested lockfile or add a specific ignore rule:
```gitignore
# Root lockfile is committed; ignore nested lockfiles from pre-workspace installs
crates/mds-napi/package-lock.json
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`mds-napi` uses `file:` protocol as a runtime dependency** - `packages/mds/package.json:31`
**Confidence**: 82%
- Problem: `"mds-napi": "file:../../crates/mds-napi"` is listed under `dependencies` (not `devDependencies` or `optionalDependencies`). When `@mds/mds` is published to npm, `file:` references are included verbatim in the published `package.json`. Consumers who `npm install @mds/mds` will get an install error because `file:../../crates/mds-napi` does not exist on their machine. The code in `node.ts` already handles this gracefully (try/catch with WASM fallback), but the hard `dependency` declaration will cause npm to report the install as failed before the code even runs.
- Fix: Move `mds-napi` to `optionalDependencies` or remove it from `package.json` entirely and rely on the runtime `require()` resolution with the existing try/catch fallback. When publishing, the native addon would be published as its own package (e.g., `@mds/native-darwin-arm64`) or bundled.
- Note: This is pre-existing (not changed in this PR) and relates to the broader publishing strategy. Not blocking.

## Suggestions (Lower Confidence)

- **Wide `@napi-rs/cli` version range** - `crates/mds-napi/package.json:13` (Confidence: 65%) -- `"@napi-rs/cli": "^3.0.0"` is a wide range for a build toolchain dependency. The resolved version is 3.6.2. Consider pinning more tightly (e.g., `^3.6.0`) to avoid unexpected build behavior changes from minor version bumps, though this is a dev dependency and lower risk.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Positive Observations**:
- Lockfile (v3) now committed with full integrity hashes -- good reproducibility practice
- `.npmrc` with `engine-strict=true` enforces the `>=22.0.0` engine requirement at install time
- `@types/node` downgraded from `^25.9.1` to `^22.0.0` to properly align with the engine constraint
- All licenses are permissive (MIT, ISC, Apache-2.0, 0BSD, Python-2.0) -- no GPL concerns
- `npm audit` reports zero known vulnerabilities across all dependencies
- Minimal production dependency surface (only `mds-napi` via workspace link; everything else is dev)
- TypeScript 5.9.3 resolved -- current and well-maintained

**Dependencies Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

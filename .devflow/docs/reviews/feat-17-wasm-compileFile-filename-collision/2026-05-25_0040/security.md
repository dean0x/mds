# Security Review Report

**Branch**: main (feat-17 wasm compileFile filename collision fix)
**Date**: 2026-05-25
**PR**: #30

## Issues in Your Changes (BLOCKING)

No CRITICAL or HIGH security issues found.

No MEDIUM or LOW security issues found.

## Issues in Code You Touched (Should Fix)

No security issues found in adjacent code.

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing security issues found.

## Suggestions (Lower Confidence)

- **Test subprocess script injection surface** - `wasm-compileFile.spec.mjs:53,66,83,94,108,119,132,150` (Confidence: 65%) -- The test file uses `JSON.stringify()` to embed fixture paths into inline ESM scripts passed to `execFile` via `-e`. While `JSON.stringify()` is safe for embedding strings into JavaScript source (it handles special characters, quotes, and backslashes correctly), and the paths originate from hardcoded constants in `helpers.mjs` (not user input), the pattern of constructing code strings for subprocess execution is inherently sensitive. If future tests ever derive paths from external input without the `JSON.stringify()` wrapper, this could become a code injection vector. Current usage is safe -- the fixture paths are compile-time constants and `execFile` (not `exec`) is used, which avoids shell interpretation.

- **Subprocess environment propagation** - `wasm-compileFile.spec.mjs:39,43` (Confidence: 60%) -- `wasmEnv()` spreads `process.env` into subprocess environments. This is standard for test code and poses no risk in a CI/test context, but it means any sensitive environment variables in the parent process are visible to the child. Acceptable for a test-only file that is not shipped.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

### Changes Reviewed

1. **`packages/mds/src/node.ts`** -- Bug fix in `wrapWithFileOps()` that extracts the entry source from the modules map and deletes the entry key before passing to WASM. The `prepareFileArgs` helper centralizes this logic.

2. **`packages/mds/__test__/wasm-compileFile.spec.mjs`** -- 8 new subprocess-isolated tests forcing the WASM backend via `MDS_BACKEND=wasm`.

3. **`packages/mds/package.json`** -- Removes `wasm/` from the `files` field (the WASM binary is already inside `dist/`).

### Security Properties Verified

- **No new input trust boundaries introduced**: The `path` parameter to `compileFile`/`checkFile` flows through the existing `buildModulesMap()` which already validates paths (symlink rejection via O_NOFOLLOW, project root confinement, null byte rejection, depth/size limits). The new code does not bypass or weaken any of these checks.

- **Mutation is safe**: The `delete modules[entryFilename]` operates on a fresh object allocated per-call by `buildModulesMap()`. There is no shared state mutation, no prototype pollution risk (the key is `basename()` of a resolved path, not user-controlled input flowing through unchecked), and no impact on concurrent callers.

- **No new external input surfaces**: The fix only changes how an internally-constructed data structure (the modules map from `buildModulesMap`) is consumed. No new network, filesystem, or user-input surfaces are introduced.

- **Test subprocess isolation**: Tests use `execFile` (not `exec`), which does not invoke a shell and is immune to shell injection. Scripts are passed via `-e` flag, and all interpolated values use `JSON.stringify()` for safe embedding.

- **Package surface reduction**: Removing `wasm/` from `files` reduces the published package surface area, which is a minor security improvement (fewer files exposed to npm consumers).

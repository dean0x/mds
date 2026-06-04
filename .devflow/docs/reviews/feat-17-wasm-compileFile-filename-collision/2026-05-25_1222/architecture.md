# Architecture Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25T12:22
**Commit Range**: db99f70...HEAD (1 commit: 687315c)
**Prior Cycles**: Cycle 1 resolved 6 issues, identified 4 false positives. This is Cycle 2.

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none -- no items between 60-79% confidence)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This incremental commit (687315c) applies resolutions from Cycle 1 review findings and adds expanded test coverage. The architecture changes are sound:

**DRY extraction (prepareFileArgs)**: The duplicated entry-extraction logic in `compileFile` and `checkFile` was correctly consolidated into a single `prepareFileArgs` helper function at `node.ts:68-81`. Both methods now delegate to it, eliminating the parallel-maintenance risk flagged in Cycle 1. The helper is a closure inside `wrapWithFileOps`, scoped correctly to capture `wasmModule` without leaking it.

**Fail-fast invariant replaces silent fallback**: The previous `modules[entryFilename] ?? ''` silent empty-source fallback was replaced with an explicit invariant assertion (`node.ts:74-77`). This is architecturally correct -- `buildModulesMap` always populates the entry file (module-scanner.ts:269 + 287), so the invariant should never fire, but its presence catches impossible states loudly rather than producing confusing downstream WASM errors.

**Contract documentation**: The `BuildModulesMapResult.modules` field now has a JSDoc comment (module-scanner.ts:43-51) documenting the caller contract: WASM callers MUST extract and remove the entry source before calling `build_modules()`. This closes the documentation gap that enabled the original bug.

**Test decomposition**: The test file was refactored for clarity -- `runWasm`/`runNative` collapsed into a single `runScript(script, env)` with `wasmEnv()`/`nativeEnv()` helpers. Fixture constants moved to shared `helpers.mjs`. New tests U-WCF9 through U-WCF11 cover import-file parity and checkFile error paths, addressing Cycle 1 findings.

**Cross-cycle false positives not re-flagged**: The following Cycle 1 false positives remain correctly excluded: asymmetric backend architectures (WASM adapter layer is intentional), delete mutation safety (buildModulesMap creates fresh objects per call), subprocess test overhead (necessary for backend isolation), parity test concern (valid testing strategy).

**Layering assessment**: The separation between `module-scanner.ts` (Node-only file I/O), `wasm.ts` (browser-safe WASM interface), and `node.ts` (Node-specific orchestration) remains clean. Dependencies point in the correct direction: `node.ts` imports from both `wasm.ts` and `module-scanner.ts`, neither of which imports from `node.ts`. No circular dependencies.

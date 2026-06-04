# Security Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25
**PR**: #30
**Cycle**: 2 (incremental review of commit 687315c)

## Cross-Cycle Awareness

Prior cycle (cycle 1) reviewed the initial fix and test suite. That review found 0 blocking security issues and 2 lower-confidence suggestions (subprocess script injection surface at 65%, subprocess environment propagation at 60%). Both were acknowledged as false positives in the resolution summary. This cycle reviews the hardening commit (687315c) which refactors the fix and expands test coverage.

## Issues in Your Changes (BLOCKING)

No CRITICAL, HIGH, MEDIUM, or LOW security issues found.

## Issues in Code You Touched (Should Fix)

No security issues found in adjacent code.

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing security issues found.

## Suggestions (Lower Confidence)

No new suggestions. The two lower-confidence items from cycle 1 (subprocess script injection surface, environment propagation) were already evaluated as false positives and remain unchanged in this diff. No new patterns warrant flagging.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

### Changes Reviewed (Commit 687315c)

1. **`packages/mds/src/node.ts:68-81`** -- Extracted `prepareFileArgs()` helper that centralizes the entry source extraction and modules map mutation for both `compileFile` and `checkFile`. Key security-relevant change: replaced the silent `?? ''` empty-string fallback with an explicit invariant violation throw. This is a positive security change -- silent fallbacks can mask unexpected states and lead to downstream bugs where the WASM compiler processes empty source without the caller realizing the entry was missing.

2. **`packages/mds/src/util/module-scanner.ts:43-51`** -- Documentation-only change adding JSDoc to the `modules` field of `BuildModulesMapResult`. The comment explicitly documents the collision hazard and the required caller contract (extract and remove entry source before passing to WASM). No code change, no security impact.

3. **`packages/mds/__test__/wasm-compileFile.spec.mjs`** -- Test refactoring and expansion:
   - Unified `runWasm`/`runNative` into `runScript(script, env)` with explicit env parameter -- eliminates the implicit `MDS_BACKEND: 'wasm'` in the old `runWasm`, making the environment control explicit and auditable.
   - Added empty-stdout guard (`if (!stdout.trim()) throw new Error(...)`) -- prevents `JSON.parse` from silently consuming empty or whitespace-only subprocess output, which could mask test failures.
   - Added U-WCF9/U-WCF10 (import parity tests) and U-WCF11 (checkFile error path) -- expands test coverage for WASM file operations.
   - Tightened U-WCF4 assertion from `includes('99')` to `includes('You have 99 items')` -- reduces false-positive match risk.

### Security Properties Verified

- **Invariant enforcement strengthened**: The `?? ''` fallback in the prior code silently produced an empty source string when `buildModulesMap` failed to populate the entry. The new code throws immediately, preventing the WASM compiler from processing an invalid empty-source scenario. This is defense-in-depth: while `buildModulesMap` should always populate the entry, the invariant check ensures any future regression in the module scanner is caught immediately rather than producing corrupted output.

- **No new input trust boundaries**: The `path` parameter still flows through `buildModulesMap()` with its existing security controls (O_NOFOLLOW, project root confinement, null byte rejection, depth/size limits). No bypass paths introduced.

- **Mutation safety unchanged**: The `delete modules[entryFilename]` operates on a per-call fresh object from `buildModulesMap()`. Now centralized in `prepareFileArgs()` rather than duplicated, reducing the risk of future divergence between `compileFile` and `checkFile`.

- **Test code uses safe subprocess patterns**: `execFile` (not `exec`) avoids shell injection. `JSON.stringify()` safely embeds fixture paths. All fixture paths are compile-time constants from `helpers.mjs`.

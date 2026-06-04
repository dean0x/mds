# Complexity Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25T12:22
**Diff**: `git diff db99f70...HEAD` (incremental, cycle 2)

## Cross-Cycle Awareness

Cycle 1 resolved 6 issues (silent empty-source fallback, undocumented return type contract, missing import scenario test, fragile assertion, missing error path test, missing empty-stdout guard) and dismissed 4 as false positives (asymmetric backend architectures, delete mutation safety, subprocess test overhead, parity test concern). This cycle reviews only the incremental diff since those fixes.

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Repetitive inline script templates across 11 test cases** - `wasm-compileFile.spec.mjs:51-211`
**Confidence**: 82%
- Problem: Each of the 11 tests (U-WCF1 through U-WCF11) constructs a nearly identical inline ESM script string with the same `import { init, compileFile } from './dist/node.js'; await init(); ...` boilerplate. The file is 214 lines and the pattern repeats 11 times. While each test is individually simple (low cyclomatic complexity per test), the aggregate duplication makes the file harder to maintain -- a signature change to `init()` or `compileFile()` requires updating all 11 inline strings.
- Note: This is pre-existing structural duplication that was NOT introduced in this diff. The diff only added 3 new tests (U-WCF9, U-WCF10, U-WCF11) that follow the pre-existing pattern, and refactored the runner functions (which is a complexity improvement). Not blocking.

## Suggestions (Lower Confidence)

(none -- no findings in the 60-79% range)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Complexity Score**: 9/10
**Recommendation**: APPROVED

## Rationale

The changes in this incremental diff are uniformly **complexity-reducing**:

1. **`prepareFileArgs()` extraction in `node.ts`** (lines 68-81): Previously, both `compileFile` and `checkFile` contained identical 5-line sequences (buildModulesMap call, source extraction, delete, option construction). The new `prepareFileArgs()` helper eliminates this duplication, reducing each method body to a single 2-line call. The helper itself is 14 lines, nesting depth 1, cyclomatic complexity 2 (one branch for the undefined check). Well within all thresholds.

2. **`runScript` / `wasmEnv` / `nativeEnv` refactoring in tests** (lines 29-47): The previous `runWasm` and `runNative` functions duplicated subprocess setup. The new design extracts environment construction into two tiny pure functions (`wasmEnv`, `nativeEnv`, 3 lines each) and unifies execution into a single `runScript` function (9 lines, complexity 1). This is a net complexity reduction.

3. **JSDoc on `BuildModulesMapResult.modules`** (module-scanner.ts lines 43-51): Pure documentation addition clarifying the contract -- no logic change, no complexity impact.

4. **New tests U-WCF9, U-WCF10, U-WCF11**: Follow the established pattern, each under 20 lines, cyclomatic complexity 1 per test. No nesting beyond the test callback.

All changed functions are well under complexity thresholds (< 30 lines, nesting < 3, parameters <= 2, cyclomatic complexity < 5). No magic values, no boolean complexity, no deep nesting. The invariant check in `prepareFileArgs` (line 74) is a clean fail-fast pattern that replaces the previous silent `?? ''` fallback -- an improvement flagged and resolved in cycle 1.

# Complexity Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10
**Cycle**: 2 (incremental — builds on cycle 1 which resolved 18/20 issues)

## Issues in Your Changes (BLOCKING)

No blocking complexity issues found.

## Issues in Code You Touched (Should Fix)

No should-fix complexity issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing complexity issues at CRITICAL severity.

## Suggestions (Lower Confidence)

- **Duplicated transform() structure across three plugins** - `rollup-plugin/src/index.ts:31-53`, `vite-plugin/src/index.ts:36-63`, `webpack-loader/src/index.ts:40-57` (Confidence: 70%) -- The three bundler plugins share a nearly identical transform-then-forward-deps-and-warnings pattern (cleanId, shouldTransform guard, transform, iterate deps, iterate warnings, error formatting). The shared `createMdsTransformer` already extracts the core logic, and each plugin layer has bundler-specific glue (Rollup uses `this.error()`, Vite throws with `.id`/`.loc`, Webpack uses `callback()`), so this is intentional divergence. If more plugins are added, a higher-order "wrapPlugin" could reduce the pattern further, but at three instances this is reasonable.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Complexity Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### Function Length and Cyclomatic Complexity

All source files in the diff are well within healthy thresholds:

| File | Longest Function | Lines | Cyclomatic Complexity | Nesting Depth |
|------|-----------------|-------|-----------------------|---------------|
| `bundler-utils/src/transform.ts` | `createMdsTransformer` | 39 | 3 | 2 |
| `bundler-utils/src/frontmatter.ts` | `shouldTransform` | 31 | 5 | 3 |
| `bundler-utils/src/errors.ts` | `formatMdsError` | 13 | 4 | 2 |
| `rollup-plugin/src/index.ts` | `mdsPlugin` | 36 | 4 | 3 |
| `vite-plugin/src/index.ts` | `mdsPlugin` | 51 | 5 | 3 |
| `webpack-loader/src/index.ts` | `mdsLoader` | 17 | 2 | 2 |

No function exceeds 50 lines. Maximum nesting depth is 3 (well under the 4-level warning threshold). Maximum cyclomatic complexity is 5 (borderline of "good" range, not a concern).

### Improvements from Cycle 1

The refactoring commit (`b1d6b6a`) that simplified plugin code after resolution fixes reduced complexity measurably:

1. **`escapeForJs` in transform.ts** -- Replaced a 14-line `switch(true)` with a 1-line `str.replace()` using a lookup map. This reduced cyclomatic complexity from 7 (one branch per escape character) to 1 (single regex replacement). Clean improvement.

2. **Rollup error handling** -- Condensed a 7-line if/else block into a 3-line ternary + single `this.error()` call. Lower nesting depth.

3. **Vite error construction** -- Replaced a 7-line imperative property assignment pattern with a 5-line `Object.assign()` call. Reduces visual noise and nesting.

4. **`shouldTransform` delegation** -- Changed from wrapping `checkTransform(id)` in a method body to direct property assignment (`shouldTransform: checkTransform`). Zero-overhead delegation.

5. **Poisoned promise fix** -- The rejection handler in `ensureInit()` (`initPromise = null; throw err;`) adds one branch but prevents a permanent failure state. This is justified complexity.

### Parameter Counts

All public functions accept 1-2 parameters. `createMdsTransformer(mds, options?)` is the maximum at 2. No parameter objects are needed at these counts.

### Boolean Complexity

No compound boolean expressions found. All conditions are simple single-predicate checks (e.g., `transformer === null`, `initialized`, `!should`).

### Magic Values

The only numeric constant is `PEEK_BYTES = 512` in `frontmatter.ts`, which is properly named. No magic numbers elsewhere.

### File Lengths

| File | Lines |
|------|-------|
| `bundler-utils/src/transform.ts` | 65 |
| `bundler-utils/src/types.ts` | 63 |
| `bundler-utils/src/frontmatter.ts` | 61 |
| `bundler-utils/src/errors.ts` | 29 |
| `vite-plugin/src/index.ts` | 74 |
| `rollup-plugin/src/index.ts` | 55 |
| `webpack-loader/src/index.ts` | 69 |

All files are under 100 lines. Well under the 300-line warning threshold.

### Readability Assessment

Code is highly readable. Each function does one thing. Early returns are used consistently to avoid nesting. Comments explain "why" (trust boundary notes, structural typing rationale, poisoned promise behavior) rather than "what". The `JS_ESCAPE_MAP` lookup table is clearer than the previous switch statement it replaced.

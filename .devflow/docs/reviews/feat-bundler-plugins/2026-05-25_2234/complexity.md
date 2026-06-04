# Complexity Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T22:34

## Issues in Your Changes (BLOCKING)

No blocking complexity issues found.

## Issues in Code You Touched (Should Fix)

No should-fix complexity issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing complexity issues found.

## Suggestions (Lower Confidence)

No lower-confidence suggestions.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Complexity Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

All source files in this PR are well within healthy complexity thresholds:

### File Length
| File | Lines | Status |
|------|-------|--------|
| `packages/bundler-utils/src/transform.ts` | 88 | Good (< 300) |
| `packages/bundler-utils/src/frontmatter.ts` | 64 | Good (< 300) |
| `packages/bundler-utils/src/errors.ts` | 30 | Good (< 300) |
| `packages/vite-plugin/src/index.ts` | 109 | Good (< 300) |
| `packages/rollup-plugin/src/index.ts` | 83 | Good (< 300) |
| `packages/webpack-loader/src/index.ts` | 83 | Good (< 300) |

### Function Length
Every function is short and focused. The longest functions are `mdsPlugin()` factory functions in the vite-plugin (lines 50-109) and rollup-plugin (lines 44-83), which return plugin objects with 3-4 hook methods. These are structurally required plugin shapes rather than monolithic functions -- the actual logic per hook is 5-15 lines each.

### Cyclomatic Complexity
- `shouldTransform` (frontmatter.ts:32-64): CC ~4 (three guards + one regex check). Uses early returns effectively.
- `transform` (transform.ts:69-86): CC ~2 (linear flow with one conditional for vars).
- `ensureInit` (transform.ts:55-64): CC ~3 (two guards for memoization). Clean promise-caching pattern.
- `ensureTransformer` (webpack-loader:19-38): CC ~3 (null-check, init guard, invariant). Clear.
- `isMdsErrorLike` (errors.ts:10-14): CC ~3. Well-structured type narrowing.
- `formatMdsError` (errors.ts:17-30): CC ~4. Clear if/else-if/else chain with early returns.
- `mdsLoader` (webpack-loader:40-57): CC ~2. Simple try/catch with iteration.
- Plugin `transform` hooks (vite, rollup): CC ~3. Guard, try/catch, iteration.

All functions are below the warning threshold of CC 5.

### Nesting Depth
Maximum nesting depth across all files is 3 levels (in `shouldTransform`'s promise chain: `.then -> try/finally -> if`), which is within the good range (< 3-4).

### Parameter Count
No function has more than 2 parameters. `createMdsTransformer(mds, options?)` is the maximum at 2.

### Boolean Complexity
No complex boolean expressions. All conditions are simple null-checks or single comparisons.

### Duplication Assessment
The three bundler plugins (vite, rollup, webpack) share a similar structure (buildStart/transform pattern), but this is by design -- each adapts to its bundler's plugin API. The shared logic is correctly extracted into `@mds/bundler-utils`, and only the bundler-specific glue code (error reporting shape, HMR handling, watch file registration) varies across plugins. This is appropriate code reuse rather than problematic duplication.

### Test File Complexity
Test files (276, 178, 142, 145 lines) use a clean helper-function pattern (`createMockMds`, `createPluginContext`, `createLoaderContext`) that keeps individual test cases short (5-15 lines each). No deep nesting or complex setup.

### Overall Assessment
This is well-structured code. Functions are short, focused, and easy to understand within the 5-minute rule. The `ensureInit` / `ensureTransformer` promise-caching pattern is the most conceptually complex piece, but it is well-commented with clear error recovery semantics (poisoned promise reset). No refactoring needed.

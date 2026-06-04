# Performance Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Sequential build commands in package.json `build` scripts** - `packages/bundler-utils/package.json:26`, `packages/webpack-loader/package.json:22`
**Confidence**: 82%
- Problem: The build script chains three sequential commands with `&&`: `tsc -p tsconfig.json && tsc -p tsconfig.cjs.json && node -e "..."`. The ESM and CJS TypeScript compilations are independent of each other and could run in parallel. For a monorepo with multiple packages, this doubles the TypeScript compilation time for each package unnecessarily.
- Fix: Run the two `tsc` invocations in parallel. For example:
  ```json
  "build": "tsc -p tsconfig.json & tsc -p tsconfig.cjs.json & wait && node -e \"require('fs').writeFileSync('dist-cjs/package.json', '{\\\"type\\\":\\\"commonjs\\\"}\\n')\""
  ```
  Or use a task runner like `concurrently` or `npm-run-all` for cross-platform compatibility. The `writeFileSync` step must still run after both complete.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`find_unquoted_operator` escape handling order** - `crates/mds-core/src/parser.rs:493-503` (Confidence: 65%) — When inside a string and the current char matches `string_char`, the code sets `in_string = false` on line 494-495, but then the escape check on lines 498-501 can still execute for that same character. If the closing quote happens to be `\`, it would incorrectly skip the next character. In practice this is unlikely since `\` is not `"` or `'`, but the control flow would be cleaner and marginally faster with an early `continue` after toggling `in_string = false`.

- **`new Function` wrapper evaluated once per module load** - `packages/webpack-loader/src/index.ts:10-13` (Confidence: 62%) — The `new Function('id', 'return import(id)')` construct creates a function object at module-load time. This is a one-time cost and correctly cached as a module-level constant, but `new Function` prevents V8 from optimizing the surrounding module in some engines. Since this is inside a Webpack loader (invoked once per `.mds` file with the result cached via `LazyInit`), the practical impact is negligible.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The Rust-side changes (parser, evaluator, validator, AST) are well-structured from a performance perspective:

- The `evaluate_if` function correctly short-circuits on the first matching `@elseif` branch (O(1) best case, O(N) worst case as documented in the PR description).
- The `MAX_ELSEIF_BRANCHES = 256` limit caps the linear scan, preventing pathological chains.
- `parse_condition` is O(N) in the condition string length via the byte-level `find_unquoted_operator` scanner -- efficient for single-pass parsing.
- `evaluate_condition` resolves the dot-path exactly once per condition evaluation via `resolve_condition_path` -- no redundant lookups.
- The `Condition::path()` method returns a borrow (`&[String]`) rather than cloning, which is the correct zero-cost approach for the validator.
- The `values_equal` function takes references and uses pattern matching with no allocations -- optimal for the hot evaluation path.

The only actionable finding is the sequential dual-`tsc` build, which is a MEDIUM concern that affects build-time but not runtime. The condition is straightforward to parallelize.

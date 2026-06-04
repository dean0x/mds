# Security Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

_No blocking security issues found._

## Issues in Code You Touched (Should Fix)

_No should-fix security issues found._

## Pre-existing Issues (Not Blocking)

_No pre-existing security issues above the confidence threshold._

## Suggestions (Lower Confidence)

- **`new Function` as eval proxy** - `packages/webpack-loader/src/index.ts:10` (Confidence: 65%) -- The `new Function('id', 'return import(id)')` pattern is functionally equivalent to `eval` and may trigger CSP violations or static analysis warnings. However, the `id` parameter is only ever called with the hardcoded string `'@mds/mds'` (line 40), never with user-controlled input, and this is a well-documented workaround for TypeScript CJS compilation stripping dynamic `import()`. No exploitable injection path exists given current usage. Consider adding an explicit comment noting the security analysis (e.g., "SECURITY: id is never user-controlled") to prevent future maintainers from passing untrusted input.

- **`Number(f64)` comparison via `==` for floats** - `crates/mds-core/src/evaluator.rs:339` (Confidence: 60%) -- `values_equal` uses Rust's `f64 ==` for numeric comparison, which follows IEEE 754 (NaN != NaN, etc.). While this is correctly documented and tested, floating-point equality comparison can be surprising for template authors (e.g., `0.1 + 0.2 != 0.3`). This is more of a correctness/DX concern than a security vulnerability, but in authorization-sensitive templates (`@if price == 0.0:`), unexpected floating-point behavior could cause unintended content inclusion/exclusion.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

### What was reviewed

Two independent changes were analyzed through a security lens:

1. **Webpack loader CJS compatibility** (`packages/webpack-loader/`, `packages/bundler-utils/`):
   - Dual ESM+CJS build configuration with `new Function` wrapper for preserving dynamic `import()`
   - New `tsconfig.cjs.json` files, package.json export maps, CJS compatibility tests

2. **@if equality/negation/@elseif in MDS template language** (`crates/mds-core/src/`):
   - New `Condition` enum with `Truthy`, `Not`, `Eq`, `NotEq` variants
   - `parse_condition` function with `find_unquoted_operator` string-aware scanner
   - `parse_cond_value` literal parser (strings, numbers, booleans, null)
   - `evaluate_condition` and `values_equal` evaluator logic
   - `MAX_ELSEIF_BRANCHES` (256) resource limit
   - Validator updates for condition path resolution

### Security properties verified

**Injection resistance (template language):**
- Condition values are restricted to literal types only (string, number, boolean, null) -- no variable-to-variable comparison, no expression evaluation, no code execution
- `parse_dot_path` validates all path segments through `is_valid_identifier` (ASCII alphanumeric + underscore only)
- `MAX_DOT_SEGMENTS` (32) and `MAX_ELSEIF_BRANCHES` (256) limits prevent resource exhaustion via pathological input
- `find_unquoted_operator` correctly tracks string context with escape handling -- tested analysis of ordering (close-quote check vs escape check) confirms no bypass; backslash can never equal a quote character
- Bare `=` (assignment-style) is caught and gives a helpful error suggesting `==`
- Double negation (`!!`) and negation-with-comparison (`!var == "x"`) are parse errors
- Unterminated strings are detected and rejected
- Undefined variables in conditions produce errors at validation time (not silently skipped)

**Resource limits (template language):**
- Nesting depth limit tests now use 8 MB threads to ensure the depth limit fires before stack overflow -- this is a correct defensive improvement
- `MAX_ELSEIF_BRANCHES` check uses `>=` (correct -- allows exactly 256, rejects 257th)
- Existing resource limits (MAX_NESTING_DEPTH, MAX_OUTPUT_SIZE, MAX_CALL_DEPTH, MAX_LOOP_ITERATIONS, MAX_TOTAL_ITERATIONS, MAX_FILE_SIZE) remain unchanged and functional

**Strict equality semantics:**
- No type coercion (`Number(3) != String("3")`) -- prevents type confusion attacks in authorization-sensitive templates
- `NaN == NaN` is false (IEEE 754 compliance)
- Cross-type comparisons always return false for `==` and true for `!=`

**CJS compatibility (webpack loader):**
- `new Function` wrapper only invoked with hardcoded `'@mds/mds'` string -- no user-controlled input reaches the function body
- Test helpers (`_resetForTesting`, `_setTransformerForTesting`) gated behind `NODE_ENV=test` check
- No new dependencies introduced; dual build uses existing TypeScript compiler

# Consistency Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### HIGH

**`CondValue::Bool` naming inconsistent with `Value::Boolean`** - `crates/mds-core/src/ast.rs:23`
**Confidence**: 85%
- Problem: The new `CondValue` enum uses `Bool(bool)` as its variant name, while the existing `Value` enum in `value.rs:25` uses `Boolean(bool)`. This naming inconsistency between two parallel enum types representing the same concept (runtime booleans) creates a pattern deviation. When comparing them in `evaluator.rs:340`, the asymmetry is visible: `(Value::Boolean(b), CondValue::Bool(e)) => b == e`.
- Fix: Rename `CondValue::Bool` to `CondValue::Boolean` to match the established `Value` enum naming convention:
```rust
pub enum CondValue {
    String(String),
    Number(f64),
    Boolean(bool),  // was: Bool(bool)
    Null,
}
```

**`MAX_ELSEIF_BRANCHES` (256) inconsistent with `MAX_NESTING_DEPTH` (64) despite comment claiming match** - `crates/mds-core/src/ast.rs:8-9`
**Confidence**: 90%
- Problem: The doc comment on `MAX_ELSEIF_BRANCHES` states "Matches MAX_NESTING_DEPTH to prevent pathological chains", but `MAX_NESTING_DEPTH` was reduced from 256 to 64 in this PR (parser.rs:17) while `MAX_ELSEIF_BRANCHES` remained at 256. The comment is now factually wrong -- the two constants do not match.
- Fix: Either update `MAX_ELSEIF_BRANCHES` to 64 to actually match, or update the comment to explain why a different limit is appropriate:
```rust
/// Maximum number of @elseif branches on a single @if block.
/// 256 is generous for any real template; unlike nesting depth,
/// @elseif branches do not create additional parse stack frames.
pub const MAX_ELSEIF_BRANCHES: usize = 256;
```

### MEDIUM

**CJS build script uses `&` (background) instead of `&&` (sequential) for parallel tsc** - `packages/webpack-loader/package.json:23`, `packages/bundler-utils/package.json:27`
**Confidence**: 80%
- Problem: The build scripts use `tsc -p tsconfig.json & tsc -p tsconfig.cjs.json & wait && node -e "..."`. While `&` + `wait` does run both tsc processes in parallel and waits for both, `wait` returns the exit status of the last background job only (POSIX). If the first tsc fails but the second succeeds, `wait` returns 0 and the build appears to succeed. The existing build pattern was `tsc -p tsconfig.json` (single command, fail-fast). The new pattern is internally consistent between the two packages, but diverges from fail-fast conventions.
- Fix: Use `wait` with explicit PID tracking or switch to sequential builds for correctness:
```json
"build": "tsc -p tsconfig.json && tsc -p tsconfig.cjs.json && node -e \"...\""
```
Or if parallelism is important, use a tool like `concurrently` or `npm-run-all` that properly surfaces failures.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`findProjectRoot` uses sync `existsSync` in an otherwise async codebase** - `packages/mds/src/util/module-scanner.ts:29` (Confidence: 65%) -- The module-scanner uses async I/O everywhere else (`open`, `realpath`, `readFile`), but `findProjectRoot` uses synchronous `existsSync`. This is called once per `buildModulesMap` invocation so the performance impact is negligible, but it deviates from the async-first pattern in this file.

- **Spec says "Maximum 256 `@elseif` branches" but nesting depth is 64** - `spec.md:168` (Confidence: 70%) -- The spec documents the 256 branch limit, which is internally consistent with `MAX_ELSEIF_BRANCHES` in code. However, if the constant is updated to match `MAX_NESTING_DEPTH` (per the HIGH finding above), the spec will also need updating.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The new code demonstrates strong internal consistency: the CJS build pattern, tsconfig structures, and test file organization are mirrored identically between `bundler-utils` and `webpack-loader`. The Rust-side additions (Condition enum, parse_condition, evaluate_condition) follow existing naming and error-handling conventions well. The `CondValue::Bool` vs `Value::Boolean` naming mismatch and the stale `MAX_ELSEIF_BRANCHES` comment are the two items that should be addressed before merge (applies ADR-001 -- pre-merge quality gate).

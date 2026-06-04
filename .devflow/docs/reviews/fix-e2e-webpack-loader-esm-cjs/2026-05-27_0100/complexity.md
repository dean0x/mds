# Complexity Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicated dot-path resolution boilerplate in `evaluate_condition` (4 occurrences)** - `crates/mds-core/src/evaluator.rs:347-377`
**Confidence**: 90%
- Problem: All four arms of `evaluate_condition` repeat the identical 4-line pattern: extract `path.first()`, produce the same error, call `resolve_dot_path(root, &path[1..], scope)?`. This is duplicated logic within a single function, increasing maintenance burden and cyclomatic complexity.
- Fix: The `Condition` enum already has a `path()` method. Extract the common resolution into a helper at the top of the function, then match only on the semantic operation:

```rust
fn evaluate_condition(condition: &Condition, scope: &Scope) -> Result<bool, MdsError> {
    let path = condition.path();
    let root = path.first().ok_or_else(|| {
        MdsError::syntax("internal error: @if block has empty condition path")
    })?;
    let value = resolve_dot_path(root, &path[1..], scope)?;

    match condition {
        Condition::Truthy(_) => Ok(value.is_truthy()),
        Condition::Not(_) => Ok(!value.is_truthy()),
        Condition::Eq(_, expected) => Ok(values_equal(&value, expected)),
        Condition::NotEq(_, expected) => Ok(!values_equal(&value, expected)),
    }
}
```

Note: The `resolve_condition_path` helper already exists in the final file at line 347 but is unused by the actual `evaluate_condition` implementation at lines 355-361. The current code inlines the resolution in every arm instead of using the helper. This is a direct contradiction -- the helper was written but not adopted.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`parse_if_block` growing in responsibilities** - `crates/mds-core/src/parser.rs:216-282` (Confidence: 65%) -- With the addition of @elseif branch collection, `parse_if_block` now handles condition parsing, then-body parsing, a while-loop for @elseif collection (with its own condition extraction and body parsing), and @else body parsing. At ~66 lines it is within acceptable range but trending upward. Consider extracting @elseif collection into a dedicated method if further condition types are added.

- **`find_unquoted_operator` byte-level scanning with mixed state** - `crates/mds-core/src/parser.rs:483-528` (Confidence: 62%) -- The function tracks `in_string`, `string_char`, and escaped character state while scanning for operators using raw byte indexing. This is a hand-rolled mini-lexer. The complexity is justified for correctness (handling `==` inside quoted strings), and the function is well-bounded at 45 lines. No action needed unless more operators are added.

- **`parse_condition` sequential dispatch pattern** - `crates/mds-core/src/parser.rs:536-604` (Confidence: 60%) -- The function uses sequential if-let/if chains to dispatch across negation, equality/inequality, bare-equals check, and truthy. At ~68 lines with 4 exit paths this is slightly above the "warning" threshold for cyclomatic complexity but each path is clear and well-commented. Acceptable for a parser entry point.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new code is well-structured overall. The parser rewrite introduces three focused functions (`parse_condition`, `find_unquoted_operator`, `parse_cond_value`) that each handle a single responsibility with clear boundaries and bounded iteration. The AST types (`Condition`, `CondValue`) encode variants cleanly with an enum, keeping downstream match exhaustiveness enforced by the compiler.

The single blocking item is the duplicated dot-path resolution in `evaluate_condition`, where an existing helper (`resolve_condition_path`) was written but not used, and the same 4-line pattern is repeated in all four match arms. This is a straightforward deduplication fix that reduces the function from ~25 lines to ~10.

The webpack loader changes (`packages/webpack-loader/src/index.ts`) introduce no complexity concerns -- the `_esmImport` wrapper and CJS build configuration are clean and minimal.

All new functions are well within acceptable complexity thresholds. No unbounded loops were introduced. The `MAX_ELSEIF_BRANCHES` limit (256) bounds the @elseif collection loop. The `find_unquoted_operator` scanner is bounded by input length. Test coverage is thorough with 40+ new integration tests covering all condition variants.

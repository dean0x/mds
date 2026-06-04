# Complexity Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**PR**: #34

## Issues in Your Changes (BLOCKING)

### HIGH

**`parse_condition` approaches complexity threshold (62 lines, ~4 distinct concern blocks)** - `crates/mds-core/src/parser.rs:570`
**Confidence**: 82%
- Problem: `parse_condition` is 62 lines with 4 sequential concern blocks: negation prefix handling, equality/inequality operator handling, bare `=` error-hint handling, and default truthy fallback. Each block contains its own validation and early returns. The function is at the upper boundary of the "Warning" range (30-50 lines) and just into the "Critical" zone (>50). Cyclomatic complexity is approximately 9 (multiple `if` branches with nested sub-conditions). The nesting depth stays reasonable (max 3) thanks to early returns, but the sheer number of distinct code paths handled in one function is high.
- Fix: Consider extracting the negation branch (lines 574-592) into a `parse_negation_condition(rest: &str) -> Result<Condition, MdsError>` helper. This would bring `parse_condition` to ~40 lines and give each concern block a named home. This is a "should consider" rather than a hard block because the current structure uses early returns effectively and reads top-to-bottom.

**`parse_directive` has 8 sequential directive checks (58 lines)** - `crates/mds-core/src/parser.rs:174`
**Confidence**: 80%
- Problem: `parse_directive` is 58 lines with 8 sequential `if` checks for different directive types, plus 3 additional error-hint blocks for `@else`, `@elseif`, and `@elseif:`. This PR added 3 new `@elseif`-related error hint blocks (lines 215-227), pushing the function from the prior ~40 lines to 58. The cyclomatic complexity is approximately 11 (each `if`/`if let` is a path). While each branch is simple (1-3 lines), the function as a whole requires reading through all 8 directive types to understand which one matched.
- Fix: The directive dispatch is inherently a match/dispatch pattern. A common Rust pattern is a lookup table or a single `match` on the first word of the directive. However, given that each branch has slightly different parsing logic (some use `strip_prefix`, some use `is_directive_token`), the current sequential `if` chain is pragmatic. The new `@elseif` error hints are well-placed and follow the existing pattern. No immediate refactor required, but monitor growth -- another 2-3 directives would warrant a dispatch table.

### MEDIUM

**`parse_cond_value` sequential type-dispatch (49 lines)** - `crates/mds-core/src/parser.rs:455`
**Confidence**: 83%
- Problem: `parse_cond_value` is 49 lines handling 5 literal types (quoted string, unterminated string error, boolean, null, numeric) in a sequential `if` chain. While each branch is simple and the function reads clearly, it is at the boundary of the "Warning" range. The cyclomatic complexity is approximately 7.
- Fix: No immediate action required. The function is well-structured with early returns and clear comments. If more literal types are added in the future (e.g., arrays, regex), consider a dispatch table or match on the first character.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`parser.rs` source section is 1137 lines (excluding tests)** - `crates/mds-core/src/parser.rs`
**Confidence**: 85%
- Problem: The non-test portion of `parser.rs` is 1137 lines, well above the 500-line "Critical" file-length threshold. This PR added approximately 200 lines of new parser logic (condition parsing, elseif branch collection, helper functions). The file now contains both the `Parser` struct methods (block parsing) and a large set of free functions (condition parsing, import/export parsing, interpolation parsing, string escaping, argument parsing). These are two distinct responsibility groups.
- Fix: Consider splitting `parser.rs` into two modules in a future PR: `parser.rs` for the `Parser` struct and block-level parsing, and `parser/expressions.rs` (or `condition.rs`) for the free functions that parse conditions, interpolations, arguments, and imports. This would bring each file under 600 lines. This is a should-fix, not a blocker, since the PR is already large and this is a structural improvement best done separately.

## Pre-existing Issues (Not Blocking)

(No CRITICAL pre-existing complexity issues found in unchanged code.)

## Suggestions (Lower Confidence)

- **`find_unquoted_operator` uses byte-level scanning with manual state tracking** - `crates/mds-core/src/parser.rs:515` (Confidence: 65%) -- The function manually tracks `in_string` and `string_char` state while scanning bytes. This is correct and performant, but a small state-machine enum (`enum ScanState { Outside, InString(u8) }`) would make the state transitions self-documenting. Low priority.

- **`parse_body` now takes two slice parameters** - `crates/mds-core/src/parser.rs:117` (Confidence: 62%) -- The function signature grew from 1 terminator parameter to 2 (`exact_terminators` and `prefix_terminators`). The dual-slice approach is clear when reading the call sites, but if more terminator types are added, consider a `TerminatorSet` struct. Current state is fine.

- **Module-level `projectRootCache` in module-scanner.ts** - `packages/mds/src/util/module-scanner.ts:25` (Confidence: 70%) -- The cache is a module-level `Map` that grows monotonically without any eviction strategy. In long-running processes (e.g., Webpack watch mode with many entry points), this could accumulate stale entries. Given the project is pre-release and the cache key is the start directory (bounded by unique directory count), this is low risk. The extraction of `_findProjectRootUncached` is a good refactoring that improves testability.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions

1. **Acknowledge** the `parser.rs` file-length growth and plan a module split in a follow-up (does not block this PR).
2. The `parse_condition` function is at the complexity boundary but uses early returns effectively -- acceptable as-is, but extracting the negation branch would improve readability.

### Positive Observations

- The `collect_elseif_branches` extraction (noted in PRIOR_RESOLUTIONS as a Cycle 2 fix) successfully reduced `parse_if_block` from ~66 lines to 34 lines. This is excellent complexity management. *applies ADR-001* (pre-merge quality gate caught this).
- New free functions (`parse_dot_path`, `parse_cond_value`, `find_unquoted_operator`, `parse_condition`) are well-decomposed: each handles a single parsing concern with clear doc comments.
- The `evaluate_condition` and `values_equal` functions in the evaluator are exemplary: 11 and 8 lines respectively, single-responsibility, exhaustive match arms.
- The `evaluate_if` function went from a monolithic condition-check-and-branch to a clean 3-step structure (primary condition, elseif loop, else fallback) in just 24 lines.
- `Condition::path()` and `Condition::root()` on the AST enum centralize dot-path extraction, avoiding repeated match arms in evaluator and validator.
- The `findProjectRoot` function in module-scanner.ts properly separates caching (`findProjectRoot`) from logic (`_findProjectRootUncached`), with a bounded traversal loop (`MAX_TRAVERSAL_DEPTH`).

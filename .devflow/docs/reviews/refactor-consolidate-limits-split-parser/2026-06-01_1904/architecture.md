# Architecture Review Report

**Branch**: refactor/consolidate-limits-split-parser -> main
**Date**: 2026-06-01

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Incomplete limits consolidation -- evaluator, resolver, value, and fs constants remain scattered** - `crates/mds-core/src/evaluator.rs:11-24`, `crates/mds-core/src/resolver.rs:31`, `crates/mds-core/src/value.rs:7`, `crates/mds-core/src/fs.rs:19`
**Confidence**: 82%
- Problem: The PR consolidates 5 constants into `limits.rs` (MAX_NESTING_DEPTH, MAX_ELSEIF_BRANCHES, MAX_FILE_SIZE, MAX_TRAVERSAL_DEPTH, MAX_DOT_SEGMENTS) but leaves 8 other resource-limit constants scattered across 4 modules: MAX_CALL_DEPTH, MAX_LOOP_ITERATIONS, MAX_TOTAL_ITERATIONS, MAX_OUTPUT_SIZE, MAX_WARNINGS (evaluator.rs), MAX_IMPORT_DEPTH (resolver.rs), MAX_VALUE_DEPTH (value.rs), MAX_PATH_SEGMENTS (fs.rs). The stated goal is "single source of truth" for limits. The SECURITY.md table already documents these as resource limits in their current locations. Leaving them scattered undermines the architectural intent of this refactor and means future contributors must look in N different files to find or audit all resource limits.
- Fix: This is a "should fix while here" item. Consider consolidating all MAX_ resource-limit constants into `limits.rs` in this PR, or explicitly document the scoping rationale (e.g., evaluator limits are private to evaluator and single-use). A follow-up issue is acceptable if scope creep is a concern, but the architectural debt should be tracked.

## Pre-existing Issues (Not Blocking)

No pre-existing architecture issues at CRITICAL severity.

## Suggestions (Lower Confidence)

- **`use helpers::*` glob import in parser.rs** - `crates/mds-core/src/parser.rs:14` (Confidence: 65%) -- The glob import `use helpers::*` re-exports all 20+ functions from parser_helpers.rs into the parser module namespace. While convenient, this makes it harder to trace which helper functions are actually used in parser.rs vs only used in tests. An explicit import list would improve readability and make dead-code detection easier. However, since parser_helpers is a `#[path]` submodule (not a sibling), the blast radius is contained and this is stylistic.

- **Evaluator constants are module-private (not `pub(crate)`) while limits.rs uses `pub(crate)`** - `crates/mds-core/src/evaluator.rs:11-24` vs `crates/mds-core/src/limits.rs` (Confidence: 62%) -- The visibility mismatch between the two groups of limits (evaluator uses `const`, limits.rs uses `pub(crate) const`) suggests they were intentionally left separate because the evaluator limits are single-module-private. If so, documenting this distinction would clarify the consolidation boundary for future contributors.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This is a well-executed structural refactor that achieves its stated goals. The key architectural observations:

**Strengths:**
- **Clean module decomposition** (applies ADR-001): parser.rs dropped from ~1820 to ~423 lines by extracting helpers and tests into focused submodules. The `#[path]` attribute pattern follows existing crate precedent (error.rs/error_tests.rs).
- **Proper layering**: `limits.rs` has zero internal dependencies -- it sits at the bottom of the dependency graph with only `pub(crate)` constants. No circular dependencies introduced.
- **Correct visibility boundaries**: `pub(super)` for helper functions limits them to the parser module tree, `pub(crate)` for `is_valid_identifier` where cross-module access is needed (validator.rs). The re-export chain `helpers::is_valid_identifier -> parser::is_valid_identifier -> crate` is clean.
- **SRP improvement**: Parser state management (Parser struct, parse_body, parse_directive, parse_if_block, parse_for_block, parse_define_block) is cleanly separated from stateless helper functions (condition parsing, import/export parsing, interpolation parsing, string utilities).
- **No behavioral changes**: 591 tests pass, the diff is purely structural reorganization.

**The one condition**: The incomplete consolidation (8 remaining scattered MAX_ constants) should either be addressed in this PR or tracked as a follow-up issue to complete the architectural intent. The current state is an improvement but leaves the "single source of truth" goal partially achieved.

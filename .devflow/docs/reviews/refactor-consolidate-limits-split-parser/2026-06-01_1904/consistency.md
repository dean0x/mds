# Consistency Review Report

**Branch**: refactor/consolidate-limits-split-parser -> main
**Date**: 2026-06-01

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Incomplete constant consolidation into limits.rs** - `crates/mds-core/src/evaluator.rs:11-24`, `crates/mds-core/src/resolver.rs:31`, `crates/mds-core/src/value.rs:7`
**Confidence**: 85%
- Problem: The PR goal is to "consolidate 5 cross-module constants into limits.rs" (closing #35). However, 7 additional `MAX_*` constants remain scattered across three other modules: `MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`, `MAX_WARNINGS` in `evaluator.rs`, `MAX_IMPORT_DEPTH` in `resolver.rs`, and `MAX_VALUE_DEPTH` in `value.rs`. The SECURITY.md resource limits table still references `evaluator.rs` and `resolver.rs` for these remaining constants, creating an inconsistent split where some limits live in `limits.rs` and others remain in their original modules. The consolidation was partial rather than complete, which means the codebase now has *two* conventions for where limit constants are defined.
- Fix: Either (a) move the remaining 7 constants into `limits.rs` to complete the consolidation, or (b) document explicitly in `limits.rs` (and the PR description) that only parser/fs-related limits were consolidated, with evaluator/resolver limits intentionally left in place. Without this clarification, the next developer will not know whether the remaining constants are "pending migration" or "intentionally local."

### MEDIUM

**SECURITY.md location references inconsistent after partial consolidation** - `SECURITY.md:56-61`
**Confidence**: 82%
- Problem: The SECURITY.md resource limits table was updated for `MAX_FILE_SIZE`, `MAX_NESTING_DEPTH`, `MAX_ELSEIF_BRANCHES`, and `MAX_DOT_SEGMENTS` to point to `limits.rs`, but the remaining five limits still reference `evaluator.rs` and `resolver.rs`. This is factually correct today, but creates an inconsistent "some here, some there" pattern in the documentation. A reader of SECURITY.md may wonder why the pattern differs. This is a direct consequence of the incomplete consolidation above.
- Fix: If all constants are consolidated into `limits.rs`, update the remaining Location column entries accordingly. If the partial consolidation is intentional, add a brief note in SECURITY.md explaining that evaluator-specific runtime limits are colocated with their enforcement code.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`parse_single_arg` test-only helper visibility** - `crates/mds-core/src/parser_helpers.rs:630-633` (Confidence: 65%) -- The `parse_single_arg` function is `#[cfg(test)] pub(super)`, meaning it compiles only in test mode but is visible to the parent module. This is fine, but it could also be `pub(super)` without `#[cfg(test)]` since it is a trivial zero-cost wrapper. The `#[cfg(test)]` gate is not harmful but is slightly unusual for a function that merely delegates to `parse_single_arg_inner(s, 0)`. Minor style point.

- **Module split pattern matches existing codebase convention** (Confidence: positive observation) -- The use of `#[path = "parser_helpers.rs"]` and `#[path = "parser_tests.rs"]` with flat sibling files follows the precedent set by `error.rs` / `error_tests.rs`. This is consistent with the codebase's established convention for extracting test modules.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

Additional note: 1 MEDIUM-severity observation about SECURITY.md is a direct corollary of the HIGH finding -- resolving one resolves both.

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The module split (parser.rs -> parser_helpers.rs + parser_tests.rs) is cleanly executed and follows established codebase conventions (`#[path = ...]` pattern from error.rs). Naming is consistent, visibility modifiers are appropriate (`pub(super)` for helpers, `pub(crate)` for `is_valid_identifier`), and the doc comments follow existing style.

The main consistency concern is the incomplete constant consolidation: the PR moves 5 constants into `limits.rs` but leaves 7 analogous constants in their original modules. This creates a split convention that could confuse future contributors. The fix is either to complete the consolidation or to explicitly document the boundary. This applies ADR-002: the PR claims to close #35 (consolidate constants into limits.rs), so the actual changes should be verified against the issue scope to confirm the consolidation is complete or intentionally scoped.

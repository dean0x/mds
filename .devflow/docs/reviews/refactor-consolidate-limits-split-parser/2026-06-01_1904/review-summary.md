# Code Review Summary

**Branch**: refactor/consolidate-limits-split-parser -> main
**Date**: 2026-06-01_1904

## Merge Recommendation: CHANGES_REQUESTED

This refactoring successfully consolidates 5 resource-limit constants into `limits.rs` and cleanly splits the 1820-line `parser.rs` into focused modules. The architectural intent is sound and well-executed. However, three HIGH-confidence issues must be resolved before merge:

1. **Incomplete constant consolidation** (95% confidence, blocking) — The PR consolidates only 5 of 13 resource-limit constants, leaving 8 others scattered. This undermines the stated goal of a "single source of truth" for limits.
2. **Missing module doc comment on parser.rs** (85% confidence, blocking) — As the orchestrating parent module for a 3-file split, it should document the new structure.
3. **Missing doc comments on 4 public functions** (82% confidence, blocking) — Four functions in `parser_helpers.rs` lack `///` doc comments, creating inconsistency with the rest of the module.

All three are fixable in 10 minutes. After these are addressed, this PR will be production-ready for merge.

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 2 | 1 | 0 | 3 |
| Should Fix | 0 | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 | 1 |

## Blocking Issues

**Incomplete constant consolidation** — `crates/mds-core/src/evaluator.rs:11-24`, `crates/mds-core/src/resolver.rs:31`, `crates/mds-core/src/value.rs:7` (95% confidence, HIGH)
- Problem: The PR goal is "consolidate constants into limits.rs" (closes #35). However, only 5 of 13 resource-limit constants were consolidated. Remaining 8 constants (`MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`, `MAX_WARNINGS` in `evaluator.rs`; `MAX_IMPORT_DEPTH` in `resolver.rs`; `MAX_VALUE_DEPTH` in `value.rs`) remain scattered. This creates a split convention where some limits live in `limits.rs` and others remain local, undermining the "single source of truth" architectural goal stated in the PR.
- Fix: Either (a) consolidate all 13 constants into `limits.rs` in this PR to complete the refactor, or (b) document explicitly in `limits.rs` and the PR description that only parser/fs/ast/resolver-interaction limits were consolidated, with evaluator/value limits intentionally kept local for module privacy. Without clarification, future contributors will not know whether remaining constants are "pending migration" or "intentionally local."
- (Deduped from architecture.md 82% + consistency.md 85% → 95%, both flagged the same issue across different reviews)

**Missing module-level doc comment on parser.rs** — `crates/mds-core/src/parser.rs:1` (85% confidence, HIGH)
- Problem: After the split, `parser.rs` was restructured from ~1820 lines to ~423 lines and now serves as the orchestrating parent module delegating to `parser_helpers.rs` and `parser_tests.rs` via `#[path]` includes. Both child modules have `//!` doc comments, but the parent has none. For a module significantly restructured, the new responsibility split should be documented for future maintainers.
- Fix: Add a module doc comment at the top of `parser.rs` (before `use` statements):
  ```rust
  //! Recursive-descent parser: converts a token stream into a `Module` AST.
  //!
  //! The parser is split across three files:
  //! - `parser.rs` (this file) — `Parser` struct, top-level parse entry point,
  //!   block-level parsing (`parse_body`, `parse_if_block`, `parse_for_block`,
  //!   `parse_define_block`, `parse_directive`).
  //! - `parser_helpers.rs` — low-level parsing primitives (conditions, imports,
  //!   exports, interpolation expressions, string utilities).
  //! - `parser_tests.rs` — unit and integration tests.
  ```

**Missing doc comments on 4 public functions** — `crates/mds-core/src/parser_helpers.rs:556`, `parser_helpers.rs:631`, `parser_helpers.rs:635`, `parser_helpers.rs:714` (82% confidence, MEDIUM/blocking)
- Problem: Functions `parse_args_inner`, `parse_single_arg`, `parse_single_arg_inner`, and `is_valid_identifier` lack `///` doc comments, while every other public function in the same file has them. This creates inconsistency within the newly extracted module.
- Fix: Add brief `///` doc comments to each:
  ```rust
  /// Recursively parse function call arguments with depth tracking.
  pub(super) fn parse_args_inner(...) { ... }

  /// Parse a single argument string (test-only convenience wrapper).
  #[cfg(test)]
  pub(super) fn parse_single_arg(...) { ... }

  /// Parse a single argument string with depth tracking for nested calls.
  pub(super) fn parse_single_arg_inner(...) { ... }

  /// Return true if `s` is a valid ASCII identifier (leading alpha/underscore, then alphanumeric/underscore).
  pub(crate) fn is_valid_identifier(...) { ... }
  ```

## Suggestions (Lower Confidence)

These are informational and do not block merge but may be worth considering:

- **SECURITY.md location references inconsistent after partial consolidation** — `SECURITY.md:56-61` (82% confidence) — The resource limits table was updated for 4 constants to point to `limits.rs`, but 5 others still reference `evaluator.rs` and `resolver.rs`. This is factually correct today but creates a split pattern. If all constants are consolidated (as above), update remaining Location entries. If partial consolidation is intentional, add a note explaining that evaluator-specific limits are colocated with enforcement code.

- **CHANGELOG wording slightly inaccurate** — `CHANGELOG.md:12` (80% confidence, pre-existing) — The entry says "Consolidated parser constants" but only 2 of 5 constants came from `parser.rs`. Consider rewording to: "Consolidated cross-module resource-limit constants into `crates/mds-core/src/limits.rs`"

- **Extracted helper functions lack direct unit tests** — `parser_helpers.rs` (65% confidence, pre-existing) — Functions like `parse_dot_path`, `parse_for_vars`, `unescape_string` are tested indirectly. This is acceptable for a move-only refactor, but targeted unit tests would improve fault isolation in future changes.

## Action Plan

1. **Consolidate all 13 resource-limit constants into `limits.rs`** (or document scoping rationale if intentionally partial)
2. **Add module doc comment to `parser.rs`** explaining the 3-file split structure
3. **Add `///` doc comments to the 4 undocumented functions** in `parser_helpers.rs`
4. **Update SECURITY.md** remaining Location entries if all constants are consolidated
5. **(Optional) Update CHANGELOG wording** to be more precise

After these are complete, re-run tests and tests should pass with zero warnings.

## Convergence Status

**Cycle**: 2
**Prior Resolution**: Available (18 issues total)
**Prior FP Ratio**: 72% (13 FP of 18) — Indicates first-cycle review had high false-positive rate
**Assessment**: Converging — Most false positives from Cycle 1 were correctly identified and not re-raised. Current cycle identifies 3 genuine blocking issues that require fixes before merge. No conflict with prior findings; instead, building on prior analysis with focused, actionable fixes.

### Cross-Cycle Analysis

Prior cycle (Cycle 1) fixed 5 issues (SECURITY.md missing row, module doc comments, CHANGELOG empty) and classified 13 as false positives (pre-existing code, deliberate design choices). Current cycle respects those FP classifications — no pre-existing code was re-raised. Instead, current reviewers identified 3 new HIGH/MEDIUM issues in the current branch state:

1. Incomplete consolidation (architectural gap not fully addressed)
2. Missing parser.rs doc comment (documentation gap introduced by the split)
3. Missing function doc comments (consistency gap in newly extracted module)

These are legitimate gaps in the current implementation that should be fixed. The convergence is healthy: reviewers are not flip-flopping on false positives but rather focusing on genuine gaps that can be resolved.

**FP Handling**: Prior FP rate of 72% reflects the challenges of cycle-1 reviews in early-stage refactors. Cycle 2 demonstrates better signal-to-noise by filtering FPs and focusing only on actionable findings with high confidence (80%+).

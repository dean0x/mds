# Resolution Summary

**Branch**: refactor/consolidate-limits-split-parser -> main
**Date**: 2026-06-01_1839
**Review**: .devflow/docs/reviews/refactor-consolidate-limits-split-parser/2026-06-01_1839
**Command**: /resolve

## Decisions Citations

- applies ADR-001 — batch-1, batch-2, batch-3 (squash merge with pre-merge gate)
- applies ADR-002 — batch-1, batch-3 (verify PR content addresses linked issues)

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 18 |
| Fixed | 5 |
| False Positive | 13 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| SECURITY.md missing MAX_ELSEIF_BRANCHES row | SECURITY.md:52 | d867b0f |
| parser_helpers.rs missing module-level doc comment | parser_helpers.rs:1 | 3e23aa9 |
| parse_args_inner missing state machine annotation | parser_helpers.rs:557 | 3e23aa9 |
| CHANGELOG [Unreleased] section empty | CHANGELOG.md:8 | 2b6ed4d |
| parser_tests.rs missing module-level doc comment | parser_tests.rs:1 | 2b6ed4d |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| MAX_ELSEIF_BRANCHES visibility narrowing | limits.rs:18 | `pub` in `ast.rs` was effectively `pub(crate)` because `ast` module is `pub(crate)` in lib.rs. Zero external consumers. No API change. |
| SECURITY.md inconsistent location granularity | SECURITY.md:52 | Table is factually correct — mix reflects deliberate scoping (cross-module vs module-private). |
| parser_helpers.rs 733 lines exceeds threshold | parser_helpers.rs:0 | Valid but out of scope for structural refactor. Better than the original 1820-line file. Follow-up candidate. |
| parse_dot_expr 6 parameters | parser_helpers.rs:401 | Pre-existing moved code, single caller within same file. SourceCtx is valid follow-up. |
| parse_import_directive 61 lines | parser_helpers.rs:233 | Pre-existing moved code, out of scope for structural refactor. |
| parse_define_block 60 lines | parser.rs:363 | Pre-existing, doing one job well, HashSet loop is only 12 lines. |
| parse_body 56 lines with 8-arm match | parser.rs:118 | Pre-existing, idiomatic Rust pattern (exhaustive enum match). |
| parse_directive 57 lines with 9 branches | parser.rs:175 | Pre-existing, linear dispatch list (1:1 with directive types). |
| is_valid_identifier unused pub(crate) | parser.rs:13 | Incorrect — has 15+ callers across parser_helpers.rs and parser.rs. |
| strip_leading_newline O(n) remove(0) | parser_helpers.rs:703 | Pre-existing, rare path, negligible on small slices. |
| parse_args_inner char-by-char | parser_helpers.rs:559 | Pre-existing, low-confidence, parser for human-authored templates. |
| #[path = ...] unconventional | parser.rs:11 | Deliberate design choice matching error.rs/error_tests.rs precedent. |
| parse_single_arg #[cfg(test)] in non-test file | parser_helpers.rs:609 | Matches codebase-wide convention (11+ files use this pattern). |

## Deferred to Tech Debt
(none)

## Blocked
(none)

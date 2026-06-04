# Resolution Summary

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16_2337
**Review**: .docs/reviews/feat-mds-three-enhancements/2026-05-16_2337
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 13 |
| Fixed | 11 |
| False Positive | 1 |
| Deferred | 1 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Cross-layer import of MAX_DOT_SEGMENTS | src/evaluator.rs, src/parser.rs, src/limits.rs (new) | 52558b6 |
| Inconsistent error message wording | src/evaluator.rs:104-106 | 52558b6 |
| path_so_far eager allocation on every resolve_dot_path call | src/evaluator.rs:112 | 52558b6 |
| run_loop_body unnecessary clone via borrowed slice | src/evaluator.rs:353-357 | 52558b6 |
| evaluate_for 64 lines — extracted evaluate_for_array | src/evaluator.rs:405-468 | 52558b6 |
| Missing tests for MAX_DOT_SEGMENTS (5 locations) | tests/integration.rs | f28cdd7 |
| Error messages not asserted (full traversed path) | tests/integration.rs | f28cdd7 |
| for_key_value_dot_path_object weak assertions | tests/integration.rs:3381 | f28cdd7 |
| Duplicated dot-path validation in parse_single_arg_inner | src/parser.rs:720-739 | fd84987 |
| QualifiedCall path missing identifier validation | src/parser.rs:529-542 | fd84987 |
| validate_file_type asymmetry with strip_type_mds | src/resolver.rs:717-722 | fd84987 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| evaluate_for_key_value takes ownership unnecessarily | src/evaluator.rs:365-372 | Reviewer acknowledged "acceptable for now." Single call site owns the data — no clone needed. The issue is about a hypothetical future caller that doesn't exist. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| parse_args_inner at 59 lines with 4-level nesting | src/parser.rs:634-692 | Pre-existing function not modified in this PR. Refactoring the stateful character loop (string escaping, paren depth, in-string flag) requires careful restructuring — not safe as a one-pass extraction. |

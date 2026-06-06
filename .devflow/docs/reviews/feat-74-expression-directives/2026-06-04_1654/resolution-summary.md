# Resolution Summary

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04_1654
**Review**: .devflow/docs/reviews/feat-74-expression-directives/2026-06-04_1654
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 19 |
| Fixed | 13 |
| False Positive | 2 |
| Deferred | 4 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Module docstring stale (parse_dot_path removed, new functions missing) | parser_helpers.rs:7 | 4bb3c1f |
| parse_condition docstring not updated for expression support | parser_helpers.rs:520 | 4bb3c1f |
| Silent close-paren absorption in strip_trailing_directive_colon | parser_helpers.rs:67 | 4bb3c1f |
| evaluate_condition scope mutation undocumented | evaluator.rs:414 | 4bb3c1f |
| Condition PartialEq doc references CondValue instead of Expr | ast.rs:37 | 4bb3c1f |
| CondValue docstring says "RHS of equality condition" (now only for defaults) | ast.rs:8 | 4bb3c1f |
| split() allocates full Vec before MAX_ARRAY_ELEMENTS check | builtins.rs:260 | 4bb3c1f |
| ForBlock import inconsistent (inline vs top-level) | validator.rs:1 | 4bb3c1f |
| validate_for_node duplicates validate_expr logic for Call/QualifiedCall | validator.rs:89 | 4bb3c1f |
| @elseif missing targeted unterminated-string error | parser.rs:311 | 8da3c26 |
| Missing NotEq (!=) operator tests with expressions | parser_tests.rs | 8da3c26 |
| Missing OR (||) operator test with expression-based operands | parser_tests.rs | 8da3c26 |
| Missing @for with qualified call test | parser_tests.rs | 8da3c26 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| evaluate_condition_value trivial wrapper | evaluator.rs:405 | Already removed by Simplifier in prior commit (ec01c78) |
| @define uses strip_suffix(':') instead of strip_trailing_directive_colon | parser.rs:373 | @define header colon is always final character — no string/paren ambiguity possible. strip_suffix(':') is correct here. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| Duplicated quote/paren scanning state machines (5+ occurrences) | parser_helpers.rs:30-645 | Architectural overhaul — extracting ScanState struct touches 5+ scanner functions; risk of subtle behavioral divergence. Deferred until grammar expansion warrants consolidation. |
| CondValue/Expr literal type duplication | ast.rs:12-123 | Cross-cutting AST type change — removing CondValue affects Param.default, parse_cond_value, condvalue_to_value, and all test code. No functional risk until literal representation changes. |
| parse_expr_inner duplicates parse_interpolation_expr | parser_helpers.rs:128-246 | Moderate refactoring — making parse_interpolation_expr delegate to parse_expr_inner requires preserving offset tracking for interpolation spans. |
| parse_simple_condition complexity (~94 lines, CC ~12) | parser_helpers.rs:569-662 | Moderate refactoring — extracting bare-= detection helper. No functional risk but improves readability. |

## Blocked
(none)

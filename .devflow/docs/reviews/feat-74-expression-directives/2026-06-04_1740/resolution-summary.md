# Resolution Summary

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04_1740
**Review**: .devflow/docs/reviews/feat-74-expression-directives/2026-06-04_1740
**Command**: /resolve

## Decisions Citations

- applies ADR-008 — batch-1, complexity-MEDIUM-parser_helpers:multi (bundling deferred scanner consolidation with this PR)
- applies ADR-008 — batch-3, performance-MEDIUM-evaluator:147 (deferring evaluate_expr clone as architectural change)

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 16 |
| Fixed | 14 |
| False Positive | 0 |
| Deferred | 2 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| parse_expr_inner escape-aware string literal detection | parser_helpers.rs:27-67 | e5a07c2 |
| Extract has_bare_equals from parse_simple_condition (94→55 lines) | parser_helpers.rs:69-104 | e5a07c2 |
| Extract parse_call_expr from parse_expr_inner (120→65 lines) | parser_helpers.rs:165-232 | e5a07c2 |
| Align literal-rejection error messages to consistent pattern | parser_helpers.rs:430,672 | e5a07c2 |
| @for uses directive_colon_error for unterminated-string diagnostic | parser.rs:333 | ee3e7b4 |
| @define strip_suffix(':') documented as intentionally safe | parser.rs:382 | ee3e7b4 |
| @for unterminated-string regression test added | parser_tests.rs:1690 | 484913a |
| split() Vec::with_capacity(64) pre-allocation | builtins.rs:267 | 40e3135 |
| join() String capacity estimate from array length | builtins.rs:391 | 40e3135 |
| CondValue/Expr duplication documented with TODO and fix path | ast.rs:8 | ee3e7b4 |
| validate_for_node Call/QualifiedCall arms merged | validator.rs:134 | ee3e7b4 |
| Nested call AST structure assertion strengthened | parser_tests.rs:1206 | 8441aae |
| Sort order explicitly asserted in evaluate_for_sort_unique | parser_tests.rs:1487 | 8441aae |
| Error message content verified in 2 error-path tests | parser_tests.rs:1506,1515 | 8441aae |

## False Positives
(none)

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| Duplicated quote/paren scanner state machines (5+ functions) | parser_helpers.rs:27-661 | Re-deferred from cycle 1. 5+ correct, well-commented scanners. Unifying requires generic callback/closure indirection across all scanning functions — moderate risk of subtle behavioral divergence. Stable duplication acceptable at current grammar complexity. |
| evaluate_expr unconditional Value clone for Var lookups | evaluator.rs:147-150 | Architectural change: requires evaluate_expr_ref returning Cow<Value> or &Value, changing Scope::get_var contract and all evaluate_expr callers. The "double-clone" in evaluate_for is partially a borrow-release necessity. Real cost but proportional to data size; acceptable at project scale. |

## Blocked
(none)

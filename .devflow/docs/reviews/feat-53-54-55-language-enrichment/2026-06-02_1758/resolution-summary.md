# Resolution Summary

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02_1758
**Review**: .devflow/docs/reviews/feat-53-54-55-language-enrichment/2026-06-02_1758
**Command**: /resolve

## Decisions Citations

- applies ADR-008 — batch-1 (builtins:413:sort-nan), batch-2 (builtins:27:dual-registry), batch-4 (all fixes)

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 20 |
| Fixed | 18 |
| False Positive | 0 |
| Deferred | 2 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| `split("")` empty separator guard | builtins.rs:219 | 3d2b396 |
| `replace("")` empty search guard | builtins.rs:212 | 3d2b396 |
| `sort()` NaN → `total_cmp` + `is_finite()` validation | builtins.rs:413 | 3d2b396 |
| `unique()` O(n²) → HashSet with type-discriminated keys | builtins.rs:436 | 3d2b396 |
| `require_number_index` overflow/infinity guard | builtins.rs:295 | 3d2b396 |
| `expect()` panic → `ok_or_else` Result | evaluator.rs:305 | 3d2b396 |
| `validate_node` decompose → `validate_if_node` + `validate_for_node` | validator.rs:23 | 3d2b396 |
| `evaluate_condition` depth `debug_assert!` for And/Or | evaluator.rs:412 | 3d2b396 |
| Validator builtin arity tests (5 new tests) | validator.rs:370 | 3d2b396 |
| `builtin_error_at` constructor added | error.rs:365 | 5c05181 |
| Function-level `use` import → module level | parser_helpers.rs:935 | 5c05181 |
| Test name `parse_condition_or_higher_precedence_than_and` → `...and_has_higher_precedence_than_or` | parser_tests.rs:957 | 5c05181 |
| `@elseif` with `&&`/`||` integration tests added | parser_tests.rs:1060 | 5c05181 |
| `length()` byte count → char count (`s.chars().count()`) | builtins.rs:341 | b60218f |
| Dual-registry → unified `BuiltinDef` with handler fn pointer | builtins.rs:27 | b60218f |
| `slice()` byte-based → char-based indexing | builtins.rs:256 | b60218f |
| `builtin_sort` refactor: `require_homogeneous()`, clone-after-validate | builtins.rs:378 | b60218f |
| `get_builtin()` linear scan documented as intentional (18-element cache-resident) | builtins.rs:126 | b60218f |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| Arity-check logic duplicated across evaluator/validator (5 sites) | evaluator.rs:273 / validator.rs:183 | Cross-module refactor — resolution chains differ in return type (Result\<Value\> vs Result\<()>), error constructors (non-span vs span-aware), and downstream action. Shared helper wouldn't reduce call-site code materially. Deferred until feature set stabilizes. |
| Four quote-aware byte scanners with duplicated in_string tracking | parser_helpers.rs:124,205,840,890 | Each scanner has different match actions, return types, and byte-vs-char iteration strategies. Extracting a generic callback-based scanner touches core parsing primitives. Requires dedicated refactor task with comprehensive testing. |

## Blocked
(none)

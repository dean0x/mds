# Resolution Summary

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**Review**: .docs/reviews/feat-compiler/2026-05-14_1937
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 14 |
| Fixed | 14 |
| False Positive | 0 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| File read before cache check (split validate_and_read_file into canonicalize_and_check + read_validated_file) | src/resolver.rs:162 | 4add2a7 |
| shift_remove O(n) replaced with pop() O(1) for LIFO unmark | src/resolver.rs:191 | 4add2a7 |
| CollectedDefs tuple alias converted to named struct | src/resolver.rs:512 | 4add2a7 |
| ModuleCtx file_str doc comment added | src/resolver.rs:515 | 4add2a7 |
| Path traversal guard for mds.json output_dir (reject ParentDir components) | src/main.rs:136 | 4add2a7 |
| load_config file size guard (1 MB cap via metadata check) | src/main.rs:51 | 4add2a7 |
| Version references restored in value.rs ("in MDS v0.1") | src/value.rs:60,92 | 4add2a7 |
| debug_assert LIFO promoted to release-mode assert | src/evaluator.rs:191 | e7a7c6b |
| evaluate_for double-fault error preservation (render error priority) | src/evaluator.rs:287 | e7a7c6b |
| scope set_var expect documented (invariant explanation) | src/scope.rs:104 | e7a7c6b |
| Exit code 3 (resource limit) integration test | tests/integration.rs | 0300cfd |
| check_collecting_warnings direct integration tests (3 tests) | tests/integration.rs | 0300cfd |
| indexmap version tightened "2" → "2.2" | Cargo.toml:9 | 0300cfd |
| serde_yml pre-release tracking comment added | Cargo.toml:11 | 0300cfd |

## False Positives

(none)

## Deferred to Tech Debt

(none from the 14 resolved issues — see Remaining Review Items below)

## Blocked

(none)

## Remaining Review Items (Not In Scope)

The following items from the code review were not included in this resolution pass. They are either larger refactors, pre-existing issues, documentation-only items, or accepted design decisions:

### Complexity Refactors (recommend follow-up PR)
| Issue | File:Line | Reason |
|-------|-----------|--------|
| run() 150 lines → extract run_build/run_check/run_init | src/main.rs:405 | Significant structural change, better as dedicated refactor |
| collect_definitions_and_imports 93 lines → extract process_export | src/resolver.rs:272 | Substantial extraction, risk of subtle breakage |
| validate_and_read_file further decomposition | src/resolver.rs:71 | Partly addressed by cache-before-read fix |
| resolve_import repetitive patterns → shared helper | src/resolver.rs:367 | Moderate refactor across 3 match arms |
| Move load_config/resolve_output_path to lib crate | src/main.rs:33-157 | Architectural move, needs careful API design |
| process_module 6 params despite ModuleCtx | src/resolver.rs:229 | Nice-to-have, low impact |
| resolve_output_path side effects (create_dir_all) | src/main.rs:97 | SRP improvement, needs caller restructuring |
| error.rs constructor boilerplate (9 pairs) | src/error.rs:177-466 | Macro vs explicit trade-off, style choice |

### Documentation Items
| Issue | Notes |
|-------|-------|
| Default output changed from stdout to file | Needs CHANGELOG / release notes (not code change) |
| to_namespace() export visibility fix | Needs release notes documenting behavioral change |
| MdsError constructor docs (5 methods) | Low priority, internal constructors |

### Accepted / Informational
| Issue | Notes |
|-------|-------|
| Closure capture Arc→owned→Arc round-trip | Documented as intentional for cycle-breaking |
| MdsError derives Clone | Acceptable for pre-1.0 project |
| serde_yml 0.0.12 pre-stability | Tracked with comment in Cargo.toml |
| evaluate_for array clone via to_vec() | Necessary due to borrow conflict |

### Pre-existing (Not Introduced by This PR)
| Issue | File |
|-------|------|
| Symlink detection only covers final component | src/resolver.rs:75-110 |
| Lexer Vec<char> + Vec<usize> allocation | src/lexer.rs:46-51 |
| parser.rs 1024 lines | src/parser.rs |
| ResolvedModule public fields bypass accessor filtering | src/resolver.rs:36-41 |
| collect_all HashMap ordering relies on from_iter | src/scope.rs:174-177 |
| Hardcoded /tmp in exit_code_file_not_found test | tests/integration.rs:2435 |

# Resolution Summary

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19
**Review**: .docs/reviews/feat-14-error-serialization-dependency-graph/2026-05-19_0002
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 11 |
| Fixed | 6 |
| False Positive | 5 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Inconsistent entry-key exclusion — unified to value-based filtering | lib.rs:527-534 | f05e07d |
| NativeFs compile_with_deps integration test | api_surface.rs | dd4b470 |
| CompileOutput.warnings tested with real compiler warnings | api_surface.rs | dd4b470 |
| compile_str_with_deps with file imports test added | virtual_fs.rs | dd4b470 |
| Misleading DFS order comment corrected | virtual_fs.rs:328 | dd4b470 |
| error.rs test module split to error_tests.rs (1077→574 lines) | error.rs, error_tests.rs | ef492f4 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| Repetitive _at/no-span constructor pairs | error.rs:230-517 | Known Rust idiom, explicit and searchable. Macro would add metaprogramming complexity for minimal gain at current scale. Deliberate design, not oversight. |
| compile_*_with_deps duplicates _collecting_warnings | lib.rs:517-612 | 3-4 lines of setup code. Functions are 7 lines each, self-contained. Extracting helper adds more complexity than it removes. |
| CompileOutput defined in lib.rs | lib.rs:61-76 | 3 fields, closely tied to public API. Module-per-concern pattern applies to large concerns, not tiny structs. Reviewer acknowledged as defensible. |
| #[must_use] message style inconsistency | lib.rs:102,516 | Intentional progressive migration per commit 4d2f097. Two coexisting styles during migration is not a defect. |
| lib.rs public API surface growing wide | lib.rs:1-843 | Pre-existing trajectory. Reviewer noted "no change needed in this PR." Builder pattern is a future-iteration concern. |

## Deferred to Tech Debt
(none)

## Blocked
(none)

# Resolution Summary

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-18
**Review**: .docs/reviews/14-error-serialization-dependency-graph/2026-05-18_1755
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 14 |
| Fixed | 8 |
| False Positive | 3 |
| Deferred | 1 |
| Blocked | 0 |
| Pre-existing (informational) | 2 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| I1: compile_with_deps bypasses FileSystem::canonicalize() | lib.rs:521-524 | 4d2f097 |
| I2: NativeFs::canonicalize() follows symlinks without rejection | fs.rs:343-348 | 9965973 |
| I3: Missing serialize() tests for UndefinedFunction, ImportError, NameCollision | error.rs | 0f2bc4c |
| I4: Missing serialize() test for ExportError | error.rs | 0f2bc4c |
| I5: Missing edge case test: span=Some, src=None | error.rs | 0f2bc4c |
| I6: Missing edge case test: compute_line_column at offset==source.len() | error.rs | 0f2bc4c |
| I7: Missing # Examples doc sections on 3 _with_deps functions | lib.rs | 4d2f097 |
| I8: #[must_use] message references type name vs content | lib.rs | 4d2f097 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| I9: _with_deps naming diverges from _collecting_warnings | lib.rs | Intentional — returns CompileOutput struct (not tuple), signaling a new API pattern |
| I11: CompileOutput.dependencies uses Vec<String> | lib.rs:75 | By design for serialization simplicity; Vec is the correct public interface |
| I14: API surface combinatorial growth (19 functions) | lib.rs | Pre-existing pattern, not introduced by this PR |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| I10: resolve_source retains &Path param despite FileSystem abstraction | resolver.rs:238 | Breaking API change; out of scope for this PR. Safe post-1.0 with migration note. |

## Pre-existing (Informational)
| Issue | File | Note |
|-------|------|------|
| I12: error.rs is 983 lines | error.rs | File length threshold — not introduced by this PR |
| I13: lib.rs is 807 lines | lib.rs | File length threshold — not introduced by this PR |

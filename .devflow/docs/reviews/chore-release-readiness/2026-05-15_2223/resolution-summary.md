# Resolution Summary

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15
**Review**: .docs/reviews/chore-release-readiness/2026-05-15_2223
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 18 |
| Fixed | 18 |
| False Positive | 0 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| CHANGELOG escape syntax (`{{` → `\{`) | CHANGELOG.md:22 | 9d46272 |
| CHANGELOG Library API completeness (7→11 functions) | CHANGELOG.md:49 | 9d46272 |
| Resolver LIFO invariant check ordering (prefer_first_error pattern) | src/resolver.rs:212 | 04df39c |
| Resolver LIFO error message alignment (expected/got details) | src/resolver.rs:218 | 04df39c |
| ResolvedModule fields pub → pub(crate) | src/resolver.rs:36 | 04df39c |
| MAX_TRAVERSAL_DEPTH consolidated (pub(crate) + re-export) | src/resolver.rs:47, src/main.rs:29, src/lib.rs | 45cb4e5 |
| evaluate_for resource limit check before clone | src/evaluator.rs:285 | 4eb4539 |
| call_depth_limit test assertion tightened | src/evaluator.rs:606 | 4eb4539 |
| output_size_limit test split to reduce peak memory | src/evaluator.rs:618 | 4eb4539 |
| resolve_input consistency (run_build uses helper) | src/main.rs:440 | 8030327 |
| write_output extraction from run_build | src/main.rs:492 | 8030327 |
| Validator @if scope invariant documented | src/validator.rs:34 | d94e5e0 |
| Spec single-quote string literals documented | spec.md:135 | d94e5e0 |
| README --quiet moved to Global options | README.md:54 | d94e5e0 |
| Spec --quiet added to mds check docs | spec.md:394 | d94e5e0 |
| Warning cap test assertion tightened (≤1000 → ==1000) | tests/integration.rs:3186 | d94e5e0 |

## False Positives
(none)

## Deferred to Tech Debt
(none)

## Blocked
(none)

## Notes

- The evaluate_for `.to_vec()` clone cannot be eliminated due to Rust borrow rules (iterating borrowed items from scope while mutating scope). The fix moved the resource limit check BEFORE the clone to avoid wasting allocation on oversized arrays.
- MAX_TRAVERSAL_DEPTH was consolidated following the existing MAX_FILE_SIZE pattern: defined as `pub(crate)` in resolver.rs, re-exported as `pub const` from lib.rs.
- All 292 tests pass after all fixes.

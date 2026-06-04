# Resolution Summary

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16_1249
**Review**: .docs/reviews/feat-mds-three-enhancements/2026-05-16_1249
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 12 |
| Fixed | 12 |
| False Positive | 0 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| `strip_type_mds` silently corrupts nested YAML objects (HIGH) | src/lib.rs:342 | 7b4326f |
| `strip_type_mds` per-line format! allocation (MEDIUM) | src/lib.rs:346 | 7b4326f |
| Redundant Vec allocation on every MemberAccess eval (HIGH) | src/evaluator.rs:164,207 | be8fde3 |
| `debug_assert` + unchecked index in `resolve_dot_path` (MEDIUM) | src/evaluator.rs:100 | be8fde3 |
| `evaluate_for` near-duplicate loop bodies (MEDIUM) | src/evaluator.rs:338 | be8fde3 |
| `debug_assert` + unchecked index in validator If arm (MEDIUM) | src/validator.rs:25 | 72096c1 |
| `debug_assert` + unchecked index in validator For arm (MEDIUM) | src/validator.rs:49 | 72096c1 |
| Validator skips dot-path type check — documented as accepted limitation (MEDIUM) | src/validator.rs:60 | 72096c1 |
| Dead `_file`/`_source` params — restored syntax_at diagnostics (MEDIUM) | src/parser.rs:500 | 9364b24 |
| `parse_for_block` oversized — extracted `parse_for_vars` helper (MEDIUM) | src/parser.rs:249 | 9364b24 |
| Missing parser unit tests — added 7 tests (MEDIUM) | src/parser.rs | 9364b24 |
| Silent YAML non-string key drops — now returns error (MEDIUM) | src/value.rs:66 | 565f438 |

## False Positives
(none)

## Deferred to Tech Debt
(none)

## Blocked
(none)

## Verification
- 336 tests pass (up from 325 — 11 new tests added)
- Zero clippy warnings
- All doctests pass

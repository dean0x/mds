# Resolution Summary

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17_1434
**Review**: .docs/reviews/feat-workspace-split/2026-05-17_1434
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 14 |
| Fixed | 8 |
| False Positive | 3 |
| Deferred | 0 |
| Blocked | 0 |
| Deduplicated | 3 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Stale integration.rs reference in Key Files | `.features/mds-compiler/KNOWLEDGE.md:632` | 55c595d |
| Stale integration.rs reference in Related | `.features/mds-compiler/KNOWLEDGE.md:644` | 55c595d |
| Stale test path in "Adding a New Directive" | `.features/mds-compiler/KNOWLEDGE.md:419` | 55c595d |
| README install command incorrect | `README.md:10` | 55c595d |
| serde_yml pre-release comment misplaced | `Cargo.toml:17` | 55c595d |
| CLI load_vars_file shadows library function | `crates/mds-cli/src/main.rs:380` | 3e99343 |
| Unused import in objects.rs | `crates/mds-cli/tests/objects.rs:2` | 3e99343 |
| Inconsistent HashMap import in frontmatter.rs | `crates/mds-cli/tests/frontmatter.rs:50` | 08f9fa2 |
| Manual path construction in security.rs | `crates/mds-cli/tests/security.rs:131-134` | 08f9fa2 |
| Inconsistent qualified mds_bin in errors.rs | `crates/mds-cli/tests/errors.rs:199` | 08f9fa2 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| pub(crate) mod removes module paths | `crates/mds-core/src/lib.rs:40-49` | Intentional design decision (commit bd011ed). Zero external users. API surface test validates supported import pattern. Not a regression — deliberate visibility lockdown. |
| Repetitive API surface pattern in lib.rs | `crates/mds-core/src/lib.rs:84-337` | Deliberate ergonomic trade-off. Each function is < 15 lines. Pattern provides convenient wrappers for different input modes. Not blocking per reviewer's own assessment. |
| api_surface.rs tests verify existence not behavior | `crates/mds-core/tests/api_surface.rs:6-20` | Intentional design — stated purpose is compile-time visibility guard, not behavioral test. Pre-existing pattern, not introduced by this PR. |

## Deferred to Tech Debt
_(none)_

## Blocked
_(none)_

## Deduplicated
| Issue | Reasoning |
|-------|-----------|
| cons:knowledge:419 (stale references) | Same locations as doc:knowledge:632 and doc:knowledge:419 — fixed once |
| reg:cargo:1 (cargo install --path .) | Same issue as doc:readme:10 — fixed via README update |
| cons:objects:2 (unused import) | Already included in batch-2 objects.rs fixes |

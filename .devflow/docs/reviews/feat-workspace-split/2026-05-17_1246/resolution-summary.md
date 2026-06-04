# Resolution Summary

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17_1246
**Review**: .docs/reviews/feat-workspace-split/2026-05-17_1246
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 21 |
| Fixed | 5 |
| False Positive | 8 |
| Deferred | 2 |
| Duplicates (resolved by other fixes) | 6 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Missing [workspace.dependencies] — shared deps declared independently | Cargo.toml, crates/*/Cargo.toml | 137d3ea |
| Missing [workspace.package] — metadata duplicated across crates | Cargo.toml, crates/*/Cargo.toml | 137d3ea |
| KNOWLEDGE.md stale paths after workspace split | .features/mds-compiler/KNOWLEDGE.md | 3c69c5d |
| run_build 6-parameter function signature | crates/mds-cli/src/main.rs:483 | 81c182f |
| Weak OR assertion in not_mds_file_error test | crates/mds-cli/tests/integration.rs:181 | 6a90096 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| resolve_output_path complexity | main.rs:126-185 | 6 exit paths track 1:1 with documented 6-level precedence. Reviewer said "monitor". |
| load_config nesting depth | main.rs:36-83 | Idiomatic Rust directory walker. Flattening adds no simplification. |
| main.rs file length | main.rs | 156 of 779 lines are tests. Production code ~450 lines, under threshold. |
| run_check warning duplication | main.rs:540-561 | Branches differ in OK message. Helper would be equally complex. |
| parse_cli_value chains | main.rs:306-344 | Idiomatic Rust ordered type coercion. No meaningful refactor. |
| MdsConfig in CLI only | main.rs:14-23 | CLI-layer concern. Moving to core would expose config as public API with no consumer. |
| Fixture duplication (not_mds.md vs type_mds_md_file.md) | tests/fixtures/ | Opposite intents: rejection vs acceptance of .md files. Both needed. |
| Version field sync risk | Cargo.toml | Resolved by [workspace.package] — single source of truth. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| Monolithic 3,617-line integration test file | integration.rs | Splitting during structural PR adds scope and merge conflict risk. Own PR. |
| No cross-crate API surface test | — | New capability addition, not a fix. Inappropriate scope for structural split. |

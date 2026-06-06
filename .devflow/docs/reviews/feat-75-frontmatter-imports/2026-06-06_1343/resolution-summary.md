# Resolution Summary

**Branch**: feat/75-frontmatter-imports -> main
**Date**: 2026-06-06_1343
**Review**: .devflow/docs/reviews/feat-75-frontmatter-imports/2026-06-06_1343
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 13 |
| Fixed | 10 |
| False Positive | 2 |
| Deferred | 1 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Blank lines in imports: block leak into compiled output | lib.rs:405 | c0f89f4 |
| Stale test names strip_type_mds_* → strip_reserved_keys_* | lib.rs:948 | c0f89f4 |
| Error type mismatch: frontmatter collision used import_error instead of name_collision | resolver.rs:467,483 | 068c0d2 |
| Duplicate names in selective import silently accepted | resolver.rs:1106 | 068c0d2 |
| Non-string YAML keys in import entries silently ignored | resolver.rs:1068 | 068c0d2 |
| type:mds detection matched indented keys (pre-existing) | resolver.rs:912 | 068c0d2 |
| High cyclomatic complexity in parse_frontmatter_imports_from_yaml | resolver.rs:1022 | 068c0d2 |
| fm_import_for_expr test used built-in split() instead of imported alias | virtual_fs.rs:646 | ff6fef7 |
| Weak test assertions (output.contains('a')) | virtual_fs.rs:653 | ff6fef7 |
| Missing merge-import collision test | virtual_fs.rs (new) | ff6fef7 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| Missing .md exemption compilation test | virtual_fs.rs:900 | The is_mds=false branch in build_scope_from_frontmatter is unreachable from compile_virtual — validate_file_type rejects plain .md files with NotMdsFile before scope-build. The existing scan_imports test is the only reachable public API surface. |
| Missing positive --set imports test on plain .md | virtual_fs.rs | compile_virtual on a plain .md fails with NotMdsFile before reaching the runtime_vars guard. No code path exists to test this. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| Code duplication between frontmatter and body import resolution | resolver.rs:456 | The two paths use fundamentally different error attachment mechanisms (index-based vs span-based). Safe unification requires threading both context types through shared helpers, affecting the public error shape. Architectural trade-off, not a quick refactor. |

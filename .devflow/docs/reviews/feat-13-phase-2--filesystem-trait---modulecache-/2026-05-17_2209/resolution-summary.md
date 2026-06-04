# Resolution Summary

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17_2209
**Review**: .docs/reviews/feat-13-phase-2--filesystem-trait---modulecache-/2026-05-17_2209
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 15 |
| Fixed | 12 |
| False Positive | 2 |
| Deferred | 1 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| NativeFs::normalize missing empty-path guard | `crates/mds-core/src/fs.rs:274` | c00b9f1 |
| NativeFs::read TOCTOU metadata pre-check removed | `crates/mds-core/src/fs.rs:305` | 7e5fa7c |
| VirtualFs::normalize unbounded segment allocation | `crates/mds-core/src/fs.rs:130` | 7e5fa7c |
| Missing Debug derives on VirtualFs and NativeFs | `crates/mds-core/src/fs.rs:71,178` | 7e5fa7c |
| Error messages show filename-only instead of full path | `crates/mds-core/src/resolver.rs:157` | 7e5fa7c |
| resolve_source bypasses depth guard for root module | `crates/mds-core/src/resolver.rs:235` | 7e5fa7c |
| validate_file_type dotfile extension parsing regression | `crates/mds-core/src/resolver.rs:709` | 7e5fa7c |
| ModuleCache missing Debug implementation | `crates/mds-core/src/resolver.rs:46` | 7e5fa7c |
| Missing check_virtual API counterparts | `crates/mds-core/src/lib.rs:440` | c00b9f1 |
| selective_import test missing negative assertion | `crates/mds-core/tests/virtual_fs.rs:114` | c00b9f1 |
| compile_virtual_collecting_warnings untested | `crates/mds-core/tests/api_surface.rs` | c00b9f1 |
| resolve_key_directly fragile value extraction | `crates/mds-core/tests/virtual_fs.rs:243` | c00b9f1 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| VirtualFs::read clones entire content | `crates/mds-core/src/fs.rs:142` | Acceptable for v1. ModuleCache prevents re-reads. MAX_FILE_SIZE caps at 10MB. Multiple reviewers agreed. |
| compile_virtual takes HashMap by value | `crates/mds-core/src/lib.rs:440` | Intentional for v1. Single-shot compilation is the expected use case. Documented by PR description. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| resolve_source bypasses FileSystem abstraction (calls Path::canonicalize directly) | `crates/mds-core/src/resolver.rs:244` | Architectural — requires adding canonicalize() to FileSystem trait, coordinated with WASM/Phase 4. resolve_source documented as NativeFs-only. Not a correctness issue for current backends. |

## Blocked
(none)

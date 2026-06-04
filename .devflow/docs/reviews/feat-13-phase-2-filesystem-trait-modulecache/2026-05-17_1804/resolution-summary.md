# Resolution Summary

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17
**Review**: .docs/reviews/feat-13-phase-2-filesystem-trait-modulecache/2026-05-17_1804
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 26 |
| Fixed | 16 |
| False Positive | 5 |
| Pre-existing (skipped) | 5 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues

| Issue | File:Line | Commit |
|-------|-----------|--------|
| VirtualFs::read bypasses MAX_FILE_SIZE | fs.rs:114 | c2d7e24 |
| NativeFs::read allocates full file before size check | fs.rs:248 | c2d7e24 |
| NativeFs::normalize lacks null-byte validation | fs.rs:222 | c2d7e24 |
| FileSystem trait lacks security documentation | fs.rs:14 | c2d7e24 |
| is_markdown implementation divergence | fs.rs:121 | c2d7e24 |
| init_root computes even when OnceLock set | fs.rs:208 | c2d7e24 |
| process_module has 7 parameters | resolver.rs:256 | dfd8c98 |
| resolve_source lacks NativeFs-only documentation | resolver.rs:225 | dfd8c98 |
| process_module passes key as both file_str and base_key | resolver.rs:156 | dfd8c98 |
| validate_file_type 4-level nesting | resolver.rs:712 | dfd8c98 |
| compile_virtual breaks API delegation pattern | lib.rs:440 | 3a59960 |
| fs module visibility inconsistency (pub vs pub(crate)) | lib.rs:43 | 3a59960 |
| No VirtualFs cross-subdirectory import test | virtual_fs.rs | a2f56bc |
| No direct test for NativeFs::set_root | fs.rs:268 | a2f56bc |
| export_visibility test lacks negative assertion | virtual_fs.rs:129 | a2f56bc |
| Path traversal test has overly broad assertions | fs.rs:452 | a2f56bc |

## False Positives

| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| set_root breaks ISP | fs.rs:37 | Rust default methods are the idiomatic ISP solution. No-op default is correct and documented. |
| validate_file_type uses third extension algorithm | resolver.rs:702 | String-key context requires string-based rsplit, different from Path::extension contexts. Appropriate to type. |
| resolve_selective_import has 7 params | resolver.rs:440 | Shared params already bundled in ModuleCtx. Remaining params are variant-specific. |
| collect_export duplicates resolve_import_from | resolver.rs:341 | Arms diverge immediately after call. Extracting single-line shared prefix adds indirection without clarity. |
| Extension extraction dotfile edge case | resolver.rs:704 | Correct behavior — .mds dotfile correctly identified as mds extension. |

## Pre-existing (Not Addressed)

| Issue | File:Line | Note |
|-------|-----------|------|
| TOCTOU between normalize and read | fs.rs | Inherent to normalize-then-read pattern |
| resolve_key bypasses validate_import_path | resolver.rs | By design for entry-point keys |
| File read before size check (old code) | fs.rs | Now fixed by metadata pre-check |
| VirtualFs root entry accepts unsanitized keys | fs.rs | Closed HashMap key-space prevents actual access |
| Dynamic dispatch overhead | resolver.rs | Negligible for template compiler |

# Regression Review Report

**Branch**: refactor/consolidate-limits-split-parser -> main
**Date**: 2026-06-01

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 10
**Recommendation**: APPROVED

## Detailed Analysis

### Constants Migration (limits.rs consolidation)

All 5 constants were moved to `crates/mds-core/src/limits.rs` with identical values and appropriate visibility:

| Constant | Old Location | New Location | Value | Visibility |
|----------|-------------|-------------|-------|------------|
| `MAX_DOT_SEGMENTS` | `limits.rs` (pre-existing) | `limits.rs` | 32 | `pub(crate)` (unchanged) |
| `MAX_NESTING_DEPTH` | `parser.rs` | `limits.rs` | 64 | `pub(crate)` (unchanged) |
| `MAX_ELSEIF_BRANCHES` | `ast.rs` | `limits.rs` | 256 | `pub(crate)` (narrowed from `pub`, FP from cycle 1) |
| `MAX_FILE_SIZE` | `resolver.rs` | `limits.rs` | 10 * 1024 * 1024 | `pub(crate)` (unchanged) |
| `MAX_TRAVERSAL_DEPTH` | `resolver.rs` | `limits.rs` | 256 | `pub(crate)` (unchanged) |

All import paths across the crate (`evaluator.rs`, `fs.rs`, `parser.rs`, `parser_helpers.rs`, `validator.rs`) have been updated to `use crate::limits::*`. Zero stale references to `resolver::MAX_*`, `ast::MAX_*`, or `parser::MAX_NESTING_DEPTH` remain.

### Public API Preservation

The two publicly re-exported constants in `lib.rs` are preserved with identical types and values:
- `pub const MAX_FILE_SIZE: u64 = limits::MAX_FILE_SIZE;` (was `resolver::MAX_FILE_SIZE`)
- `pub const MAX_TRAVERSAL_DEPTH: usize = limits::MAX_TRAVERSAL_DEPTH;` (was `resolver::MAX_TRAVERSAL_DEPTH`)

No public exports were added or removed. External consumers of `mds::MAX_FILE_SIZE` and `mds::MAX_TRAVERSAL_DEPTH` are unaffected.

### Parser Split (parser.rs -> parser.rs + parser_helpers.rs + parser_tests.rs)

The split uses Rust's `#[path = "..."]` attribute to include `parser_helpers.rs` as `mod helpers` and `parser_tests.rs` as `mod tests` (cfg(test) only), both children of the `parser` module. This means:

- **All functions retain the same module path** (e.g., `parser::parse_condition` is still callable within the crate as before)
- `is_valid_identifier` is re-exported via `pub(crate) use helpers::is_valid_identifier;` -- maintaining cross-module access for any crate-internal caller
- Helper functions use `pub(super)` visibility, correctly scoping them to the parent `parser` module
- The `use helpers::*;` glob import in `parser.rs` brings all helpers into scope for `Parser` methods

### Signature Change: parse_export_directive

The `_offset` parameter was removed from `parse_export_directive(directive, offset)` -> `parse_export_directive(directive)`. This is safe because `ExportDirective` variants (`Named`, `ReExport`, `Wildcard`) do not carry an `offset` field in the AST, unlike `ImportDirective` variants which do. The parameter was genuinely unused. (applies ADR-001 -- the change correctly addresses the linked issue scope)

### Test Coverage

- All 591 existing tests pass (up from 590, +1 new pinning test in `limits.rs::tests::limits_have_expected_values`)
- Tests for all parser features (conditions, imports, exports, nesting, dot-paths, elseif branches, string escaping) are preserved in `parser_tests.rs`
- Pinning test ensures constant values cannot drift without a test failure

### Cross-Cycle Awareness

From cycle 1 PRIOR_RESOLUTIONS (18 issues, 5 fixed, 13 FP):
- **FP: `is_valid_identifier` unused `pub(crate)`** -- confirmed still FP; 21+ usages across parser module tree
- **FP: `MAX_ELSEIF_BRANCHES` visibility narrowing** -- confirmed still FP; `pub` in `ast.rs` was effectively `pub(crate)` (never re-exported from `lib.rs`)
- No previously fixed issues have been reverted

### Documentation Updates

- `CHANGELOG.md`: New "Internal" section under `[Unreleased]` correctly describes both changes
- `SECURITY.md`: Resource limits table updated to reference `limits.rs` instead of `parser.rs`/`resolver.rs`; new `MAX_ELSEIF_BRANCHES` row added (applies ADR-002 -- PR content matches claimed scope)

### Regression Checklist

- [x] No exports removed without deprecation
- [x] Return types backward compatible
- [x] Default values unchanged
- [x] Side effects preserved
- [x] All consumers of changed code updated
- [x] Migration complete across codebase (zero stale references)
- [x] Commit message matches implementation
- [x] Breaking changes documented in CHANGELOG (N/A -- no breaking changes)
- [x] All 591 tests pass
- [x] cargo fmt and clippy clean (zero warnings)

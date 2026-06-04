# Security Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

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

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

This PR is a workspace restructuring with no substantive security-relevant code changes. The diff consists of:

1. **Cargo workspace split** — The single-crate project is split into `mds-core` (library) and `mds-cli` (binary). No new dependencies are introduced; all dependencies are the same versions moved into workspace declarations.

2. **Module visibility tightening** — `pub mod error` and `pub mod value` are narrowed to `pub(crate) mod`, which is a security *improvement*: it reduces the public API surface, preventing external crates from depending on internal module paths.

3. **Return type simplification** — Function signatures changed from `Result<T, miette::Error>` to `Result<T>` (using a `miette::Result` type alias). This is purely cosmetic with no behavioral change.

4. **Import consolidation** — `MAX_FILE_SIZE as MAX_STDIN_SIZE` and `MAX_TRAVERSAL_DEPTH` are now imported in a single `use` statement instead of separate declarations. The actual values and their usage are unchanged.

5. **`run_build` parameter refactor** — Arguments are bundled into a `BuildArgs` struct. No behavioral change.

6. **Security test suite preserved** — All security tests (path traversal, symlink rejection, file size limits, import depth, config size, loop iteration limits) are preserved and moved to `crates/mds-cli/tests/security.rs`. The test coverage is equivalent.

### Security Controls Verified As Intact

- Symlink rejection in imports (`check_symlink`)
- Path traversal prevention (`check_path_traversal`, `find_project_root`)
- MAX_FILE_SIZE = 10MB with TOCTOU-safe read-first-then-check pattern
- MAX_IMPORT_DEPTH = 64
- MAX_TRAVERSAL_DEPTH = 256 for directory walks
- MAX_CONFIG_SIZE = 1MB for mds.json
- `validate_import_path` rejects non-relative paths and null bytes
- CLI rejects directory input and `..` in init filenames
- Stdin size limit enforced via `take(MAX_STDIN_SIZE + 1)` pattern
- `output_dir` path traversal rejection in `resolve_output_path`
- Loop iteration limits (per-loop and total)
- Parser nesting depth limit

No security guards were weakened, removed, or bypassed by this refactoring.

# Regression Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Stale comment references removed function name `strip_type_mds`** - `crates/mds-core/src/resolver.rs:899`
**Confidence**: 90%
- Problem: The comment on line 899 reads `"mirroring the strip_type_mds helper elsewhere"` but `strip_type_mds` was renamed to `strip_reserved_keys` in this PR. This stale reference will confuse future developers searching for the function by name.
- Fix: Update the comment to reference `strip_reserved_keys`:
  ```rust
  // mirroring the strip_reserved_keys helper elsewhere.
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`strip_reserved_keys` trims trailing whitespace differently from `strip_type_mds`** - `crates/mds-core/src/lib.rs:433-437` (Confidence: 65%) -- The old `strip_type_mds` returned `Some(filtered)` preserving any trailing blank lines in the filtered output. The new `strip_reserved_keys` applies `filtered.trim()` then re-appends a single `\n`, which normalizes trailing whitespace. This is likely intentional (cleaner output), but any downstream consumer that depended on trailing blank lines in frontmatter would see a formatting change. All existing tests pass, so this is low-risk.

- **`scan_imports` public API behavior change is additive but undocumented in CHANGELOG** - `crates/mds-core/src/lib.rs:794-833` (Confidence: 70%) -- `scan_imports` now returns paths from frontmatter `imports:` blocks in addition to body `@import` directives. This is an intentional enhancement per the PR description, but callers that assumed `scan_imports` only returned body-level paths may receive unexpected results. The function's doc comment was updated, and the change is additive (no paths removed), so regression risk is low. Consider noting this in the CHANGELOG.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Analysis Summary

This PR introduces frontmatter imports, a well-structured feature addition to the MDS compiler. From a regression standpoint:

**Return type change (`build_scope_from_frontmatter`)**: Changed from `Result<Scope>` to `Result<(Scope, Vec<FrontmatterImport>)>`. The single call site in `process_module` was correctly updated. No external consumers exist (function is private). No regression.

**Function rename (`strip_type_mds` to `strip_reserved_keys`)**: All call sites updated. Both the function and test invocations now use the new name. One stale comment in `resolver.rs:899` still references the old name (the sole actionable finding).

**Removed function (`parse_frontmatter`)**: Inlined into `build_scope_from_frontmatter`. No other callers existed (it was a private function). No regression.

**Namespace collision check added (`resolve_alias_import`)**: New guard clause at line 444 prevents duplicate namespace aliases. This is additive safety; existing valid imports are unaffected.

**New types and functions (`FrontmatterImport`, `parse_frontmatter_imports*`, `resolve_frontmatter_imports`)**: All `pub(crate)` visibility. No public API surface change. Comprehensive test coverage with 20+ new tests covering happy paths, error cases, edge cases, and integration scenarios (applies ADR-002 -- semantic verification of feature completeness).

**Test coverage**: 820 tests pass (up from 590+ on main). New integration tests in `virtual_fs.rs` cover alias, merge, selective, chaining, collision detection, circular import detection, dependency tracking, and coexistence with body imports.

**Defense-in-depth**: `MAX_FRONTMATTER_IMPORTS` limit (256) added in `limits.rs` prevents adversarial inputs from triggering unbounded file resolutions. The `--set imports=...` path is blocked for MDS files, preventing runtime override of the reserved key.

### Conditions for Approval

1. Update stale comment in `resolver.rs:899` referencing the old function name.

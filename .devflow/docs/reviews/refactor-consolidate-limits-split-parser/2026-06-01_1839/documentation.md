# Documentation Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01

## Issues in Your Changes (BLOCKING)

### MEDIUM

**SECURITY.md resource limits table incomplete after constant consolidation** - `SECURITY.md:52-64`
**Confidence**: 85%
- Problem: The SECURITY.md resource limits table was updated to reflect the new location of `MAX_FILE_SIZE` and `MAX_NESTING_DEPTH` (now in `limits.rs`), but `MAX_ELSEIF_BRANCHES` (256) was moved from `ast.rs` to `limits.rs` and is not listed in the table at all. Since the PR consolidates all cross-module constants into `limits.rs` as the single source of truth, this limit should be documented alongside the others. `MAX_ELSEIF_BRANCHES` is a security-relevant resource limit that guards against adversarial input with excessive `@elseif` branches.
- Fix: Add a row to the SECURITY.md resource limits table:
```markdown
| Max @elseif branches per @if | 256 | `limits.rs` (`MAX_ELSEIF_BRANCHES`) |
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**New `parser_helpers.rs` module lacks a module-level doc comment** - `crates/mds-core/src/parser_helpers.rs:1`
**Confidence**: 82%
- Problem: The new `parser_helpers.rs` file (733 lines) contains 20+ public helper functions extracted from `parser.rs` but has no module-level documentation (`//!` comment) explaining the module's purpose and relationship to `parser.rs`. Since this is a non-trivial extraction (the file is loaded via `#[path = "parser_helpers.rs"] mod helpers;` in `parser.rs`), a brief module doc would help future maintainers understand the module boundary and the reason these functions live in a separate file rather than inline.
- Fix: Add a module-level doc comment at the top of `parser_helpers.rs`:
```rust
//! Helper functions extracted from the parser.
//!
//! These free functions handle parsing of individual constructs (conditions,
//! arguments, import/export directives, interpolation expressions) and are
//! used by the `Parser` methods in `parser.rs`. Extracted to keep the main
//! parser module focused on the recursive-descent structure.
```

## Pre-existing Issues (Not Blocking)

### LOW

**SECURITY.md resource limits table has inconsistent location granularity** - `SECURITY.md:52-64`
**Confidence**: 65%
- Some entries reference bare module names (`evaluator.rs`, `resolver.rs`, `value.rs`) while others now reference `limits.rs`. After this PR, `MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`, and `MAX_WARNINGS` still live in `evaluator.rs` and `MAX_IMPORT_DEPTH` still lives in `resolver.rs`. The table is factually accurate, but the mix of locations could be confusing since the PR's stated goal is consolidation into `limits.rs` as a single source of truth -- yet only 5 of the 10+ constants were actually moved.

## Suggestions (Lower Confidence)

- **CHANGELOG [Unreleased] section is empty** - `CHANGELOG.md:8-9` (Confidence: 70%) -- The PR makes a significant internal refactoring (parser split from 1820 to ~423 lines, constant consolidation into `limits.rs`). While the PR states "no behavioral changes" and this is purely internal, the [Unreleased] section has no entry. For a pre-1.0 project, internal refactoring entries under a `### Changed` or `### Internal` heading can be valuable for contributors tracking code organization changes.

- **`parser_tests.rs` lacks a module-level doc comment** - `crates/mds-core/src/parser_tests.rs:1` (Confidence: 62%) -- The new 668-line test file has no `//!` doc comment. A one-liner like `//! Parser unit and integration tests, extracted from parser.rs.` would clarify provenance.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 1 |

**Documentation Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The documentation updates in SECURITY.md correctly reflect the relocation of `MAX_FILE_SIZE` and `MAX_NESTING_DEPTH` to `limits.rs`. The new `limits.rs` file has excellent inline documentation with doc comments on every constant. The extracted `parser_helpers.rs` functions retain their original doc comments. The two conditions for full approval are: (1) add `MAX_ELSEIF_BRANCHES` to the SECURITY.md resource limits table since it was consolidated into `limits.rs` alongside the other documented limits, and (2) add a module-level doc comment to `parser_helpers.rs` to explain the extraction rationale. Applies ADR-002 -- the SECURITY.md location updates directly address the documentation alignment concern raised by the constant consolidation.

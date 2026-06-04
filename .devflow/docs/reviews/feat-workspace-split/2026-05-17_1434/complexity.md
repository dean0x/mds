# Complexity Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Repetitive API surface pattern in lib.rs (6 function pairs)** - `crates/mds-core/src/lib.rs:84-337`
**Confidence**: 82%
- Problem: The library exposes 10 public functions that follow two near-identical patterns: (1) `compile/check` variants that call their `_collecting_warnings` counterpart and emit warnings, and (2) `_collecting_warnings` variants that duplicate the `cache.resolve()` + body assembly logic. The `compile_collecting_warnings` and `compile_str_collecting_warnings` bodies are almost identical (lines 241-257 vs 264-281), differing only in whether they call `cache.resolve(path, ...)` or `cache.resolve_source(source, dir, ...)`. Same for `check_collecting_warnings` vs `check_str_collecting_warnings` (lines 297-307 vs 326-337).
- Fix: This is a maintainability smell rather than a blocking issue. The pattern is deliberate (ergonomic convenience wrappers) and each function is individually simple (< 15 lines). However, a single internal `compile_inner` helper accepting an enum `Source { File(path), Str(source, dir) }` could reduce the duplication from 6 function bodies to 2 + 4 thin wrappers. This is a suggestion for future refactoring, not a merge blocker given the low per-function complexity.

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

_No issues found._

## Suggestions (Lower Confidence)

- **`resolve_output_path` has 4 parameters (all `&Option<T>`)** - `crates/mds-cli/src/main.rs:126` (Confidence: 65%) -- The function takes 4 reference-to-Option parameters. A `ResolveContext` struct could improve readability, but this was already present on main and the `BuildArgs` struct introduction for `run_build` shows the right direction is understood.

- **`lib.rs` file length approaching warning threshold** - `crates/mds-core/src/lib.rs` (Confidence: 62%) -- At 576 lines (including tests and doc comments), the file is within acceptable range but growing. The bulk is documentation and unit tests, so effective code density is good. Worth monitoring as features are added.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED

## Rationale

This PR is a significant complexity *reduction*. Key observations:

1. **Monolithic integration.rs (3617 lines) eliminated** -- Split into 10 focused test modules averaging 370 lines each. This is a major maintainability improvement. The largest (cli_build.rs at 753 lines) consists entirely of independent test functions with no shared mutable state or complex control flow.

2. **CLI refactored with `BuildArgs` struct** -- The old `run_build` took 6 positional parameters; the new version uses a named struct, improving readability without adding indirection.

3. **Individual function complexity is low** -- The most complex function (`resolve_output_path`) has cyclomatic complexity ~7 (6 exit points in a clear priority chain), well-structured with early returns. Max nesting depth is 3 (the `if let Some` chain for mds.json config). All functions are < 60 lines.

4. **Workspace split adds structural clarity** -- Separating library from CLI binary is the canonical Rust pattern. No new indirection or abstraction layers were introduced.

5. **All loops are bounded** -- `load_config` iterates with `for _ in 0..MAX_TRAVERSAL_DEPTH`, and `auto_detect_mds_file` collects from a single directory read.

The single MEDIUM finding (repetitive API surface) is a conscious design trade-off for ergonomics and does not impair comprehension of any individual function.

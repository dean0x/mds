# Complexity Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23
**Prior Resolutions**: Cycle 1 resolved 6/6 issues (0 FP, 0 deferred)

## Issues in Your Changes (BLOCKING)

No blocking complexity issues found.

## Issues in Code You Touched (Should Fix)

No should-fix complexity issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing complexity issues at CRITICAL severity in changed files.

## Suggestions (Lower Confidence)

- **Repetitive path-conversion boilerplate in public API functions** - `lib.rs:180,302,347,553` (Confidence: 65%) -- Four public functions (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`) each follow an identical `let path_str = path_to_str(path)?; ... cache.resolve_path(path_str, ...)` pattern. This is not a complexity *problem* today (each function is short and clear), but if more path-based entry points are added the duplication could justify a shared helper that wraps the resolve_path call behind a Path-accepting facade. Low urgency given the functions are each under 15 lines.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Complexity Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This PR is a net complexity *reduction*. The key changes and their complexity impact:

**LazyInit<T> (new, 48 lines)**: Clean single-responsibility class with 4 private fields and 3 methods. `get()` has a cyclomatic complexity of ~4 (two guard clauses, one generation check in the `.then` success path, one in the rejection path). Nesting depth is 2. The generation-counter TOCTOU defense is well-documented and straightforward. This class replaces ad-hoc init/pending/flag patterns that were duplicated across `transform.ts` and `webpack-loader/src/index.ts`, consolidating the logic into one tested location.

**webpack-loader/src/index.ts**: The old `ensureTransformer` function (19 lines, cyclomatic complexity ~4, with an invariant-violation throw after await) is replaced by a 4-line `getLazy` function (complexity 1) plus `lazy.get()`. The module shrank from ~82 lines to ~81 lines while removing the tricky post-await null check (`if (transformer === null) throw ...`). This is a clear readability and maintainability win.

**transform.ts**: The inline `ensureInit` closure (9 lines, 3 state variables) is replaced by a single `LazyInit<void>` instantiation. Reduces function-level cognitive load.

**resolve_base_dir (lib.rs:212-226)**: The `None` arm now chains `.and_then(|p| p.to_str()...)` adding one nesting level (depth 3 inside the match arm). This is standard Rust combinator style and stays well within acceptable limits.

**path_to_str (lib.rs:259-262)**: A 3-line helper with complexity 1. Trivial.

**Resolver signature changes (resolver.rs)**: `resolve_path` and `resolve_source` now take `&str` instead of `&Path`. This *removes* the `.display().to_string()` lossy conversion that was inside the methods, pushing the conversion to the caller boundary where it can be validated. The resolver methods themselves became simpler (one fewer intermediate variable each).

**Tests**: 155 lines of new test code in `lazy-init.spec.mjs` and ~65 lines in `api_surface.rs`. All tests are simple, single-assertion, behavior-focused. No complexity concerns.

No function exceeds 30 lines. No nesting exceeds depth 3. No cyclomatic complexity exceeds 5. No parameter lists exceed 4. All files remain well under 300 lines. The overall direction of this PR is toward lower complexity through consolidation of duplicated concurrency patterns.

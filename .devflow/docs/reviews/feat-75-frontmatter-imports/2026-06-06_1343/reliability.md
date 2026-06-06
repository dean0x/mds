# Reliability Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06

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

- **Duplicate names in selective import `names` array silently overwrite** - `resolver.rs:1106-1118` (Confidence: 65%) -- `parse_frontmatter_imports_from_yaml` does not check for duplicate entries within a single `names` list (e.g., `names: [greet, greet]`). The duplicate would silently overwrite the function in scope. This matches the body `@import { greet, greet } from` behavior, so it is consistent, but a defense-in-depth duplicate check could prevent confusing user-facing behavior. Not blocking since the body import path has the same behavior.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

The frontmatter imports feature demonstrates strong reliability practices:

1. **Bounded iteration** -- `MAX_FRONTMATTER_IMPORTS` (256) caps the number of frontmatter import entries. This feeds into the existing `MAX_IMPORT_DEPTH` (64) guard via the `resolve_import_from` -> `resolve_by_key` -> `check_import_depth()` chain. Both bounds are enforced, preventing unbounded recursion or resolution.

2. **Cycle detection** -- Frontmatter imports resolve through `resolve_import_from` which delegates to `resolve_by_key`, reusing the existing `resolving` IndexSet for cycle detection. The test `fm_import_circular` confirms A->B->A cycles are caught. The LIFO invariant assertion (`check_lifo_pop`) is maintained.

3. **Error context at every boundary** -- `attach_frontmatter_index` enriches error messages with `(in frontmatter imports[N])` context for spanless errors (FileNotFound, CircularImport, ImportError). Errors that already carry spans from the imported file are passed through unchanged, which is correct.

4. **Name collision guards** -- All three frontmatter import forms (alias, merge, selective) check for name collisions before inserting into scope. The alias form checks `scope.get_namespace()`, the merge form checks `scope.get_function()` per export, and these match their body-import counterparts.

5. **Reserved key protection** -- `--set imports=...` is blocked for MDS files via the runtime_vars guard, preventing users from injecting arbitrary import declarations via the CLI. Plain `.md` files without `type: mds` correctly treat `imports` as a regular variable.

6. **Resource limits** -- The `MAX_FRONTMATTER_IMPORTS` constant (256) is well-calibrated alongside the existing limits (`MAX_IMPORT_DEPTH` = 64, `MAX_NESTING_DEPTH` = 64, `MAX_ELSEIF_BRANCHES` = 256). It prevents adversarial YAML from triggering unbounded file resolution while remaining generous for real templates.

7. **No unbounded allocation** -- `Vec::with_capacity(seq.len())` pre-sizes the result vector based on the (already-bounded) input length.

8. **Consistent with existing patterns** -- The frontmatter import resolution mirrors the body import resolution (alias/merge/selective), sharing the same `resolve_import_from` path. This means all existing reliability guarantees (depth, cycles, file-not-found) apply uniformly.

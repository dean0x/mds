# Complexity Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06T13:43

## Issues in Your Changes (BLOCKING)

### HIGH

**`parse_frontmatter_imports_from_yaml` is 108 lines with high cyclomatic complexity** - `resolver.rs:1022-1129`
**Confidence**: 85%
- Problem: This function has ~108 lines with 4 nesting levels, multiple match arms, nested loops, and a high number of early-return error paths. It handles type extraction, path validation, unknown-key scanning, and the `(as, names)` dispatch all in a single function body. Cyclomatic complexity is approximately 15-18 (multiple `if`, `match`, `for` loops with conditional returns). This exceeds the "Warning" threshold (10) and approaches "Critical" (>10) per complexity metrics.
- Fix: Extract the per-entry parsing into a dedicated `parse_single_import_entry` helper. This would reduce `parse_frontmatter_imports_from_yaml` to the sequence-level validation + loop, and move the per-entry logic (path extraction, key validation, as/names dispatch) into a focused ~60-line function:

```rust
fn parse_single_import_entry(
    map: &serde_yaml_ng::Mapping,
    index: usize,
) -> Result<FrontmatterImport, MdsError> {
    let err = |msg: &str| MdsError::import_error(
        format!("imports[{index}]: {msg} (in frontmatter)")
    );
    // ... path extraction, key validation, as/names dispatch ...
}

pub(crate) fn parse_frontmatter_imports_from_yaml(
    imports_val: &serde_yaml_ng::Value,
) -> Result<Vec<FrontmatterImport>, MdsError> {
    let serde_yaml_ng::Value::Sequence(seq) = imports_val else { ... };
    if seq.len() > MAX_FRONTMATTER_IMPORTS { ... }
    let mut result = Vec::with_capacity(seq.len());
    for (index, entry) in seq.iter().enumerate() {
        let serde_yaml_ng::Value::Mapping(map) = entry else { ... };
        result.push(parse_single_import_entry(map, index)?);
    }
    Ok(result)
}
```

### MEDIUM

**`resolve_frontmatter_imports` duplicates logic from existing body-import resolvers** - `resolver.rs:456-524`
**Confidence**: 82%
- Problem: The `Merge` arm (lines 477-492) is nearly identical to `resolve_merge_import` (lines 526-549) — both iterate `get_all_exports()`, check for name collisions, call `set_function`, and handle `get_prompt_value`. The `Selective` arm (lines 494-519) duplicates the prompt-vs-function dispatch from `resolve_selective_import` (lines 551-593). The difference is only in error formatting (frontmatter index vs source span). This is 3 instances of the same resolve-and-populate pattern with minor error-context variations.
- Fix: Extract a shared helper that takes a generic error-wrapper closure. For example:

```rust
fn apply_merge_to_scope(
    resolved: &ResolvedModule,
    scope: &mut Scope,
    collision_err: impl Fn(String) -> MdsError,
) -> Result<(), MdsError> {
    for (name, func) in resolved.get_all_exports() {
        if scope.get_function(&name).is_some() {
            return Err(collision_err(name));
        }
        scope.set_function(&name, func);
    }
    if let Some(val) = resolved.get_prompt_value() {
        scope.set_var("prompt", val);
    }
    Ok(())
}
```

Both `resolve_merge_import` and the `Merge` arm of `resolve_frontmatter_imports` would call this with different error constructors. Similar extraction for the selective import logic.

**`build_scope_from_frontmatter` grew to 67 lines with mixed concerns** - `resolver.rs:757-823`
**Confidence**: 80%
- Problem: This function now handles four concerns: (1) `is_mds` detection, (2) YAML parsing, (3) key-by-key scope population with special-case routing for `type` and `imports`, and (4) runtime vars override with the `imports` reservation guard. At 67 lines and 4 nesting levels (function > if-let > if-let Mapping > for + conditionals), it is at the upper boundary of the "Warning" zone (30-50 lines). The `is_mds` determination logic (lines 768-776) is a separate concern from scope population.
- Fix: Extract the `is_mds` detection into its own function (it already has `has_type_mds_frontmatter_raw` but the `.mds` vs `.md` routing is inline). This would reduce `build_scope_from_frontmatter` by ~10 lines and clarify intent:

```rust
fn is_mds_file(is_md: bool, frontmatter: Option<&Frontmatter>) -> bool {
    if !is_md { return true; }
    frontmatter.is_some_and(|fm| has_type_mds_frontmatter_raw(&fm.raw))
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`resolver.rs` file length grew from 345 to 1334 lines** - `resolver.rs`
**Confidence**: 85%
- Problem: The file was already at a moderate size and has nearly quadrupled with this PR (345 -> 1334 lines, including ~200 lines of tests). While the new code is well-organized into logical sections, the file now exceeds the 500-line "Critical" threshold by a wide margin. The file contains the `ModuleCache` impl, all import resolution logic, frontmatter parsing, scope building, validation helpers, the `FrontmatterImport` type and parser, and tests.
- Fix: Consider splitting into submodules in a future PR: `resolver/frontmatter.rs` (the `FrontmatterImport` enum, `parse_frontmatter_imports_from_yaml`, `parse_frontmatter_imports`, `has_type_mds_frontmatter_raw`, `has_type_mds_frontmatter`, `build_scope_from_frontmatter`) and `resolver/cache.rs` (the `ModuleCache` impl). This is consistent with the project's existing pattern of splitting large modules (applies ADR-008 — related features in the same layer are batched, but the result should still be maintainably structured).

## Suggestions (Lower Confidence)

- **Duplicated `type: mds` detection logic across three locations** - `resolver.rs:889-904`, `resolver.rs:910-917`, `lib.rs:417-420` (Confidence: 70%) — The pattern `v == "mds" || v == "\"mds\"" || v == "'mds'"` appears in `has_type_mds_frontmatter`, `has_type_mds_frontmatter_raw`, and `strip_reserved_keys`. Consider extracting a `is_mds_value(v: &str) -> bool` helper to centralize the YAML quoting logic.

- **`attach_frontmatter_index` match arms could grow unbounded** - `resolver.rs:972-993` (Confidence: 65%) — The function pattern-matches on specific `MdsError` variants. If new error variants are added, this function must be updated in lockstep. A trait method or generic "attach context" pattern on `MdsError` itself would be more maintainable.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Complexity Score**: 6/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new code is logically sound and well-tested (488 new test lines covering all three import forms, error paths, collisions, and edge cases). The primary complexity concern is `parse_frontmatter_imports_from_yaml` at 108 lines with high cyclomatic complexity — extracting per-entry parsing into a helper would bring this under control. The duplication between frontmatter and body import resolution is a secondary concern worth addressing to prevent drift. The file length growth is notable but acceptable if a module split is planned.

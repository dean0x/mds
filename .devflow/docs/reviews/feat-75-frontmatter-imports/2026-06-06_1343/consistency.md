# Consistency Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent error type for name collision in frontmatter alias imports vs body alias imports** - `crates/mds-core/src/resolver.rs:467`
**Confidence**: 90%
- Problem: Body alias imports use `MdsError::name_collision(alias.to_string())` (line 445), which is the dedicated error constructor for namespace collisions. Frontmatter alias imports at line 467 use `MdsError::import_error(format!("name collision: '{alias}' is already defined ..."))` instead -- a different error variant with a hand-crafted message. The same inconsistency exists for frontmatter merge imports at line 483 vs body merge imports at line 541. This means error handling code that matches on `MdsError::NameCollision` will not catch frontmatter collisions.
- Fix: Use `MdsError::name_collision(alias.to_string())` in the frontmatter alias branch, and `MdsError::name_collision(name)` in the frontmatter merge branch, with the `"(in frontmatter imports[{i}])"` context appended via the same pattern used by `attach_frontmatter_index`. Alternatively, create a `name_collision_in_frontmatter` constructor that wraps the context. The key is to use the same error variant for the same semantic condition.

```rust
// Current (inconsistent):
FrontmatterImport::Alias { path, alias } => {
    if scope.get_namespace(alias).is_some() {
        return Err(MdsError::import_error(format!(
            "name collision: '{alias}' is already defined \
             (in frontmatter imports[{i}])"
        )));
    }

// Consistent with body imports (line 445):
FrontmatterImport::Alias { path, alias } => {
    if scope.get_namespace(alias).is_some() {
        return Err(MdsError::name_collision(alias.to_string()));
        // Or if frontmatter context is needed, wrap consistently
    }
```

### MEDIUM

**Inconsistent `type: mds` detection between `has_type_mds_frontmatter_raw` and `strip_reserved_keys`** - `crates/mds-core/src/resolver.rs:910-916`
**Confidence**: 82%
- Problem: The new `has_type_mds_frontmatter_raw` function (line 910) uses `line.trim().strip_prefix("type:")`, matching `type: mds` at any indentation level. Meanwhile, `strip_reserved_keys` in `lib.rs:417` only strips top-level (non-indented) `type: mds` lines, which is correct per the existing code comment that was removed. Both the existing `has_type_mds_frontmatter` (line 889, also uses `line.trim()`) and the new `_raw` variant will detect an indented `type: mds` inside a nested YAML mapping (e.g., `config:\n  type: mds`) and incorrectly classify the file as MDS. This means a `.md` file with an indented `type: mds` could have its `imports` key parsed as structured imports rather than as a plain variable, when `strip_reserved_keys` would correctly leave that indented `type: mds` alone in the output.
- Fix: Change `has_type_mds_frontmatter_raw` to only match top-level lines (no leading whitespace), consistent with `strip_reserved_keys`. Note: This is partially pre-existing (the full-source variant `has_type_mds_frontmatter` has the same issue), but `has_type_mds_frontmatter_raw` is new code and directly uses that pre-existing inconsistent behavior to drive the new frontmatter imports logic, making it newly consequential.

```rust
// Current (matches indented lines):
fn has_type_mds_frontmatter_raw(raw: &str) -> bool {
    raw.lines().any(|line| {
        line.trim().strip_prefix("type:").is_some_and(|v| {
            ...
        })
    })
}

// Consistent with strip_reserved_keys:
fn has_type_mds_frontmatter_raw(raw: &str) -> bool {
    raw.lines().any(|line| {
        line.strip_prefix("type:").is_some_and(|v| {
            let v = v.trim();
            v == "mds" || v == "\"mds\"" || v == "'mds'"
        })
    })
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Test function names reference old `strip_type_mds` name while testing `strip_reserved_keys`** - `crates/mds-core/src/lib.rs:948-1012`
**Confidence**: 95%
- Problem: Six test functions still use the old naming `strip_type_mds_*` (e.g. `strip_type_mds_plain_value`, `strip_type_mds_double_quoted`, etc.) while the function under test was renamed to `strip_reserved_keys`. The section comment at line 945 was updated to reference the new name, but the function names themselves were not. This is a naming inconsistency within the same test module where new tests (line 1017 onwards: `strip_imports_block`, `strip_imports_flow_style`, etc.) correctly use the new naming convention.
- Fix: Rename the six test functions to match the new function name (e.g., `strip_reserved_keys_plain_value`, `strip_reserved_keys_double_quoted`, etc.).

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`has_type_mds_frontmatter` uses `line.trim()` for detection, inconsistent with `strip_reserved_keys` top-level-only approach** - `crates/mds-core/src/resolver.rs:889-904`
**Confidence**: 82%
- Problem: Pre-existing inconsistency between the detection function (`line.trim()`, matches any depth) and the stripping function (top-level only). This existed before this PR but gains significance now that the detection function drives whether `imports` is parsed as structured imports vs a variable.

## Suggestions (Lower Confidence)

- **Duplicated `type: mds` matching logic across three locations** - `crates/mds-core/src/resolver.rs:896`, `crates/mds-core/src/resolver.rs:912`, `crates/mds-core/src/lib.rs:417` (Confidence: 70%) -- The expression `v == "mds" || v == "\"mds\"" || v == "'mds'"` appears in three places. Consider extracting a shared `is_mds_value(v: &str) -> bool` helper to ensure all three stay in sync.

- **Frontmatter merge import collision check omits `scope.get_namespace` guard** - `crates/mds-core/src/resolver.rs:482` (Confidence: 65%) -- Body merge imports only check `scope.get_function`. Frontmatter merge imports also only check `scope.get_function`. This is consistent between the two, but neither checks for a namespace collision (an alias import could have created a namespace with the same name as a function being merged). This is likely a pre-existing design choice rather than a bug, but worth noting since the PR description mentions "namespace collision detection added."

- **`strip_reserved_keys` output now trims leading/trailing whitespace from frontmatter** - `crates/mds-core/src/lib.rs:433-437` (Confidence: 65%) -- The old `strip_type_mds` returned `filtered` with its trailing newlines preserved. The new version applies `filtered.trim()` before re-adding a single trailing newline. This subtly changes output formatting for frontmatter that had leading/trailing blank lines. Probably intentional cleanup, but it is a behavior change.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR demonstrates strong consistency in several areas: the `FrontmatterImport` enum correctly mirrors `ImportDirective` without the `offset` field (as stated in the PR description), the new limit constant in `limits.rs` follows the exact doc-comment and naming pattern of existing constants, the resolution logic in `resolve_frontmatter_imports` structurally parallels the body import resolution methods, and the comprehensive test coverage follows the existing test organization style. The main consistency gaps are the error variant mismatch for name collisions (using `import_error` where body imports use `name_collision`) and the stale test function names.

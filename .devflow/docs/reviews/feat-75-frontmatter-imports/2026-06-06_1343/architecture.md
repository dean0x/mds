# Architecture Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06
**PR**: #85

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Code duplication between frontmatter and body import resolution** - `resolver.rs:456-524`
**Confidence**: 82%
- Problem: `resolve_frontmatter_imports` duplicates the core resolution logic from `resolve_merge_import` (lines 526-549) and `resolve_selective_import` (lines 551-593). The merge arm (lines 477-493) mirrors lines 534-547 nearly verbatim. The selective arm (lines 494-519) mirrors lines 575-591. The only difference is the error-context wrapper (`attach_frontmatter_index` vs `attach_import_span`). This creates a maintenance risk: if merge/selective resolution logic changes (e.g., adding validation, changing prompt handling), two places must be updated in lockstep.
- Fix: Extract the common scope-population logic (resolve + insert functions/prompt into scope) into shared helpers parameterized by an error-context strategy. For example:
  ```rust
  fn apply_merge_to_scope(
      resolved: &ResolvedModule,
      scope: &mut Scope,
      err_context: impl Fn(String) -> MdsError,
  ) -> Result<(), MdsError> {
      for (name, func) in resolved.get_all_exports() {
          if scope.get_function(&name).is_some() {
              return Err(err_context(name));
          }
          scope.set_function(&name, func);
      }
      if let Some(val) = resolved.get_prompt_value() {
          scope.set_var("prompt", val);
      }
      Ok(())
  }
  ```
  Then both `resolve_merge_import` and the merge arm of `resolve_frontmatter_imports` call this helper. Same pattern for selective. This is a should-fix-while-here situation since you are adding the new code path.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Inconsistent `type: mds` detection between `strip_reserved_keys` and `has_type_mds_frontmatter_raw`** - `lib.rs:405-420` vs `resolver.rs:910-917`
**Confidence**: 83%
- Problem: `strip_reserved_keys` in `lib.rs` only checks true top-level lines (no leading whitespace via `starts_with(' ')` / `starts_with('\t')` guard at line 405), while `has_type_mds_frontmatter_raw` in `resolver.rs:912` uses `line.trim().strip_prefix("type:")` which would match indented `type: mds` lines inside nested YAML mappings. This means a `.md` file with nested `type: mds` inside a mapping could be incorrectly classified as an MDS file by `build_scope_from_frontmatter`, enabling frontmatter imports parsing on a file that should be treated as plain markdown. The `strip_reserved_keys` function correctly avoids this by only matching top-level keys, but the detection function does not.
- Fix: Make `has_type_mds_frontmatter_raw` consistent with the existing `has_type_mds_frontmatter` (which already uses `line.trim()`) or, better, check only top-level lines:
  ```rust
  fn has_type_mds_frontmatter_raw(raw: &str) -> bool {
      raw.lines().any(|line| {
          // Only match top-level (no leading whitespace) type: mds keys,
          // consistent with strip_reserved_keys behavior.
          let is_top_level = !line.starts_with(' ') && !line.starts_with('\t');
          is_top_level && line.strip_prefix("type:").is_some_and(|v| {
              let v = v.trim();
              v == "mds" || v == "\"mds\"" || v == "'mds'"
          })
      })
  }
  ```
  Note: `has_type_mds_frontmatter` (line 889) also uses `line.trim()` before `strip_prefix`, so it has the same pre-existing inconsistency. However, the new `_raw` variant was added in this PR and directly controls whether `imports` is parsed as structured imports, making the inconsistency actionable here.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`has_type_mds_frontmatter` matches indented `type: mds` lines** - `resolver.rs:896`
**Confidence**: 80%
- Problem: The pre-existing `has_type_mds_frontmatter` function (line 896) uses `line.trim()` before checking for `type:` prefix, which means `  type: mds` nested inside a YAML mapping would also match. This is inconsistent with `strip_reserved_keys` in `lib.rs` which explicitly checks for top-level keys only. In practice this is unlikely to cause issues because most `.md` files won't have nested `type: mds` keys, but it is a latent semantic inconsistency. The new `has_type_mds_frontmatter_raw` introduced in this PR (line 910) copies this same pattern.
- Fix: Both functions should check only top-level lines (no leading whitespace) to match the `strip_reserved_keys` behavior.

## Suggestions (Lower Confidence)

- **`type: mds` detection logic repeated in 4 places** - `lib.rs:417`, `resolver.rs:896`, `resolver.rs:912`, and the implied pattern in YAML parsing (Confidence: 72%) -- The `type: mds` matching logic (checking plain/single-quoted/double-quoted variants) is duplicated across four locations. A single `fn is_type_mds_value(s: &str) -> bool` helper would reduce drift risk. Not blocking because the logic is simple and stable.

- **`resolve_source` always passes `is_md: false`** - `resolver.rs:262` (Confidence: 65%) -- The `resolve_source` API (used by Node.js/WASM bindings) always treats source strings as `.mds` files. If a caller passes source that is semantically a `.md` file with `type: mds` frontmatter, the `type` key would be exposed as a scope variable instead of being filtered. This is a pre-existing design choice rather than a bug, but worth documenting.

- **`parse_frontmatter_imports` double-parses YAML when called from `scan_imports`** - `resolver.rs:1135-1148` vs `lib.rs:805` (Confidence: 62%) -- `scan_imports` in `lib.rs` calls `parse_frontmatter_imports(&fm.raw)` which parses the YAML a second time (the first parse happens in the lexer/parser). Meanwhile, `build_scope_from_frontmatter` already parses YAML once and uses `parse_frontmatter_imports_from_yaml` on the parsed value. The `scan_imports` path is a lightweight scanning API where the double-parse is unlikely to be a performance concern, but it is an asymmetry worth noting.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The architecture is well-structured overall. The new `FrontmatterImport` enum cleanly mirrors the existing `ImportDirective` AST type, and the resolution is correctly integrated into the existing `process_module` pipeline at the right point (after scope creation, before body imports). The single-parse approach in `build_scope_from_frontmatter` avoids redundant YAML parsing. The resource limit (`MAX_FRONTMATTER_IMPORTS = 256`) follows the existing defense-in-depth pattern (applies ADR-008 by adding a related feature into the same module structure). The namespace collision detection gap fix in body alias imports (line 444-446) is a good defensive addition.

The primary architectural concern is the code duplication between frontmatter and body import resolution paths. This is a medium-severity issue because the duplicated logic is straightforward and well-tested, but extracting shared helpers would improve maintainability as the import system evolves. The `type: mds` detection inconsistency should be addressed to prevent subtle misclassification of `.md` files.

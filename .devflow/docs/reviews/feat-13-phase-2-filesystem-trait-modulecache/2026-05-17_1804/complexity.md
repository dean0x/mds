# Complexity Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**`process_module` has 7 parameters (threshold: 5)** - `resolver.rs:256`
**Confidence**: 90%
- Problem: `process_module` accepts 7 parameters (`&mut self`, `source`, `file_str`, `base_key`, `is_md`, `runtime_vars`, `warnings`). Even counting `&mut self` separately, the 6 remaining value/reference parameters exceed the warning threshold. In the new code, `file_str` and `base_key` are passed the same value (`key`) from `resolve_by_key` (line 156), which makes the separate parameters confusing -- the caller passes `key, key` for two different conceptual roles.
- Fix: Bundle the per-module context into a struct. The existing `ModuleCtx` already holds `file_str` and `base_key` -- construct it before calling `process_module` and pass it instead of the individual fields:
  ```rust
  fn process_module(
      &mut self,
      source: &str,
      ctx: &ModuleCtx<'_>,
      is_md: bool,
      runtime_vars: &HashMap<String, Value>,
      warnings: &mut Vec<String>,
  ) -> Result<ResolvedModule, MdsError> { ... }
  ```
  This reduces the parameter count to 5 (excluding `&mut self`) and eliminates the `key, key` double-pass at the call site.

### MEDIUM

**`resolve_selective_import` has 7 parameters** - `resolver.rs:440`
**Confidence**: 85%
- Problem: `resolve_selective_import` takes `&mut self`, `names`, `path`, `offset`, `scope`, `ctx`, and `warnings` -- 7 total. The `path` and `offset` values are always destructured from an `ImportDirective::Selective` variant. All three import-variant resolvers (`resolve_alias_import`, `resolve_merge_import`, `resolve_selective_import`) share the same `(path, offset, scope, ctx, warnings)` tail, suggesting a common pattern.
- Fix: Since `ctx` already bundles most context, consider passing the import directive variant directly or at minimum bundling `(path, offset)` into a small struct. Alternatively, the three variant resolvers could be inlined into `resolve_import`'s match arms -- each is only 10-20 lines -- eliminating the parameter forwarding entirely.

**`collect_export` match arms contain duplicated `resolve_import_from` call sequences** - `resolver.rs:341-397`
**Confidence**: 82%
- Problem: The `ReExport` and `Wildcard` arms both perform the same 5-line `self.resolve_import_from(ctx.base_key, import_path, ctx.runtime_vars, warnings)?` call. The duplication is modest (2 occurrences) but combined with the large match body, the function spans 57 lines -- above the 50-line warning threshold.
- Fix: Extract the shared resolution into a local helper or restructure the match to resolve first and then dispatch on the export kind:
  ```rust
  ExportDirective::ReExport { name, path } | ExportDirective::Wildcard { path } => {
      let source_module = self.resolve_import_from(ctx.base_key, path, ctx.runtime_vars, warnings)?;
      // then match on name.is_some() vs wildcard
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`validate_file_type` has 4 nesting levels in the `.md` frontmatter check** - `resolver.rs:712-726`
**Confidence**: 83%
- Problem: The `.md` frontmatter validation chain uses `strip_prefix` -> `and_then` -> `is_some_and` -> `lines().any()` -> `strip_prefix` -> `is_some_and` with a nested closure 4 levels deep. While the iterator-chain style avoids explicit nesting, the mental model requires tracking 4 transformations to understand the logic. This function was moved unchanged from the previous version but its signature was modified (`&Path` -> `&str`).
- Fix: Extract the frontmatter check into a named function:
  ```rust
  fn has_type_mds_frontmatter(source: &str) -> bool {
      let fm = source
          .strip_prefix("---\n")
          .or_else(|| source.strip_prefix("---\r\n"))
          .and_then(|after| after.find("\n---").map(|end| &after[..end]));
      let Some(fm) = fm else { return false };
      fm.lines().any(|line| {
          let Some(v) = line.trim().strip_prefix("type:") else { return false };
          let v = v.trim();
          v == "mds" || v == "\"mds\"" || v == "'mds'"
      })
  }
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`resolve_by_key` is 69 lines with 7 decision points** - `resolver.rs:122-191` (Confidence: 70%) -- The function handles cache lookup, cycle detection, depth check, file read, type validation, LIFO push/pop, double-fault handling, and cache insertion. The LIFO invariant section alone (lines 158-183) is 25 lines. Consider extracting the LIFO guard into a helper that returns the resolved module or an error.

- **`VirtualFs::normalize` duplicates traversal validation with `validate_import_path`** - `fs.rs:64-112` (Confidence: 65%) -- Both `VirtualFs::normalize` and `validate_import_path` (resolver.rs:685) check for null bytes and empty paths. Since `resolve_import_from` calls `validate_import_path` before `normalize`, the null-byte and empty-path checks in `VirtualFs::normalize` are technically redundant for import flows. However, `normalize` is also called directly for root entry points where `validate_import_path` is skipped, so the duplication is defensible.

- **`NativeFs::normalize` checks `base.is_empty()` twice** - `fs.rs:222-244` (Confidence: 62%) -- The function checks `base.is_empty()` at line 223 (to decide path construction) and again at line 235 (to decide root initialization). These could be unified into a single branch.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The refactoring from `PathBuf`-keyed resolution to `String`-keyed resolution with the `FileSystem` trait is a net complexity reduction. The resolver dropped from 801 to 801 lines but removed multiple security helper methods (`check_symlink`, `check_path_traversal`, `canonicalize_and_check`, `read_validated_file`, `resolve_path`, `find_project_root`) that were monolithically embedded -- these are now properly separated into `fs.rs` (276 lines of production code + 240 lines of tests). Individual functions are well-sized with clear single responsibilities. The main complexity concerns are parameter counts on `process_module` and the three import-variant resolvers, which could be addressed with modest restructuring. The `resolve_by_key` function is at the upper edge of comfortable complexity but is well-commented and follows a clear sequential protocol.

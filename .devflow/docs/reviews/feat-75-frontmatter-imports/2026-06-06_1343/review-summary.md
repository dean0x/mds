# Code Review Summary

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06_1343

## Merge Recommendation: CHANGES_REQUESTED

**Reasoning**: The PR introduces a well-structured frontmatter imports feature with strong security and reliability practices. However, there are 3 CRITICAL/HIGH blocking issues that must be fixed before merge:

1. **HIGH**: Blank lines in `imports:` YAML block prematurely terminate stripping, leaking import paths into compiled output (Rust review)
2. **HIGH**: Error type mismatch — frontmatter alias/merge imports use `import_error` instead of `name_collision` (Consistency review)
3. **HIGH**: Test does not actually exercise frontmatter imports in expressions (Testing review)

These issues span different categories (functional correctness, consistency, test coverage) and must be addressed before approval.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 3 | 4 | 0 | 7 |
| Should Fix | 0 | 0 | 4 | 0 | 4 |
| Pre-existing | 0 | 0 | 4 | 0 | 4 |

---

## Critical & High Bugs

### **HIGH: Blank lines in `imports:` YAML block leak into compiled output** — `lib.rs:405-427`
**Confidence**: 85% (Rust reviewer)
- **Problem**: The stripping logic in `strip_reserved_keys` treats blank lines as top-level keys, resetting the `in_imports_block` flag. A YAML frontmatter with a blank line within the `imports:` block:
  ```yaml
  imports:
    - path: ./a.mds

    - path: ./b.mds
  ```
  causes lines after the blank line to leak into the compiled output because the blank line (which has no leading whitespace) exits the imports-block context. Meanwhile, `build_scope_from_frontmatter` parses the YAML correctly via serde, so the imports _resolve_ but the paths _leak_ into the output.
- **Fix**: Add an empty-line guard that continues stripping when `in_imports_block` is true:
  ```rust
  let trimmed = line.trim();
  if trimmed.is_empty() {
      if in_imports_block {
          continue; // blank lines inside imports block are also stripped
      }
      filtered.push_str(line);
      filtered.push('\n');
      continue;
  }
  ```

### **HIGH: Inconsistent error type for name collision in frontmatter imports** — `resolver.rs:467, 483`
**Confidence**: 90% (Consistency reviewer)
- **Problem**: Body alias imports use `MdsError::name_collision(alias)` (line 445), but frontmatter alias imports use `MdsError::import_error(format!("name collision: ..."))` (line 467). Same inconsistency exists for merge imports (frontmatter line 483 vs body line 541). This means error-matching code that catches `MdsError::NameCollision` will not catch frontmatter collisions, breaking error handling consistency.
- **Fix**: Use the same `MdsError::name_collision()` constructor for both frontmatter and body imports, consistent with each other:
  ```rust
  FrontmatterImport::Alias { path, alias } => {
      if scope.get_namespace(alias).is_some() {
          return Err(MdsError::name_collision(alias.to_string()));
      }
      // ...
  }
  ```

### **HIGH: Test `fm_import_for_expr` does not exercise what it claims to test** — `virtual_fs.rs:629-655`
**Confidence**: 95% (Testing reviewer)
- **Problem**: The test comment claims it validates `@for x in lib.split_items(csv, ",")` (using the namespaced alias import), but the actual template body (line 646) uses `@for x in split(csv, ",")` — the built-in function, not the imported alias. The `lib` module is imported but never used in the template. The test passes even if frontmatter alias imports are broken in expression contexts.
- **Fix**: Change line 646 to actually use the imported namespace:
  ```rust
  "@for x in lib.split_items(csv, \",\"):\n",
  ```

---

## Should-Fix Issues

### **MEDIUM: Code duplication between frontmatter and body import resolution** — `resolver.rs:456-524`
**Confidence**: 82% (Architecture reviewer)
- **Problem**: The `resolve_frontmatter_imports` function duplicates core resolution logic from `resolve_merge_import` (lines 526-549) and `resolve_selective_import` (lines 551-593). The merge arm mirrors lines 534-547 nearly verbatim, and the selective arm mirrors lines 575-591. Only the error-context wrapper differs. This creates maintenance risk: if merge/selective resolution logic changes, both places must be updated in lockstep.
- **Fix**: Extract common scope-population logic into shared helpers parameterized by an error-context strategy (e.g., `apply_merge_to_scope`, `apply_selective_to_scope` with a closure for error context).

### **MEDIUM: High cyclomatic complexity in `parse_frontmatter_imports_from_yaml`** — `resolver.rs:1022-1129`
**Confidence**: 85% (Complexity reviewer)
- **Problem**: This function has ~108 lines with 4 nesting levels, multiple match arms, nested loops, and high cyclomatic complexity (~15-18). It handles type extraction, path validation, unknown-key scanning, and the `(as, names)` dispatch all in one body, exceeding the complexity warning threshold.
- **Fix**: Extract per-entry parsing into a dedicated `parse_single_import_entry` helper that returns a `FrontmatterImport`. This reduces the main function to sequence-level validation + loop, moving per-entry logic into a focused ~60-line helper function.

### **MEDIUM: Test name mismatch — functions named `strip_type_mds_*` but test `strip_reserved_keys`** — `lib.rs:948-1012`
**Confidence**: 95% (Consistency reviewer)
- **Problem**: Six test functions still use the old naming `strip_type_mds_plain_value`, `strip_type_mds_double_quoted`, etc. while the function under test was renamed to `strip_reserved_keys`. The section comment was updated to reference the new name, but the test function names were not. Newer tests in the same module correctly use `strip_reserved_keys_*` naming.
- **Fix**: Rename the six test functions to `strip_reserved_keys_plain_value`, `strip_reserved_keys_double_quoted`, etc., matching the function under test.

### **MEDIUM: Missing test for `.md` without `type: mds` exemption** — `virtual_fs.rs:900-921`
**Confidence**: 82% (Testing reviewer)
- **Problem**: `fm_import_md_without_type_mds` tests the exemption via `scan_imports` only, which is source-only and does not check file types. The actual exemption logic lives in `build_scope_from_frontmatter` (resolver.rs:792-800) where `is_mds` gates the behavior. No integration test compiles a `.md` file with `imports` as a regular variable and verifies it appears in scope.
- **Fix**: Add an integration test:
  ```rust
  #[test]
  fn fm_import_md_plain_treats_imports_as_var() {
      let mut modules = HashMap::new();
      modules.insert(
          "main.md".to_string(),
          "---\nimports: hello\n---\n{imports}\n".to_string(),
      );
      let output = compile_vfs(modules, "main.md").expect("plain .md should compile");
      assert!(output.contains("hello"), "got: {output}");
  }
  ```

---

## Suggestions (Lower Confidence)

(Confidence 60–79%, not blocking)

- **Duplicate names within selective import `names` list silently accepted** — `resolver.rs:1106-1118` (80%, Rust) — `names: [greet, greet]` passes validation and silently overwrites. Use a `HashSet` during parsing to reject duplicates.

- **Non-string YAML keys in import entries silently ignored** — `resolver.rs:1068-1071` (80%, Rust) — `{ path: ./lib.mds, 42: something }` passes validation. Reject non-string keys explicitly.

- **Inconsistent `type: mds` detection (pre-existing)** — `resolver.rs:896, 912` vs `lib.rs:417` (82%, Architecture/Consistency/Rust) — Both `has_type_mds_frontmatter` and `has_type_mds_frontmatter_raw` use `line.trim()`, matching indented `type: mds` inside nested YAML. Only top-level keys should match (consistent with `strip_reserved_keys`). This is pre-existing but newly consequential with frontmatter imports.

- **Extra allocation in `strip_reserved_keys`** — `lib.rs:437` (65%, Performance) — `format!("{trimmed}\n")` allocates after building filtered string. Negligible impact.

- **Duplicated `type: mds` matching logic** — `resolver.rs:896, 912`, `lib.rs:417` (70%, Complexity) — Pattern `v == "mds" || v == "\"mds\"" || v == "'mds'"` appears in three places. Extract `is_mds_value(v: &str) -> bool` helper.

- **Test assertions are weak** — `virtual_fs.rs:653-654` (65%, Testing) — `output.contains('a')` is fragile; use more specific assertions like `contains("a\n")` or full-line matches.

- **Missing merge-import collision test** — `virtual_fs.rs` (80%, Testing) — Alias collisions are tested but merge-import collisions (resolver.rs:481-487) have no dedicated test.

- **Missing positive test for `--set imports` on plain `.md`** — `virtual_fs.rs` (85%, Testing) — Tests cover blocking and error cases but not the positive case where `--set imports=foo` succeeds on a plain `.md` file without `type: mds`.

---

## Convergence Status

**Cycle**: 1
**Prior Resolution**: none (first review cycle)
**Prior FP Ratio**: N/A (first cycle)
**Assessment**: First cycle — all findings are new issues from the initial review.

---

## Summary by Reviewer

### Architecture (Score: 8/10)
Well-structured integration of frontmatter imports into the existing import system. The `FrontmatterImport` enum cleanly mirrors `ImportDirective`. Primary concern: code duplication with body import resolution paths.

### Security (Score: 9/10)
Strong security posture. Path validation, resource limits (`MAX_FRONTMATTER_IMPORTS=256`), identifier validation, cycle detection, and `--set imports` blocking all follow existing patterns. No new attack surfaces.

### Performance (Score: 9/10)
Single-parse YAML strategy, pre-sized allocations, and defense-in-depth limits are well-designed. No performance regressions.

### Complexity (Score: 6/10)
HIGH cyclomatic complexity in `parse_frontmatter_imports_from_yaml` (108 lines, ~15-18 complexity). Extract per-entry parsing helper. File size growth is notable but acceptable. Duplication with body import resolution should be addressed.

### Consistency (Score: 7/10)
BLOCKING: Error type mismatch for name collisions. MEDIUM: Test function names don't match renamed function. MEDIUM: `type: mds` detection inconsistency (trim vs strict top-level check).

### Regression (Score: 9/10)
No regressions in public APIs or existing behavior. One stale comment reference to old function name. The `scan_imports` behavior enhancement is additive and well-tested.

### Testing (Score: 7/10)
BLOCKING: `fm_import_for_expr` doesn't actually use the imported alias. MEDIUM: Missing integration test for `.md` without `type: mds`. MEDIUM: Missing merge-import collision test. MEDIUM: Missing positive `--set imports` test. Overall ~37 new tests provide good coverage when fixed.

### Reliability (Score: 9/10)
Bounded iteration, cycle detection, error context enrichment, collision guards, and resource limits are all in place. Inherits reliability guarantees from existing `resolve_import_from` path.

### Rust (Score: 8/10)
BLOCKING: Blank lines in `imports:` block leak into output. MEDIUM: Duplicate names in selective imports silently accepted. MEDIUM: Non-string YAML keys silently ignored. Well-structured `FrontmatterImport` enum design.

---

## Action Plan

1. **Fix blank-line stripping bug** — Add empty-line guard in `strip_reserved_keys`
2. **Fix error type inconsistency** — Use `MdsError::name_collision()` for frontmatter imports
3. **Fix test assertion** — Change `fm_import_for_expr` to use the imported namespace
4. **Rename test functions** — Update `strip_type_mds_*` to `strip_reserved_keys_*`
5. **Extract complexity** — Split `parse_frontmatter_imports_from_yaml` into helper function
6. **Add missing tests** — Merge collision, `.md` variable, positive `--set imports` tests
7. **Add validation** — Reject duplicate names, non-string keys in imports
8. **Fix consistency** — Correct `type: mds` detection to top-level only

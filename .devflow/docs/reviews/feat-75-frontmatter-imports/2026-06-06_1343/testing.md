# Testing Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06
**PR**: #85

## Issues in Your Changes (BLOCKING)

### HIGH

**Test does not exercise the behavior described in its comment** - `crates/mds-core/tests/virtual_fs.rs:629-655`
**Confidence**: 95%
- Problem: `fm_import_for_expr` comment says it tests `@for x in lib.split_items(csv, ",")` (using the namespaced alias import), but the actual template body on line 646 uses `@for x in split(csv, ",")` — the built-in function, not the imported one. The `lib` module is imported via frontmatter but never referenced in the template body. This means the test passes even if frontmatter alias imports are completely broken in expression contexts.
- Fix: Change line 646 to actually use the imported namespace:
  ```rust
  "@for x in lib.split_items(csv, \",\"):\n",
  ```
  This makes the test exercise what it claims to test — that frontmatter alias imports are usable inside `@for` expressions.

### MEDIUM

**`.md` without `type: mds` exemption is tested via `scan_imports`, not via compilation** - `crates/mds-core/tests/virtual_fs.rs:900-921`
**Confidence**: 82%
- Problem: `fm_import_md_without_type_mds` claims to validate that a plain `.md` file treats `imports` as a regular variable, but it tests this via `scan_imports` (which is source-only and does not check file types). The test comments acknowledge this limitation explicitly (lines 910-913). The actual exemption logic lives in `build_scope_from_frontmatter` (resolver.rs:792-800) where `is_mds` gates the behavior, but no integration test compiles a `.md` file with `imports` as a regular variable and verifies the value appears in scope.
- Fix: Add an integration test that compiles a `.md` file (no `type: mds`) with `imports: some_value` in frontmatter and asserts `{imports}` renders as `some_value` in the output:
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

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing test for `--set imports=foo` allowed on plain `.md` without `type: mds`** - `crates/mds-core/tests/virtual_fs.rs:865-898`
**Confidence**: 85%
- Problem: Tests cover `--set imports` being blocked for `.mds` files (`fm_import_set_blocked`) and for `.md` with `type: mds` (`fm_import_set_blocked_md_type_mds`), but there is no positive test verifying that `--set imports=foo` is *allowed* on a plain `.md` file. The three tests form a truth table with one row missing: `.md` without `type: mds` + `--set imports` should succeed.
- Fix: Add the complementary positive test:
  ```rust
  #[test]
  fn fm_import_set_allowed_md_plain() {
      let mut modules = HashMap::new();
      modules.insert("main.md".to_string(), "Hello {imports}!\n".to_string());
      let mut vars = HashMap::new();
      vars.insert("imports".to_string(), Value::String("world".to_string()));
      let output = mds::compile_virtual(modules, "main.md", Some(vars))
          .expect("--set imports should be allowed for plain .md");
      assert!(output.contains("Hello world!"), "got: {output}");
  }
  ```

**No test for merge-import name collision in frontmatter** - `crates/mds-core/tests/virtual_fs.rs:722-779`
**Confidence**: 80%
- Problem: Both collision tests (`fm_import_collision_with_body` and `fm_import_collision_within_fm`) use alias imports. The merge-import collision path at resolver.rs:481-487 (where a merge-imported function name collides with an already-defined function) has no dedicated test. This code path is distinct — it checks `scope.get_function()` rather than `scope.get_namespace()`.
- Fix: Add a merge collision test:
  ```rust
  #[test]
  fn fm_import_merge_collision() {
      let mut modules = HashMap::new();
      modules.insert("a.mds".to_string(), "@define f():\nA\n@end\n".to_string());
      modules.insert("b.mds".to_string(), "@define f():\nB\n@end\n".to_string());
      modules.insert(
          "main.mds".to_string(),
          "---\nimports:\n  - path: ./a.mds\n  - path: ./b.mds\n---\n{f()}\n".to_string(),
      );
      let err = compile_vfs(modules, "main.mds")
          .expect_err("merge collision should fail");
      let msg = err.to_string();
      assert!(
          msg.contains("collision") || msg.contains("already defined"),
          "expected collision error, got: {msg}"
      );
  }
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing testing issues identified.

## Suggestions (Lower Confidence)

- **Weak single-char assertions in `fm_import_for_expr`** - `crates/mds-core/tests/virtual_fs.rs:653-654` (Confidence: 65%) — Asserting `output.contains('a')` on output that may contain "main" or other text with 'a' is a fragile assertion. Consider asserting on the full expected line or using `contains("a\n")`.

- **Missing assertion for 'c' in `fm_import_for_expr`** - `crates/mds-core/tests/virtual_fs.rs:653-654` (Confidence: 60%) — The test splits "a,b,c" but only asserts 'a' and 'b' are present, not 'c'. While unlikely to mask a bug, asserting all expected items improves test confidence.

- **`has_type_mds_frontmatter_raw` has no direct unit test** - `crates/mds-core/src/resolver.rs:910-917` (Confidence: 65%) — This new helper function is only tested indirectly through integration tests. A direct unit test would catch edge cases like indented `type: mds` lines (which currently match due to `line.trim()`).

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The test suite is thorough overall — ~37 new tests cover the three import forms, collisions, circular imports, output stripping, dependency tracking, and `scan_imports` ordering well. The blocking HIGH issue is the `fm_import_for_expr` test that does not actually test what its comment describes (the imported alias is never used in the template), which leaves a real coverage gap for frontmatter imports in expression contexts. The MEDIUM issues are coverage gaps for the `.md` exemption path and merge-import collisions.

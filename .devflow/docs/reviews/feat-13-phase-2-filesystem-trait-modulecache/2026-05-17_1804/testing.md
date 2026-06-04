# Testing Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing error-path tests for VirtualFs subdirectory imports** - `crates/mds-core/tests/virtual_fs.rs`
**Confidence**: 85%
- Problem: The `virtual_fs.rs` integration tests cover happy-path subdirectory imports (e.g., `two_file_import_merge`, `three_file_chain`) but all modules live at the root level (flat keys like `"lib.mds"`, `"a.mds"`). No test verifies that VirtualFs correctly resolves imports between modules in different subdirectories (e.g., `"components/header.mds"` importing `"../shared/footer.mds"`). The unit tests in `fs.rs` test `normalize` with subdirectory paths, but the integration tests never exercise the full pipeline with nested key hierarchies.
- Fix: Add an integration test with nested keys:
```rust
#[test]
fn subdirectory_cross_import() {
    let mut modules = HashMap::new();
    modules.insert(
        "shared/utils.mds".to_string(),
        "@define greet(x):\nHi {x}!\n@end\n".to_string(),
    );
    modules.insert(
        "pages/main.mds".to_string(),
        "@import \"../shared/utils.mds\"\n{greet(\"World\")}\n".to_string(),
    );
    let output = compile_vfs(modules, "pages/main.mds").expect("should compile");
    assert!(output.contains("Hi World!"), "got: {output}");
}
```

**No test for `NativeFs::set_root` behavior** - `crates/mds-core/src/fs.rs:268`
**Confidence**: 82%
- Problem: `NativeFs::set_root` is a new public trait method that is called by `resolve_source` (the `compile_str_with` path). There are zero unit tests for `set_root` in `fs::tests`. The only coverage comes from the integration test `compile_str_with_import_resolves_relative_to_base_dir` in `api_surface.rs`, but that test does not verify the root was correctly set (only that the import resolved). If `set_root` silently fails (e.g., the `OnceLock` was already set), the test would still pass since traversal checks are permissive when no root is set.
- Fix: Add a unit test in `fs::tests`:
```rust
#[test]
fn native_set_root_then_traversal_rejected() {
    let dir = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let _entry = make_temp_file(&dir, "main.mds", "hello");
    let outside_file = make_temp_file(&outside, "secret.mds", "secret");

    let fs = NativeFs::new();
    fs.set_root(&dir.path().canonicalize().unwrap().display().to_string())
        .expect("set_root should succeed");

    let result = fs.normalize("", &outside_file.display().to_string());
    assert!(result.is_err(), "should reject path outside root");
}
```

### MEDIUM

**`export_visibility` test does not verify that `internal` is actually hidden** - `crates/mds-core/tests/virtual_fs.rs:129-144`
**Confidence**: 85%
- Problem: The test comment says "greet is exported; internal is not" but the test only verifies that `greet` works. It never asserts that `internal` is inaccessible. The test would pass even if `@export` had no effect and all functions were public. A behavior-focused test should assert that calling `internal()` from the importer fails.
- Fix: Add a negative assertion:
```rust
#[test]
fn export_visibility_hides_non_exported() {
    let mut modules = HashMap::new();
    modules.insert(
        "lib.mds".to_string(),
        "@define internal():\nhidden\n@end\n@define greet(x):\nHi {x}!\n@end\n@export greet\n"
            .to_string(),
    );
    modules.insert(
        "main.mds".to_string(),
        "@import \"./lib.mds\"\n{internal()}\n".to_string(),
    );
    let err = compile_vfs(modules, "main.mds").expect_err("internal should not be accessible");
    assert!(
        matches!(err, MdsError::UndefinedFunction { .. }),
        "expected UndefinedFunction, got {err:?}"
    );
}
```

**`module_cache_with_fs_constructor` is a smoke test only** - `crates/mds-core/tests/api_surface.rs:232-236`
**Confidence**: 80%
- Problem: The test constructs a `ModuleCache::with_fs(Box::new(NativeFs::new()))` but never uses it. This only verifies the constructor compiles, not that the cache actually uses the injected filesystem. Since `with_fs` is the primary extensibility point for custom `FileSystem` implementations, it should be tested with an actual resolution to confirm the injected fs is wired correctly.
- Fix: Either resolve a module through the `with_fs` cache, or add an integration test with a custom `FileSystem` implementation to verify the injected filesystem is actually used.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`native_normalize_path_traversal_rejected` has a weak assertion chain** - `crates/mds-core/src/fs.rs:452-483`
**Confidence**: 82%
- Problem: The test uses `let _ = fs.normalize(...)` for the absolute path injection case (line 466), silently discarding whether it succeeded or failed. The comment says "may or may not error depending on platform path depth" but this means the test is non-deterministic on this assertion. The second part (relative traversal with 20 `..` segments) is also fragile -- it accepts three different error messages (`escapes project`, `symlinks`, or `not found`), making it unclear which security boundary is actually being tested. If the traversal guard broke and the file happened to not exist, the test would still pass via `not found`.
- Fix: Separate the test into two focused tests: one for absolute-path injection and one for relative traversal. For the relative traversal test, construct a path that definitively triggers the traversal guard and assert only `escapes project`:
```rust
#[test]
fn native_normalize_absolute_path_injection_rejected() {
    let project_dir = TempDir::new().unwrap();
    let outside_dir = TempDir::new().unwrap();
    let entry = make_temp_file(&project_dir, "main.mds", "hello");
    let outside = make_temp_file(&outside_dir, "secret.mds", "secret");

    let fs = NativeFs::new();
    let base_key = fs.normalize("", &entry.display().to_string()).expect("entry");
    let result = fs.normalize(&base_key, &outside.display().to_string());
    // Should error: absolute path to a file outside the project root
    assert!(result.is_err(), "should reject absolute path outside root");
}
```

## Pre-existing Issues (Not Blocking)

### LOW

**Existing CLI import tests duplicate VirtualFs integration tests** - `crates/mds-cli/tests/imports.rs`
**Confidence**: 80%
- Problem: Many of the new `virtual_fs.rs` integration tests (merge import, selective import, namespace import, wildcard re-export, circular import) exercise the same behavioral contracts as the existing CLI-level tests in `crates/mds-cli/tests/imports.rs`. This is not harmful but creates maintenance overhead -- if a behavioral change is made, both test files need updating.
- Observation: The VirtualFs tests are still valuable because they exercise the pipeline without OS filesystem dependencies, making them faster and suitable for WASM targets. Consider adding a comment in `virtual_fs.rs` noting these are the VirtualFs equivalents of the CLI tests, to help maintainers understand the intentional overlap.

## Suggestions (Lower Confidence)

- **Missing VirtualFs test for `.md` files with `type: mds` frontmatter** - `crates/mds-core/tests/virtual_fs.rs` (Confidence: 75%) -- The `validate_file_type` function now uses string-based key extension detection. No VirtualFs integration test verifies that a `.md` key with `type: mds` frontmatter is accepted or that a `.md` key without it is rejected.

- **No test for `NativeFs::read` rejecting files larger than MAX_FILE_SIZE** - `crates/mds-core/src/fs.rs:246-256` (Confidence: 70%) -- The file-size limit check was moved from `resolver::read_validated_file` to `NativeFs::read`. There is an existing CLI-level test (`file_size_limit_rejects_huge_file` in `security.rs`), but no unit test for the `NativeFs::read` method directly. If someone replaces `NativeFs` with a custom `FileSystem` that omits the check, the CLI test would still catch it, but it would be unclear which layer is responsible for enforcement.

- **`compile_virtual` doc-test duplicates integration test** - `crates/mds-core/src/lib.rs:429-437` (Confidence: 65%) -- The `#[must_use]` doc-test for `compile_virtual` is nearly identical to `compile_virtual_exists` in `api_surface.rs` and `single_file_compile` in `virtual_fs.rs`. Not a problem per se, but the doc-test is the canonical one; consider removing the redundant `compile_virtual_exists` test.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 1 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The new test suite is well-structured and covers the major VirtualFs compilation paths (single file, imports, cycles, depth limits, re-exports). The test naming is clear and follows AAA structure. However, two high-severity gaps should be addressed: (1) no integration test exercises cross-subdirectory imports in VirtualFs, which is the primary use case for the path normalization logic, and (2) the `set_root` method -- a security-relevant entry point -- has no direct test coverage. The `export_visibility` test also needs a negative assertion to actually verify the behavior it claims to test.

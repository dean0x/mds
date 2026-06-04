# Testing Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Missing negative assertion in `selective_import` test** - `crates/mds-core/tests/virtual_fs.rs:114-127`
**Confidence**: 85%
- Problem: The `selective_import` test defines two functions (`greet` and `farewell`) in `lib.mds` but only imports `greet`. The test asserts that `greet` works, but does not verify that `farewell` is inaccessible. Without a negative assertion, this test would pass even if selective import filtering were broken (i.e., if all symbols leaked through). The `export_visibility_non_exported_function_inaccessible` test at line 146 demonstrates the correct pattern -- a paired positive/negative test.
- Fix: Add a companion test `selective_import_excludes_non_imported`:
```rust
#[test]
fn selective_import_excludes_non_imported() {
    let mut modules = HashMap::new();
    modules.insert(
        "lib.mds".to_string(),
        "@define greet(x):\nHi {x}!\n@end\n@define farewell(x):\nBye {x}!\n@end\n".to_string(),
    );
    modules.insert(
        "main.mds".to_string(),
        "@import { greet } from \"./lib.mds\"\n{farewell(\"Bob\")}\n".to_string(),
    );
    let err = compile_vfs(modules, "main.mds")
        .expect_err("non-imported function should be inaccessible");
    let msg = err.to_string();
    assert!(
        msg.contains("farewell")
            || msg.contains("not defined")
            || msg.contains("not found")
            || msg.contains("undefined")
            || msg.contains("unknown"),
        "expected error about missing symbol, got: {msg}"
    );
}
```

**`compile_virtual_collecting_warnings` public API has no test** - `crates/mds-core/src/lib.rs:473-489`
**Confidence**: 82%
- Problem: The new public function `compile_virtual_collecting_warnings` is exercised only indirectly through `compile_virtual` (which delegates to it). There is no test that calls it directly or asserts on the returned warnings vector. The companion function `compile_collecting_warnings` for the NativeFs path has analogous CLI-level coverage, but the virtual variant does not. This is a gap in the API surface test coverage added in `api_surface.rs`.
- Fix: Add a test to `api_surface.rs` or `virtual_fs.rs`:
```rust
#[test]
fn compile_virtual_collecting_warnings_exists() {
    let mut modules = HashMap::new();
    modules.insert("main.mds".to_string(), "Hello!\n".to_string());
    let result = mds::compile_virtual_collecting_warnings(modules, "main.mds", None);
    assert!(result.is_ok(), "compile_virtual_collecting_warnings should succeed: {result:?}");
    let (output, warnings) = result.unwrap();
    assert_eq!(output, "Hello!\n");
    assert!(warnings.is_empty());
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`resolve_key_directly` test uses fragile value extraction pattern** - `crates/mds-core/tests/virtual_fs.rs:243-262`
**Confidence**: 80%
- Problem: The `resolve_key_directly` test extracts the prompt body via `get_prompt_value()` matching `Some(Value::String(ref s))`, falling back to an empty string. If the return type of `get_prompt_value()` ever changes or if the module resolves to a different `Value` variant, the test would silently pass with an empty body rather than failing explicitly. The fallback `String::new()` masks potential regressions.
- Fix: Replace the silent fallback with an explicit assertion:
```rust
let body = match resolved.get_prompt_value() {
    Some(Value::String(ref s)) => s.clone(),
    other => panic!("expected Value::String for prompt body, got: {other:?}"),
};
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **10 MB allocation in boundary tests** - `crates/mds-core/src/fs.rs:618-636` (Confidence: 65%) -- The `vfs_read_over_size_limit_errors` and `vfs_read_at_size_limit_ok` tests each allocate a 10 MB+ string. While correct for boundary testing, these allocations could slow down test runs on memory-constrained CI environments. Consider using a `#[ignore]` attribute or a feature gate if test suite speed becomes a concern.

- **`ModuleCache::with_fs` constructor tested only for compilation, not behavior** - `crates/mds-core/tests/api_surface.rs:232-236` (Confidence: 62%) -- The `module_cache_with_fs_constructor` test only verifies that the constructor compiles and can be called. It does not exercise any resolve/compile behavior through the custom FS. An integration test using `with_fs` + a custom `FileSystem` impl would strengthen coverage of the extension point.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

The test suite for this PR is strong. Key observations:

1. **Good test shape**: 24 unit tests for `fs.rs` internals, 13 integration tests in `virtual_fs.rs`, 8 API surface tests -- follows the testing pyramid well.
2. **Behavior-focused**: Tests exercise real compiler pipelines through `compile_vfs` rather than mocking internals. The `VirtualFs` itself serves as a proper test fake (in-memory filesystem substitute), which is the correct test double for filesystem abstraction.
3. **Error path coverage**: Circular imports, deep chain depth limits, file-not-found, path traversal, null byte injection, symlink rejection, and size limits are all tested with appropriate error variant matching.
4. **Security boundary tests**: NativeFs tests cover absolute path injection, relative traversal escape, and `set_root` enforcement -- thorough for a filesystem abstraction.
5. **Cross-implementation consistency**: The `vfs_is_markdown_matches_native_behavior` test ensures VirtualFs and NativeFs agree, which is an excellent pattern for trait implementations.

The two MEDIUM blocking items are genuine coverage gaps: the `selective_import` test lacks a negative assertion (the PR's own `export_visibility` tests demonstrate this pattern is known), and the `compile_virtual_collecting_warnings` public API function has no direct test despite being part of the public surface.

All 419 tests pass. No flaky patterns detected. Test names describe expected behavior. AAA structure is consistently followed.

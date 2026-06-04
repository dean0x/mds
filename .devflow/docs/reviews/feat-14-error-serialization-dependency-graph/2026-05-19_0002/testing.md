# Testing Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### HIGH

**`compile_with_deps` (NativeFs path) lacks integration test with real files** - `crates/mds-core/tests/api_surface.rs:277`
**Confidence**: 85%
- Problem: `compile_with_deps_exists` merely calls the function with a nonexistent file and discards the result (`let _ = ...`). This is an existence/smoke check, not a behavioral test. The NativeFs-backed `compile_with_deps` is the only `_with_deps` variant that exercises the `split_last()` entry-key exclusion logic against real canonicalized paths (lib.rs:531). `compile_virtual_with_deps` uses a simple string-equality filter (lib.rs:610), while `compile_str_with_deps` skips exclusion entirely (lib.rs:573). The NativeFs path is therefore architecturally distinct but has no test that verifies its dependency list is correct with actual file imports.
- Fix: Add an integration test in `api_surface.rs` (or a dedicated `tests/deps_native.rs`) that creates a tempdir with two `.mds` files, compiles via `compile_with_deps`, and asserts that `result.dependencies` contains the imported file but not the entry file:
```rust
#[test]
fn compile_with_deps_native_two_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let lib = dir.path().join("lib.mds");
    std::fs::write(&lib, "@define greet(x):\nHello {x}!\n@end\n").unwrap();
    let main = dir.path().join("main.mds");
    std::fs::write(&main, "@import \"./lib.mds\"\n{greet(\"World\")}\n").unwrap();

    let result = mds::compile_with_deps(&main, None).expect("should compile");
    assert!(result.output.contains("Hello World!"));
    assert_eq!(result.dependencies.len(), 1);
    assert!(result.dependencies[0].contains("lib.mds"));
    // Entry file must NOT appear in dependencies
    assert!(!result.dependencies.iter().any(|d| d.contains("main.mds")));
}
```

### MEDIUM

**`CompileOutput.warnings` field is never tested with actual warning content** - `crates/mds-core/tests/api_surface.rs:252` and `crates/mds-core/tests/virtual_fs.rs:294-417`
**Confidence**: 82%
- Problem: All `_with_deps` tests assert either `warnings: vec!["warn".to_string()]` (a synthetic construction in `compile_output_type_importable`) or use inputs that produce no warnings. No test verifies that `CompileOutput.warnings` captures real compiler warnings from the compilation pipeline. If the warning-threading from `process_module` to `CompileOutput` were broken, no test would catch it.
- Fix: Create a test case that triggers a real warning (if one exists in the compiler, e.g. empty `@include`) and asserts it appears in `result.warnings`. If no warning is easily triggerable via `compile_virtual_with_deps`, this is lower priority -- but worth noting as a coverage gap.

**`compile_str_with_deps` with file imports is untested** - `crates/mds-core/tests/virtual_fs.rs:387-401`
**Confidence**: 80%
- Problem: The test comment on line 389-391 explicitly acknowledges this gap: "compile_str_with_deps: inline source that imports a virtual module. Use a base_dir so the import resolution works; but with NativeFs that would look for real files. Skip this variant here -- covered in api_surface.rs." However, api_surface.rs also only tests the no-import case. The `compile_str_with_deps` function with imports (where `base_dir` matters and `dependencies` should be populated) has zero test coverage.
- Fix: Add a test using `tempfile::TempDir` that creates a lib file, then calls `compile_str_with_deps` with a source string containing `@import "./lib.mds"` and `base_dir` pointing to the tempdir:
```rust
#[test]
fn compile_str_with_deps_with_import() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("lib.mds"),
        "@define greet(x):\nHello {x}!\n@end\n",
    ).unwrap();
    let result = mds::compile_str_with_deps(
        "@import \"./lib.mds\"\n{greet(\"World\")}\n",
        Some(dir.path()),
        None,
    ).expect("should compile");
    assert!(result.output.contains("Hello World!"));
    assert!(!result.dependencies.is_empty());
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Dependency order assertion in `deps_three_file_chain` is comment-specified but fragile** - `crates/mds-core/tests/virtual_fs.rs:328-345`
**Confidence**: 80%
- Problem: The test comment says "deps of a = ['b.mds', 'c.mds'] in DFS order" but the assertion checks `["c.mds", "b.mds"]` (post-order DFS). The comment's stated order contradicts the asserted order. While the assertion itself is correct (it matches the actual post-order DFS implementation where leaves are inserted first), the comment text is misleading and could cause confusion during future maintenance.
- Fix: Update the comment on line 328 from `deps of a = ["b.mds", "c.mds"] in DFS order` to `deps of a = ["c.mds", "b.mds"] in post-order DFS` to match the actual assertion.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing roundtrip test for `SerializedError` deserialization** - `crates/mds-core/src/error.rs:778-809` (Confidence: 65%) -- The JSON serialization tests verify the output structure but never deserialize back to a Rust struct. Since `SerializedError` only derives `Serialize` (not `Deserialize`), this may be intentional -- but if consumers are expected to parse this JSON, a roundtrip assertion using `serde_json::Value` would strengthen confidence.

- **`deps_diamond_no_duplicates` does not verify exact ordering** - `crates/mds-core/tests/virtual_fs.rs:349-384` (Confidence: 62%) -- The diamond dependency test asserts all three deps are present and there are no duplicates, but uses `contains` checks rather than asserting exact order. Since the API contract specifies "first-resolution (depth-first) order", a stricter assertion like `assert_eq!(result.dependencies, vec!["shared.mds", "a.mds", "b.mds"])` would guard against ordering regressions. However, this may be intentionally relaxed if ordering is considered an implementation detail.

- **`module_cache_dependencies_exists` asserts entry key IS in dependencies** - `crates/mds-core/tests/api_surface.rs:309-318` (Confidence: 70%) -- This test calls `ModuleCache::dependencies()` directly (which includes the entry module) and asserts that `"main.mds"` is present. This is correct for the raw `dependencies()` API, but it may confuse readers because the `compile_*_with_deps` public API explicitly excludes the entry key. A clarifying comment distinguishing the low-level API from the public API would help.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The test suite is well-structured with strong coverage of the new serialization and VirtualFs-backed dependency tracking. The `compute_line_column` unit tests are thorough with excellent boundary case coverage. The `serialize()` tests cover all 16 error variants exhaustively. The dependency tracking tests cover single-file, chain, diamond, and error cases effectively. The main gap is that the NativeFs-backed `compile_with_deps` function -- which has a distinct entry-key exclusion mechanism (`split_last()` on canonicalized paths) compared to the VirtualFs variant (string equality filter) -- lacks any behavioral test with real file imports. This is the only HIGH finding and should be addressed before merge.

# Testing Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Missing exit code 3 (resource limit) integration test** - `tests/integration.rs`
**Confidence**: 90%
- Problem: The new `exit_code()` function in `src/main.rs:329` maps `MdsError::ResourceLimit` to exit code 3. Integration tests cover exit code 0 (success), 1 (syntax error), and 2 (file not found), but exit code 3 is never tested end-to-end. This is a new behavioral contract introduced in this PR. The `for_loop_iteration_limit_rejects_huge_array` test (line ~1773) only validates the error message, not the exit code.
- Fix: Add an integration test that triggers a resource limit (e.g., a loop exceeding MAX_LOOP_ITERATIONS) and asserts `status.code() == Some(3)`:
```rust
#[test]
fn exit_code_resource_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge_loop.mds");
    // 100_001 items exceeds MAX_LOOP_ITERATIONS (100_000)
    let items: Vec<String> = (0..100_001).map(|i| i.to_string()).collect();
    let source = format!(
        "---\nitems: [{}]\n---\n@for item in items:\n{{item}}\n@end\n",
        items.join(", ")
    );
    std::fs::write(&path, &source).unwrap();
    let status = mds_bin()
        .args(["build"])
        .arg(&path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to run mds");
    assert_eq!(status.code(), Some(3), "expected exit code 3 for resource limit");
}
```

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### HIGH

(none)

### MEDIUM

**New `check_collecting_warnings` and `check_str_collecting_warnings` API functions lack direct integration tests** - `src/lib.rs:288`, `src/lib.rs:317`
**Confidence**: 82%
- Problem: Two new public API functions were added to the library (`check_collecting_warnings` and `check_str_collecting_warnings`). While they are exercised indirectly through the CLI `check` command tests and have passing doc-tests, there are no direct integration tests in `tests/integration.rs` that call these functions and verify they return the correct `((), warnings)` tuple. The analogous `compile_collecting_warnings` is exercised directly via `include_respects_export_visibility_for_prompt` (line 1151, which calls it). A direct test would catch regressions in the warning-collection path independent of CLI argument parsing.
- Fix: Add a test that directly calls `mds::check_collecting_warnings` on a file with a known warning (e.g., `include_empty_body.mds`) and asserts warnings are returned:
```rust
#[test]
fn check_collecting_warnings_returns_warnings() {
    let ((), warnings) = mds::check_collecting_warnings(
        fixture("include_empty_body.mds"),
        None,
    ).unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("empty output")),
        "check_collecting_warnings should return empty-include warning, got: {warnings:?}"
    );
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Hardcoded `/tmp` path in `exit_code_file_not_found` test** - `tests/integration.rs:2435`
**Confidence**: 85%
- Problem: The test uses `"/tmp/no_such_file_12345.mds"` as a non-existent path. While this is very unlikely to collide, it is technically non-deterministic and non-portable (Windows has no `/tmp`). All other file-based tests correctly use `tempfile::tempdir()`.
- Fix: Use a path inside a temp directory instead:
```rust
let dir = tempfile::tempdir().unwrap();
let nonexistent = dir.path().join("no_such_file.mds");
```

## Suggestions (Lower Confidence)

- **No test for `check` command's exit code categorization** - `tests/integration.rs` (Confidence: 70%) -- The `check` command also routes through `exit_code()` on failure, but only the `build` command's exit codes are tested. A failing `check` could silently produce wrong exit codes without detection.

- **`build_to_file` test does not capture stdout/stderr** - `tests/integration.rs:750` (Confidence: 65%) -- Unlike the new file-output tests which carefully assert stdout is empty when writing to file, the pre-existing `build_to_file` test (which uses `-o <path>`) does not pipe stdout/stderr. Adding `.stdout(Stdio::piped())` and asserting emptiness would bring consistency with the new tests.

- **Repeated stdin-piping boilerplate across 5+ tests** - `tests/integration.rs` (Confidence: 62%) -- The pattern of `mds_bin().spawn()` -> `child.stdin.take().unwrap().write_all(...)` -> `child.wait_with_output()` is repeated in `check_stdin_valid`, `build_from_stdin`, `build_stdin_defaults_to_stdout`, `build_stdin_with_output_writes_file`, and `build_stdin_with_out_dir_writes_to_directory`. A small helper function (e.g., `mds_stdin_build(args, input) -> Output`) would reduce duplication and make intent clearer.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 1 | 0 |

**Testing Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The test suite is comprehensive and well-structured, growing from 245 to 276 tests across the PR. New features (exit codes, file output, mds.json config, `--out-dir`, stdin + output combos, export visibility) all have thorough integration coverage with proper use of temp directories and clear assertion messages. The test design follows behavior-focused patterns (asserting on output content and file existence, not implementation details). The one blocking issue is the missing exit code 3 test for resource limits -- a newly introduced behavioral contract that should have corresponding coverage before merge.

# Testing Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing test for mds.json output_dir path traversal guard** - `src/main.rs:149-158`
**Confidence**: 90%
- Problem: The PR adds a new security control in `resolve_output_path` that rejects `output_dir` values containing `..` components. This is a new error path with no integration test. The path traversal guard for *imports* (`path_traversal_import_rejected`) has a test, but this analogous guard for `mds.json output_dir` does not. New security-critical error paths should have behavioral tests verifying both the rejection and the error message.
- Fix: Add an integration test that creates an `mds.json` with `output_dir: "../escape"` and asserts the build fails with an error containing `"must not contain '..' components"`.

```rust
#[test]
fn build_mds_json_output_dir_rejects_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("hello.mds");
    std::fs::write(&src, "---\nname: World\n---\nHello {name}!\n").unwrap();
    std::fs::write(
        dir.path().join("mds.json"),
        r#"{"build":{"output_dir":"../escape"}}"#,
    ).unwrap();
    let output = mds_bin()
        .args(["build"])
        .arg(&src)
        .output()
        .expect("failed to run mds");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("must not contain '..'"),
        "expected path traversal rejection, got: {stderr}"
    );
}
```

**Missing test for mds.json config size limit** - `src/main.rs:54-64`
**Confidence**: 85%
- Problem: `MAX_CONFIG_SIZE` (1 MB) is a new security guard that prevents runaway memory allocation from maliciously large config files. Other resource limits in the codebase have corresponding tests (e.g., `file_size_limit_rejects_huge_file`, `stdin_size_limit_rejects_oversized_input`, `vars_file_size_limit_rejects_oversized_file`). This one does not. While creating a 1 MB+ temp file in a test is straightforward, the gap breaks the existing pattern of exhaustive resource-limit test coverage.
- Fix: Add an integration test that writes a >1 MB `mds.json` file and verifies the error message.

```rust
#[test]
fn build_mds_json_rejects_oversized_config() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("hello.mds");
    std::fs::write(&src, "---\nname: World\n---\nHello {name}!\n").unwrap();
    // Write a mds.json just over 1 MB
    let padding = "x".repeat(1024 * 1024 + 1);
    let json = format!("{{\"_padding\":\"{padding}\"}}");
    std::fs::write(dir.path().join("mds.json"), &json).unwrap();
    let output = mds_bin()
        .args(["build"])
        .arg(&src)
        .output()
        .expect("failed to run mds");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("too large"),
        "expected config size limit error, got: {stderr}"
    );
}
```

### MEDIUM

**exit_code_resource_limit test is slow and fragile** - `tests/integration.rs:3027-3067`
**Confidence**: 82%
- Problem: The test builds a YAML frontmatter with 2,002 items (two arrays of 1,001 each) to trigger `MAX_TOTAL_ITERATIONS`. This approach has two concerns: (1) It generates a large input file (~20 KB of YAML) making the test slower than necessary. (2) It relies on exact constant values (1,001 * 1,001 > 1,000,000) that are tightly coupled to internal constants. If `MAX_TOTAL_ITERATIONS` ever changes, the test silently stops testing what it claims to test. The existing `nested_loop_total_iteration_limit` test (line 2240) uses the library API with runtime vars which is both faster and more direct. However, since this test specifically validates the *exit code* (a CLI concern), it necessarily must use the binary. The size/speed concern is the primary issue.
- Fix: Use smaller arrays that still exceed the limit but minimize I/O. For example, 1,001 * 1,000 = 1,001,000 > 1,000,000. Or better, use `--set` flags to inject arrays at the CLI level and skip the large YAML frontmatter entirely. Also add a comment noting the constant dependency.

**No test for double-fault error preservation in invoke_function** - `src/evaluator.rs:200-208`
**Confidence**: 80%
- Problem: The double-fault handling in `invoke_function` (render error + scope pop error) is a new behavior. The logic explicitly prioritizes the render error over the pop error. While the pop error is described as a "compiler bug" unlikely in practice, the behavior is explicitly documented in the match arms. The analogous pattern in `evaluate_for` (line 299-307) also lacks a direct test. Without a test, a refactor could accidentally swap the priority (returning the pop error instead of the render error) without detection. However, since the pop error requires a truly broken scope invariant to trigger, this is difficult to test at the integration level without internal test hooks.
- Fix: Consider adding a unit test within `src/evaluator.rs` (behind `#[cfg(test)]`) that constructs a scenario where `scope.pop()` can fail after a render error. Alternatively, document this as a defense-in-depth pattern that is tested indirectly by the scope invariant tests.

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

### MEDIUM

**3,100+ line monolithic test file** - `tests/integration.rs`
**Confidence**: 85%
- Problem: All integration tests live in a single file that now exceeds 3,100 lines. This is a maintenance burden. Test discovery, IDE navigation, and focused test runs all suffer. The file mixes CLI binary tests (using `mds_bin()`) with library API tests (using `mds::compile`, `mds::check_str_collecting_warnings`, etc.), making the grouping unclear.
- Fix: Consider splitting into separate test modules by feature area (e.g., `tests/cli.rs`, `tests/compile.rs`, `tests/imports.rs`, `tests/security_limits.rs`).

## Suggestions (Lower Confidence)

- **No test for `to_namespace()` prompt_body visibility through alias import** - `src/resolver.rs:508-519` (Confidence: 70%) -- The PR description mentions "to_namespace() prompt_body exposure" as a fix. The existing test `include_respects_export_visibility_for_prompt` covers the `@include` path. However, there is no test that verifies `to_namespace()` correctly excludes `prompt_body` from the namespace scope when "prompt" is not in explicit exports and then accessed via qualified call (e.g., `{p.prompt}`). The existing test only checks `@include`, which uses a different code path.

- **resolver `pop()` vs `shift_remove()` change lacks regression test** - `src/resolver.rs:201-204` (Confidence: 65%) -- The change from `shift_remove(&canonical)` to `pop()` assumes strict LIFO ordering of `resolving` entries. While a `debug_assert` guards this, the change is only protected in debug builds. A test that exercises the LIFO property (e.g., diamond import pattern: A imports B and C, both import D) would validate that the pop-based approach works correctly for all import topologies.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 2 | 2 | - |
| Should Fix | - | - | - | - |
| Pre-existing | - | - | 1 | - |

**Testing Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The PR adds 3 well-structured integration tests (exit code 3, `check_collecting_warnings`, `check_str_collecting_warnings`) that follow existing patterns and cover new public API surface. However, two new security-critical code paths introduced in this PR -- the `mds.json output_dir` path traversal guard and the config size limit -- lack test coverage. These are HIGH-severity gaps because every other resource limit and path traversal guard in the codebase has a corresponding test, and leaving these untested breaks an otherwise consistent security-test pattern.

# Testing Review Report

**Branch**: feat/workspace-split -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**api_surface.rs tests verify existence, not behavior** - `crates/mds-core/tests/api_surface.rs:6-20`
**Confidence**: 82%
- Problem: The `public_functions_exist` test discards all return values with `let _ = ...` and never asserts on success/failure. While the stated intent is a compile-time visibility regression guard, the `#[test]` attribute still runs the function bodies at runtime. Any panic would surface as a test failure, but the test never validates that valid inputs succeed (`is_ok()`) or that invalid inputs fail (`is_err()`). This is more of a smoke test than a behavioral test.
- Fix: This is intentionally designed as a "does it compile" guard (per the PR description and commit message). The design is acceptable for its stated purpose -- catching accidental `pub` removals. No action needed, but consider adding a comment like `// Compile-time visibility check -- results intentionally discarded` for future maintainers.

## Suggestions (Lower Confidence)

- **`#[allow(unreachable_patterns)]` in mds_error_variants_exist may hide new variants** - `crates/mds-core/tests/api_surface.rs:107` (Confidence: 72%) -- The `_ => {}` arm with `#[allow(unreachable_patterns)]` means a new `MdsError` variant added to the enum will not cause a compile error here, partially defeating the purpose of an exhaustive match as a regression guard. Removing the wildcard arm would make the compiler flag any new unhandled variant.

- **Some CLI tests spawn a binary per test without parallelism guards** - `crates/mds-cli/tests/cli_build.rs`, `cli_commands.rs`, `security.rs` (Confidence: 65%) -- Tests using `mds_bin()` spawn a subprocess for each test. Under high parallelism on CI, this could lead to resource contention (many simultaneous process spawns). The tests use `tempfile::tempdir()` for isolation which is correct, but no `#[serial]` or Cargo config limits parallel integration tests. This is likely fine for 40-50 CLI tests but worth noting if the suite grows significantly.

- **`nested_loop_total_iteration_limit` and `nested_loop_under_total_iteration_limit` are slow tests** - `crates/mds-cli/tests/security.rs:176-217` (Confidence: 62%) -- These tests construct arrays of 1000+ elements and compile them, which involves non-trivial computation. They are valid behavioral tests for resource limits, but consider adding `#[ignore]` and running them only in CI, or documenting them as intentionally slow.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Testing Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This PR demonstrates excellent testing practices:

1. **Test count preserved exactly** -- The old monolithic `tests/integration.rs` (205 tests) is split across 10 categorized files with the exact same 205 test count maintained, plus a new API surface test (8 tests) and unit tests (12 in lib.rs).

2. **Behavioral focus** -- Tests assert on observable outputs (compiled markdown content, error messages, exit codes) rather than implementation details. No mocking of internals.

3. **Clear categorization** -- Files are named by domain (language, imports, objects, security, frontmatter, warnings, cli_build, cli_commands, errors) making it trivial to find relevant tests.

4. **Shared helpers are minimal** -- `common/mod.rs` is only 14 lines with two helpers (`fixture()` and `mds_bin()`), keeping setup overhead near zero per test.

5. **Good assertion messages** -- Most assertions include `got: {result}` or `got: {err}` in their failure messages, making debugging easy.

6. **Security tests are comprehensive** -- Tests cover file size limits, path traversal, symlink rejection, import depth, iteration limits, nesting depth, and config size limits.

7. **Edge cases tested** -- Empty arrays, CRLF line endings, zero-parameter functions, empty function bodies, boundary values (32 vs 33 dot segments), and mutual recursion.

8. **Test isolation** -- Each test uses `tempfile::tempdir()` for filesystem operations, preventing cross-test contamination.

9. **No flaky patterns** -- No timing-dependent assertions, no shared mutable state, no test ordering dependencies.

The only minor concern is the `api_surface.rs` test intentionally using `let _ =` which is a valid design choice for compile-time regression guards, not a behavioral test gap.

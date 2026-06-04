# Testing Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13

## Test Results

All 213 tests pass (0 failures, 0 skipped):
- **56 unit tests** across 8 source modules (lexer: 6, parser: 24, evaluator: 8, scope: 2, value: 5, validator: 2, lib: 5, main: 4)
- **144 integration tests** in `tests/integration.rs`
- **13 doc tests** in `src/lib.rs` covering all public API functions

## Coverage Analysis

### Spec Features -- Tested

| Spec Feature | Covered | Test Count | Notes |
|---|---|---|---|
| Variables (YAML frontmatter) | Yes | 5+ | simple_variable_interpolation, runtime_vars_override, type_key_available_in_mds_files, compile_str_simple, etc. |
| Interpolation | Yes | 5+ | Variable, function call, qualified call, nested calls, variable-as-arg |
| Conditionals (@if/@else) | Yes | 10+ | truthy, falsy, nested (2-deep, 3-deep), all falsy values (zero, null, empty string, empty array, boolean false) |
| Loops (@for) | Yes | 10+ | basic, nested, empty array, single element, var shadow, null iterable, non-array error, iteration limit, total iteration limit |
| Functions (@define) | Yes | 12+ | basic, multi-param, zero-param, empty body, fn-calls-fn, nested calls, param shadow, recursion detection, mutual recursion, lexical scope |
| Imports (alias) | Yes | 5+ | alias import, absolute path rejected, no unqualified access, file not found spans |
| Imports (merge) | Yes | 3+ | merge import, name collision, does not leak vars |
| Imports (selective) | Yes | 4+ | selective import, nonexistent symbol, non-exported name, prompt body |
| Exports (@export) | Yes | 6+ | named, re-export, wildcard, phantom export, export-from no local scope, explicit export hides non-exported |
| Includes (@include) | Yes | 4+ | basic include, empty body no crash, empty body warning, without import errors |
| Code block passthrough | Yes | 2 | code_block_passthrough, unicode_content (verifies no interpolation in code blocks) |
| Escaped braces | Yes | 8+ | basic escape, multiple, in function body, in blocks, symmetric \}, open+close together |
| Error: undefined variable | Yes | 4+ | basic, in for body, dot notation, error message format |
| Error: circular import | Yes | 2 | circular_import_error, circular_import_error_has_help_text |
| Error: arity mismatch | Yes | 1 | arity_mismatch_error |
| Error: file not found | Yes | 5+ | basic, import spans (alias, merge, selective), nonexistent file |
| Error: recursion | Yes | 2 | recursion_detected, mutual_recursion_detected |
| Security: file size limit | Yes | 2 | file, stdin |
| Security: path traversal | Yes | 1 | path_traversal_import_rejected |
| Security: import depth limit | Yes | 1 | import_depth_limit |
| Security: symlink rejection | Yes | 1 | symlink_import_rejected (unix-only) |
| Security: YAML/JSON depth limit | Yes | 2 | yaml_value_depth_limit, json_value_depth_limit |
| CLI: build, check, init | Yes | 15+ | build to file/stdin/stdout, check valid/invalid, init create/overwrite/force, auto-detect, quiet mode |
| CLI: --set flag | Yes | 6 | string, boolean true/false, numeric, null, empty array, duplicate key |
| CLI: --vars file | Yes | 2 | vars_file_loading, build_with_vars_file |
| Default-public exports | Yes | 1 | default_public_when_no_exports |
| CRLF handling | Yes | 1 | crlf_line_endings |
| Unicode content | Yes | 1 | unicode_content |
| .md with type:mds | Yes | 2 | md_file_with_type_mds_compiles, frontmatter_type_only_compiles |
| YAML map type rejected | Yes | 1 | yaml_map_type_rejected |

### Spec Features -- Coverage Gaps

No major spec features are missing test coverage. All features listed in the spec requirements (variables, interpolation, conditionals, loops, functions, imports, exports, includes, code block passthrough, escaped braces, and all error cases) have dedicated tests.

## Issues in Your Changes (BLOCKING)

### MEDIUM

**No unit tests for error.rs module** - `src/error.rs`
**Confidence**: 82%
- Problem: The `error.rs` module (393 lines) has zero unit tests. It contains 16+ constructor methods (syntax, syntax_at, undefined_var, undefined_var_at, arity, arity_at, type_error, type_error_at, etc.) and a helper function `at()` that builds source span pairs. While errors are thoroughly exercised by integration tests, the constructors' span-attachment logic is only tested indirectly.
- Fix: Consider adding unit tests for the `at()` helper and the `_at` constructors to verify correct span calculation, especially edge cases like zero-length spans or offsets at source boundaries.

**No unit tests for resolver.rs module** - `src/resolver.rs`
**Confidence**: 83%
- Problem: The `resolver.rs` module (578 lines) handles module resolution, caching, cycle detection, path traversal prevention, and symlink rejection -- all critical compiler infrastructure -- yet has zero unit tests. All testing is via integration tests. Functions like `validate_import_path`, `validate_file_type`, `build_cycle_string`, `path_display_name`, and `parse_frontmatter` are pure functions that would benefit from focused unit tests.
- Fix: Add unit tests for `validate_import_path` (null bytes, absolute paths, relative paths), `build_cycle_string` (various cycle shapes), and `parse_frontmatter` (edge cases like empty YAML, YAML with unsupported types).

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Weak assertions in some error tests** (6 occurrences) - Confidence: 84%
- `tests/integration.rs:131` (undefined_variable_error)
- `tests/integration.rs:138` (arity_mismatch_error)
- `tests/integration.rs:165-167` (file_not_found_error)
- `tests/integration.rs:233` (absolute_import_path_rejected)
- `tests/integration.rs:1539` (include_without_import_errors)
- `tests/integration.rs:1086-1089` (alias_import_no_unqualified_access)
- Problem: These tests check that a result `is_err()` but use overly broad or disjunctive `assert!` checks on the error message (e.g., `err.contains("arity") || err.contains("expected 1")`). The `file_not_found_error` test at line 165-167 only asserts `result.is_err()` with no message verification at all. This means if the error type changes (e.g., an IO error instead of a FileNotFound error), the test would still pass.
- Fix: Assert on the specific error variant or diagnostic code when possible. For example, `file_not_found_error` should at minimum verify the message mentions the file path or contains "not found".

**Integration test `file_not_found_error` has no error message assertion** - `tests/integration.rs:164-167`
**Confidence**: 90%
- Problem: This test only checks `result.is_err()` without verifying the error message content, unlike every other error test in the file. If `compile(PathBuf::from("nonexistent.mds"), None)` returned an error for a different reason (e.g., "not an MDS file"), the test would still pass.
- Fix: Add message assertion:
  ```rust
  let err = format!("{}", result.unwrap_err());
  assert!(
      err.contains("not found") || err.contains("nonexistent"),
      "expected file not found error, got: {err}"
  );
  ```

### LOW

**Inconsistent test naming pattern** (minor)
**Confidence**: 80%
- Problem: Test names mix several conventions: some use snake_case descriptions of behavior (`simple_variable_interpolation`), some describe the expected outcome (`recursion_detected`), and some include the feature area (`if_falsy_zero`). While individually clear, there is no unifying naming convention like `test_<feature>_<scenario>_<expected>`.
- Fix: Not blocking. Consider adopting a consistent naming pattern if the test count continues to grow. The current names are descriptive enough to understand test intent.

## Pre-existing Issues (Not Blocking)

(none -- this is an entirely new codebase on a feature branch)

## Suggestions (Lower Confidence)

- **Property-based testing for lexer/parser** - `src/lexer.rs`, `src/parser.rs` (Confidence: 65%) -- The lexer and parser handle complex string manipulation (escape sequences, brace matching, argument splitting with nested parens). Property-based tests with `proptest` or `quickcheck` would catch edge cases that hand-written tests miss, e.g., `tokenize(s).and_then(parse)` should never panic for any valid UTF-8 input.

- **Missing test for compile_collecting_warnings API** - `src/lib.rs:229` (Confidence: 70%) -- The `compile_collecting_warnings` and `compile_str_collecting_warnings` public API functions are tested indirectly via doc tests, but no integration test verifies that warnings are correctly collected (e.g., that an `@include` of a functions-only module produces a warning in the returned Vec).

- **No test for `--set` with array syntax `[a,b,c]`** - CLI (Confidence: 62%) -- Tests cover `--set items=[]` (empty array) but not `--set items=[a,b,c]` (populated array via CLI). If the CLI supports this syntax, it should be tested; if not, the empty-array test is sufficient.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

This is a strong test suite for a new compiler. The coverage is comprehensive: 213 tests covering all spec-required features, all error cases, security limits, CLI integration, edge cases (CRLF, unicode, empty values, boundary values), and scoping rules. Tests follow the Arrange-Act-Assert pattern, verify behavior rather than implementation details, and use real fixtures rather than excessive mocking.

The conditions for full approval are minor:
1. The `file_not_found_error` integration test should verify the error message (not just `is_err()`).
2. The `error.rs` and `resolver.rs` modules would benefit from targeted unit tests for their pure-function helpers, but this is not blocking since integration tests exercise these code paths.

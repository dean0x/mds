# Testing Review Report

**Branch**: main (d0624a2...HEAD)
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### HIGH

**No test for MAX_OUTPUT_SIZE resource limit** - `src/evaluator.rs:84`
**Confidence**: 95%
- Problem: The evaluator enforces a 50 MB output size limit (`MAX_OUTPUT_SIZE`) at line 84, but no test verifies this boundary. A regression could silently remove this DoS protection.
- Fix: Add an integration test that generates output exceeding 50 MB and asserts that `MdsError::ResourceLimit` is returned.

**No test for MAX_CALL_DEPTH resource limit** - `src/evaluator.rs:175`
**Confidence**: 95%
- Problem: The evaluator guards against call depth exceeding 128 (`MAX_CALL_DEPTH`) at line 175, but no test exercises this path. The `recursion_detected` test only covers direct self-recursion, not deeply chained non-recursive calls that exceed the depth limit.
- Fix: Add a test that constructs a chain of 129+ distinct functions calling each other (not recursive, but exceeding depth) and asserts the resource limit error fires.

**No test for MAX_WARNINGS cap** - `src/evaluator.rs:22`
**Confidence**: 90%
- Problem: `MAX_WARNINGS` (1,000) is defined but no test verifies that warnings beyond this limit are silently dropped. If the cap logic is removed, thousands of warnings could cause unbounded memory growth.
- Fix: Create a scenario with 1,001+ warnings (e.g., many `@include` of empty modules) and verify the warning vector is capped at 1,000.

### MEDIUM

**Weak assertion pattern: 205 `contains()` checks with no exact output verification** - `tests/integration.rs` (throughout)
**Confidence**: 85%
- Problem: Out of 338 assertions, 205 use `.contains()` for substring matching and only 6 use `assert_eq!` for exact equality. This means tests pass even if the compiler produces extra, incorrect, or garbled output as long as the expected substring appears somewhere. For a template compiler, exact output correctness is the primary contract.
- Fix: For at least the core happy-path tests (`simple_variable_interpolation`, `conditional_truthy`, `loop_over_array`, `function_definition_and_call`, `compile_str_simple`), replace `assert!(result.contains(...))` with `assert_eq!(result.trim(), "expected exact output")`. This locks in the output contract without making every test brittle.

**resolver.rs has 0 unit tests (740 lines)** - `src/resolver.rs`
**Confidence**: 85%
- Problem: The resolver is the most complex module (740 lines) handling imports, caching, cycle detection, path traversal prevention, symlink detection, and export visibility. It has zero unit tests. All its coverage comes from integration tests, which do not isolate individual functions like `canonicalize_and_check`, `validate_import_path`, `build_cycle_string`, `find_project_root`, or `validate_exports`. A subtle regression in any of these internal functions may not be caught because integration tests only exercise specific composite paths.
- Fix: Add unit tests for at least: `validate_import_path` (null bytes, absolute paths, valid relative paths), `build_cycle_string` (with 2-node and 3-node cycles), `find_project_root` (with and without `.git` marker), and `validate_exports` (phantom export, valid export, "prompt" special case).

**No test for `reject_directory_input`** - `src/main.rs:403`
**Confidence**: 82%
- Problem: The CLI rejects directory paths passed as input (line 403-412), but no integration test covers this. A user who accidentally runs `mds build .` should get a clear error.
- Fix: Add a CLI integration test that passes a directory path to `mds build` and asserts non-zero exit and descriptive error on stderr.

**No test for `init` path traversal guard** - `src/main.rs:547-553`
**Confidence**: 82%
- Problem: `mds init` rejects filenames containing `..` components (line 547-553), but no test exercises this security check.
- Fix: Add `let output = mds_bin().args(["init", "../escape.mds"]).output().unwrap(); assert!(!output.status.success());`

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Orphaned test fixture `test_export_from_scope.mds` never exercised** - `tests/fixtures/test_export_from_scope.mds`
**Confidence**: 90%
- Problem: This fixture tests `@export from` scope semantics (re-exporting makes a symbol available to call locally) but is not referenced by any integration test. The behavior it represents -- whether `@export from` brings a symbol into local scope -- is tested by `export_from_no_local_scope`, but the fixture itself is dead code that may confuse contributors.
- Fix: Either write an integration test that compiles `test_export_from_scope.mds` and asserts the expected behavior, or delete the orphaned fixture.

**Orphaned test fixture `formatting.mds` not directly tested** - `tests/fixtures/formatting.mds`
**Confidence**: 80%
- Problem: The `formatting.mds` fixture defines `bullet_list` and `numbered_list` utility functions with exports, but is only transitively referenced via `index.mds` (barrel file). There is no dedicated test for the formatting module's output correctness.
- Fix: Add a test that imports and calls `bullet_list` and `numbered_list` to verify their output.

### LOW

**validator.rs has only 2 unit tests for 190 lines** - `src/validator.rs`
**Confidence**: 80%
- Problem: The validator checks variable references, function arity, type constraints, `@include` scope, and nested argument validation. Only 2 unit tests exist (both for `@define` body validation). The `validate_var_args` depth guard (line 154) and `@include` namespace validation (line 77-82) have no direct unit test.
- Fix: Add unit tests for: (1) `validate_var_args` with depth > 256, (2) `@include` with undefined alias, (3) `@if` with undefined condition variable, (4) qualified call arity mismatch in validator.

## Pre-existing Issues (Not Blocking)

_No pre-existing issues (all code is new)._

## Suggestions (Lower Confidence)

- **Property-based testing for lexer round-trip** - `src/lexer.rs` (Confidence: 65%) -- The lexer handles escaped braces, code blocks, frontmatter, and interpolation. Property-based testing (e.g., with `proptest`) could generate random MDS source strings and verify the tokenizer never panics and round-trips correctly.

- **No concurrency/parallelism test for ModuleCache** - `src/resolver.rs:50` (Confidence: 60%) -- `ModuleCache` is not `Send`/`Sync` and is used single-threaded, but if future versions add parallel compilation, the lack of thread-safety tests could mask race conditions. Not currently needed, but worth noting.

- **ast.rs has 0 tests** - `src/ast.rs` (Confidence: 70%) -- The AST module is 133 lines of pure data structures with no logic beyond derives, so the lack of tests is acceptable, but any future methods added would need test coverage.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 3 | 4 | 0 |
| Should Fix | 0 | 0 | 2 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The MDS compiler has a strong test suite for a v0.1 project: 286 tests (87 unit + 171 integration + 13 CLI + 15 doc-tests) all pass with zero failures. Test organization follows the Rust convention (unit tests in `#[cfg(test)]` modules, integration tests in `tests/`), and there is excellent coverage of error paths (54 `is_err()` assertions), security boundaries (symlinks, path traversal, file size limits, loop iteration limits), and the full CLI surface area (build, check, init with various flag combinations).

However, three resource-limit guards (MAX_OUTPUT_SIZE, MAX_CALL_DEPTH, MAX_WARNINGS) lack any test coverage, which is the most significant gap. Additionally, the resolver module (the most complex component at 740 lines) has zero unit tests, relying entirely on integration tests for coverage. The overwhelming use of substring matching (`contains()`) over exact output assertions is a concern for a template compiler where output correctness is the core contract.

**Conditions for approval:**
1. Add tests for at least MAX_OUTPUT_SIZE and MAX_CALL_DEPTH resource limits (HIGH severity)
2. Add at least one exact-output `assert_eq!` test for a core compilation path

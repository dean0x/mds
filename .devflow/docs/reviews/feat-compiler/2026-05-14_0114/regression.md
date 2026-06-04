# Regression Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**Context**: New project -- entire MDS compiler added in this PR. No prior code to regress against. Review focuses on public API surface stability, CLI contract, test fixture expectations, and whether the test suite adequately guards against future regressions.

## Issues in Your Changes (BLOCKING)

### HIGH

**Integration tests rely almost entirely on `contains` assertions, not exact output matching** - `tests/integration.rs` (63 occurrences)
**Confidence**: 85%
- Problem: Of ~66 assertions in the integration test suite, 63 use `assert!(result.contains(...))` and only 3 use `assert_eq!`. This means the test suite will silently pass even if the compiler emits extra unexpected output, reorders content, introduces whitespace drift, or duplicates sections. For a compiler where output fidelity is critical (LLM prompt templates), this creates a blind spot for output regressions.
- Impact: A future change could alter output formatting, add stray whitespace, or duplicate content blocks, and none of the integration tests would catch it. For a prompt compiler, even minor whitespace changes can alter LLM behavior.
- Fix: Add exact output matching tests for at least the core compilation paths. For each major feature (interpolation, loops, conditionals, functions, imports), add at least one test that asserts the complete output string:
  ```rust
  #[test]
  fn simple_variable_interpolation_exact() {
      let result = mds::compile(fixture("simple.mds"), None).unwrap();
      assert_eq!(result, "Hello Alice!\nYou have 3 items.\n");
  }
  ```
  This provides a regression tripwire that catches any output drift.

### MEDIUM

**Analysis report file committed to repository** - `autobeat-orchestrator-analysis.md`
**Confidence**: 90%
- Problem: The file `autobeat-orchestrator-analysis.md` is a development tool analysis report committed at the repository root. It is not `.gitignore`d, and contains internal tooling metadata (orchestration IDs, loop IDs, autobeat version). This file will become stale immediately and is not part of the compiler deliverable.
- Impact: This file will persist as dead weight in the repo. If a future contributor updates the file or it drifts from reality, it creates confusion about what constitutes the project's actual documentation.
- Fix: Either remove from version control and add to `.gitignore`, or move to `.docs/` (which is already gitignored):
  ```bash
  git rm autobeat-orchestrator-analysis.md
  echo "autobeat-orchestrator-analysis.md" >> .gitignore
  ```

**CLI exit code only distinguishes success (0) vs failure (1)** - `src/main.rs:165`
**Confidence**: 82%
- Problem: All error conditions produce `process::exit(1)`. There is no differentiation between user errors (bad input, missing file), compilation errors (syntax, undefined vars), and internal errors (I/O failures). Scripts and CI pipelines that invoke `mds build` cannot distinguish between "file not found" and "syntax error" without parsing stderr.
- Impact: Future tooling integration will need to parse error messages instead of checking exit codes. This makes the CLI contract fragile -- any rewording of error messages could break downstream consumers.
- Fix: Consider establishing exit code conventions now (while the API surface is new) before consumers depend on the current behavior:
  ```rust
  // Convention:
  // 0 = success
  // 1 = compilation error (syntax, undefined var, type error)
  // 2 = I/O error (file not found, permission denied)
  // 3 = usage error (bad arguments)
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`clean_output` strips `\r` unconditionally without test coverage for mixed line endings** - `src/lib.rs:290-291`
**Confidence**: 80%
- Problem: The `clean_output` function silently strips all `\r` characters. While CRLF normalization is sensible, this means standalone `\r` (bare carriage returns, used in some legacy systems) are also silently eaten. The only CRLF test (`crlf_line_endings` at `tests/integration.rs:1429`) tests that CRLF input compiles correctly, but does not verify the exact output line-ending style.
- Impact: If a user's template content intentionally contains `\r` characters (e.g., generating legacy protocol content), they will be silently dropped. Future attempts to add `\r` preservation would be regression-risky without exact output tests.
- Fix: Add a test that verifies the exact line ending behavior:
  ```rust
  #[test]
  fn clean_output_normalizes_crlf_to_lf() {
      assert_eq!(clean_output("hello\r\nworld\r\n"), "hello\nworld\n");
  }
  ```

**`Value::from_yaml` rejects Map types with version-specific message** - `src/value.rs:61-63`
**Confidence**: 80%
- Problem: The error message "object/map types are not supported in MDS v0.1" embeds a version number. When MDS reaches v0.2+, this message will be misleading unless updated. Both `from_yaml` and `from_json` share this pattern. The version string creates a maintenance obligation.
- Impact: Future version bumps will leave stale version references in error messages if developers do not grep for "v0.1" across all source files.
- Fix: Remove the version reference from the error message:
  ```rust
  serde_yaml::Value::Mapping(_) => Err(MdsError::YamlError {
      message: "object/map types are not supported".to_string(),
  }),
  ```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`compile_collecting_warnings` discards empty prompt body without warning** - `src/lib.rs:244-248`
**Confidence**: 80%
- Problem: When `prompt_body` is `None` (module has no body text, only functions/exports), `compile_collecting_warnings` silently returns an empty string. This is correct for library modules, but when a user accidentally creates a functions-only file and passes it to `mds build`, they get empty output with no warning. The `check` command would also report success for such files.
- Impact: Users may be confused by silent empty output. However, this is intentional behavior for library modules, so the current approach is defensible.

## Suggestions (Lower Confidence)

- **Scope `pop` panics on underflow are not tested** - `src/scope.rs:75-83` (Confidence: 70%) -- The `pop()` method returns `Err` if called when only the global frame remains, but no integration test exercises this error path. A bug in the evaluator that imbalances push/pop could go undetected.

- **`compile_str` does not support `@import` resolution from cwd** - `src/lib.rs:98-100` (Confidence: 65%) -- `compile_str` delegates to `compile_str_with(source, None, None)`, which defaults `base_dir` to `std::env::current_dir()`. This means the same source string can produce different results depending on the working directory. This is documented behavior but could surprise library consumers.

- **`parse_cli_value` bracket-list parsing does not handle nested brackets** - `src/main.rs:144-154` (Confidence: 72%) -- The `[a, b, c]` parsing splits on `,` without bracket depth tracking. Input like `--set items=[a, [b, c]]` would produce unexpected results. However, the comment explicitly notes "does not recurse" and nested arrays are not a v0.1 feature.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Regression Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The compiler is a new project with no prior code to regress against. The public API surface (`lib.rs`) is well-designed with `#[must_use]` annotations, consistent `Result` returns, and comprehensive doc examples that all pass `cargo test --doc`. The CLI contract (3 subcommands: `build`, `check`, `init`) is clean with proper exit-code behavior on errors.

The primary regression concern is forward-looking: the test suite provides excellent breadth (213 tests, 100+ test fixtures) but uses almost exclusively `contains`-based assertions. This means the test suite is strong at detecting broken features but weak at detecting output drift -- exactly the kind of subtle regression that matters most for a prompt compiler. Adding a handful of exact-output assertion tests for the critical paths would significantly strengthen the regression safety net for future development.

All 213 tests pass. All 13 doc tests pass. No TODO/FIXME/HACK markers in the codebase. No `unwrap()` calls in production code paths. Resource limits are bounded. The codebase is in solid shape for merge with the noted conditions.

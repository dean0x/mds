# Testing Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**Evaluator unit test for undefined variable uses `is_err()` without asserting error content** - `src/evaluator.rs:419`
**Confidence**: 85%
- Problem: The `evaluate_undefined_var_error` test only checks `is_err()` but never inspects the error variant or message. A regression could cause the function to return a different error type (e.g., a panic-turned-Err, a wrong variant) and this test would still pass. This pattern contradicts behavior-focused testing: the test verifies "it fails" but not "it fails for the right reason."
- Fix: Assert on the error content to confirm the correct error variant is returned:
```rust
#[test]
fn evaluate_undefined_var_error() {
    let nodes = vec![Node::Interpolation(Interpolation {
        expr: Expr::Var("unknown".to_string()),
        offset: 0,
        len: 7,
    })];
    let mut scope = Scope::new();
    let mut warnings = vec![];
    let err = evaluate(&nodes, &mut scope, &mut warnings).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("unknown") || msg.contains("undefined"),
        "error should mention the undefined variable, got: {msg}"
    );
}
```

**Resolver module (573 lines) has zero unit tests** - `src/resolver.rs`
**Confidence**: 88%
- Problem: The resolver is one of the most complex modules in the codebase -- it handles import resolution, path security validation, cycle detection, export visibility, frontmatter parsing, and namespace construction. Despite 573 lines and at least 15 functions, it has no `#[cfg(test)]` block. While many of its behaviors are tested transitively through integration tests, there are no isolated unit tests for critical internal functions such as `resolve_path`, `validate_import_path`, `validate_file_type`, `build_cycle_string`, or `parse_frontmatter`. These are pure functions that would benefit from focused unit testing to catch edge cases that integration tests may miss (e.g., unusual path characters, edge-case YAML structures).
- Fix: Add a `#[cfg(test)]` module to `resolver.rs` with unit tests for the pure utility functions. Priority targets:
  - `validate_import_path`: test with `./ok`, `../ok`, `/absolute` (reject), empty string, path with `..` components
  - `resolve_path`: test relative resolution, symlink paths, Unicode filenames
  - `parse_frontmatter`: test empty YAML, invalid YAML, nested objects (rejected), bare scalars
  - `build_cycle_string`: test with 2-file and 3-file cycles

**Error module (441 lines) has zero unit tests** - `src/error.rs`
**Confidence**: 82%
- Problem: The error module defines a rich error hierarchy with source-span-aware constructors (`_at` variants), Display/miette integration, and help text. None of the 20+ constructor functions or their formatting behavior is directly unit-tested. While integration tests verify error messages at the boundary, there is no test that the `_at` constructors correctly propagate source location, that help text is attached, or that miette rendering produces the expected format. A refactoring mistake in error construction could silently degrade diagnostic quality without any test catching it.
- Fix: Add a `#[cfg(test)]` module to `error.rs` with tests for:
  - Each error variant's Display output contains the expected keywords
  - The `_at` variants correctly populate source span and named source
  - Help text is present on errors that define it (e.g., circular import)

### MEDIUM

**Scope unit tests do not cover `pop()` on empty stack (error path)** - `src/scope.rs:157-188`
**Confidence**: 85%
- Problem: The scope module has only 2 unit tests. There is no test for `pop()` on an empty scope (which should return an error), no test for namespace operations (`set_namespace`/`get_namespace`), and no test for the `collect_all` helper. The `pop()` error path is particularly important since an unhandled scope underflow could corrupt evaluation state.
- Fix: Add tests for:
```rust
#[test]
fn scope_pop_empty_errors() {
    let mut scope = Scope::new();
    // Initial frame is present; pop it
    assert!(scope.pop().is_ok());
    // Now stack is empty; pop should fail
    // (or if Scope::new() creates exactly one frame, pop should error when it's the last)
}

#[test]
fn scope_namespace_operations() {
    let mut scope = Scope::new();
    let ns = NamespaceScope { functions: HashMap::new(), prompt: None };
    scope.set_namespace("utils", ns.clone());
    assert!(scope.get_namespace("utils").is_some());
    assert!(scope.get_namespace("unknown").is_none());
}
```

**Integration tests rely heavily on `assert!(result.contains(...))` -- weak substring assertions** (144 occurrences across `tests/integration.rs`)
**Confidence**: 80%
- Problem: Most integration tests use `assert!(result.contains("..."))` for output verification. While this is reasonable for a template compiler (where exact whitespace may vary), some tests only check for a single word, making them fragile in the false-positive direction. For example, `unicode_content` checks `assert!(result.contains("Hello"))` which would pass even if Unicode handling was completely broken, as long as the ASCII word "Hello" appeared anywhere. Similarly, `check_valid_file` only asserts `result.is_ok()` with no output verification.
- Fix: For critical tests, consider using `assert_eq!` on the trimmed output or at minimum checking for multiple distinguishing substrings. The existing pattern of combining positive and negative assertions (e.g., `assert!(result.contains("X")); assert!(!result.contains("Y"))`) is good and should be applied more consistently.

**Validator has only 2 unit tests for a module with complex recursive logic** - `src/validator.rs:191-238`
**Confidence**: 83%
- Problem: The validator performs recursive AST traversal with scope manipulation to catch errors at compile-time rather than runtime. It handles `@if`, `@for`, `@define`, interpolation, function calls (both simple and qualified), and nested blocks. Only two scenarios are tested: undefined var in define body (fails) and param reference in define body (passes). Missing coverage includes: `@for` body validation with loop variable in scope, nested `@if`/`@for` inside `@define`, qualified function calls in validation, and the recursive descent through `validate_nodes` and `validate_expr`.
- Fix: Add unit tests for:
  - `@for` body correctly adds loop variable to validation scope
  - Nested blocks inside `@define` (e.g., `@if` inside function body)
  - Qualified call expression validation (`Expr::QualifiedCall`)
  - `@if` condition referencing undefined variable

## Issues in Code You Touched (Should Fix)

### MEDIUM

**No negative test for `EscapedCloseBrace` in evaluator unit tests** - `src/evaluator.rs:517-531`
**Confidence**: 80%
- Problem: The evaluator unit tests cover `Node::EscapedBrace` (open brace) but the recently-added `EscapedCloseBrace` node type has no evaluator unit test. Integration tests cover this at the end-to-end level (`escaped_close_brace_produces_literal_brace`), but a unit-level test would confirm the evaluator handles this node type correctly in isolation.
- Fix: Add a test similar to `evaluate_escaped_brace` but for `Node::EscapedCloseBrace`:
```rust
#[test]
fn evaluate_escaped_close_brace() {
    let nodes = vec![text("Use "), Node::EscapedCloseBrace, text(" to close")];
    let mut scope = Scope::new();
    let mut warnings = vec![];
    assert_eq!(
        evaluate(&nodes, &mut scope, &mut warnings).unwrap(),
        "Use } to close"
    );
}
```

## Pre-existing Issues (Not Blocking)

_No pre-existing issues detected (this is a greenfield project on `feat/compiler`)._

## Suggestions (Lower Confidence)

- **No property-based tests for the lexer/parser** - `src/lexer.rs`, `src/parser.rs` (Confidence: 65%) -- The lexer and parser handle arbitrary user input including edge cases around Unicode, nesting, escaping, and boundary conditions. Property-based testing (e.g., with `proptest` or `quickcheck`) would systematically explore these inputs. Consider adding roundtrip properties like "tokenize then parse never panics" and "parse(tokenize(s)) produces valid AST for well-formed inputs."

- **Integration test file is a single 2320-line file** - `tests/integration.rs` (Confidence: 70%) -- As the test count grows (currently 144), navigation and maintenance become harder. Consider splitting into multiple integration test files organized by feature area (e.g., `tests/imports.rs`, `tests/cli.rs`, `tests/security.rs`, `tests/directives.rs`).

- **No doc-test on `Value::from_yaml` or `Value::from_json`** - `src/value.rs` (Confidence: 60%) -- These public APIs have unit tests but no doc-tests demonstrating usage, which would improve API documentation quality.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 3 | 3 | - |
| Should Fix | - | - | 1 | - |
| Pre-existing | - | - | - | - |

**Testing Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The test suite is strong overall: 213 tests (56 unit + 144 integration + 13 doc-tests) with comprehensive behavioral coverage across the compiler pipeline. The integration tests are well-structured with clear Arrange-Act-Assert patterns, good error message assertions (most error-path tests verify both `is_err()` AND the error message content), and thorough coverage of edge cases including security boundaries (file size limits, symlink rejection, path traversal, iteration limits, depth limits).

The conditions for approval relate to the uneven distribution of unit tests: the resolver (573 lines, the most complex module) and error module (441 lines) have zero unit tests, while the scope module has only 2 and the validator only 2. The integration tests compensate significantly, but these modules would benefit from targeted unit tests to catch regressions during future refactoring.

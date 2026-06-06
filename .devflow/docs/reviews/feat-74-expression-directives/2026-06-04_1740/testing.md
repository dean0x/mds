# Testing Review Report

**Branch**: feat/74-expression-directives -> main
**Date**: 2026-06-04

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Nested call argument structure not verified in parse test** - `crates/mds-core/src/parser_tests.rs:1206`
**Confidence**: 82%
- Problem: `parse_for_nested_call_iterable` asserts the outer call is `sort` but does not verify the inner argument is `Expr::Call { name: "unique", .. }` with a `tags` argument. The test would pass even if the parser silently dropped or mangled the inner call structure. The evaluation test (`evaluate_for_sort_unique_iterable`) covers the end-to-end behavior, but the parser-level test should verify the AST shape it claims to test.
- Fix: After matching the outer `Expr::Call`, destructure `args` and assert the inner argument is a nested call to `unique`:
```rust
if let Expr::Call { name, args } = &block.iterable {
    assert_eq!(name, "sort");
    assert_eq!(args.len(), 1);
    assert!(
        matches!(&args[0], Arg::NestedCall(inner_name, inner_args)
            if inner_name == "unique" && inner_args.len() == 1),
        "expected nested unique(tags) call, got {:?}",
        args[0]
    );
} else {
    panic!("expected Expr::Call");
}
```

**Sort order not explicitly asserted in evaluate_for_sort_unique_iterable** - `crates/mds-core/src/parser_tests.rs:1487`
**Confidence**: 80%
- Problem: The test checks that both `- a` and `- b` are present and that there are exactly 2 items, but does not assert that `a` appears before `b`. Since the test name says "sort", it should verify the ordering guarantee. A broken `sort()` that returned `[b, a]` would still pass this test.
- Fix: Add an ordering assertion:
```rust
let dashes: Vec<_> = result.lines().filter(|l| l.starts_with("- ")).collect();
assert_eq!(dashes.len(), 2);
assert_eq!(dashes[0], "- a", "first sorted item should be 'a'");
assert_eq!(dashes[1], "- b", "second sorted item should be 'b'");
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**evaluate_if_undefined_function_is_error does not verify error message content** - `crates/mds-core/src/parser_tests.rs:1515`
**Confidence**: 83%
- Problem: The test asserts `result.is_err()` but does not check the error message. An unrelated panic or wrong error type would pass. Other error-path tests in this PR (e.g., `parse_if_bare_literal_rejected`, `split_resource_limit_too_many_elements`) follow the pattern of asserting on error message content. This test is inconsistent.
- Fix: Add an error message assertion:
```rust
let err = result.unwrap_err().to_string();
assert!(
    err.contains("notabuiltin") || err.contains("undefined") || err.contains("unknown"),
    "error should mention the undefined function, got: {err}"
);
```

**evaluate_for_non_array_result_is_error does not verify error message content** - `crates/mds-core/src/parser_tests.rs:1506`
**Confidence**: 83%
- Problem: Same pattern as above -- `result.is_err()` without checking the error message. The test should verify the error mentions the type mismatch (e.g., "string" or "array").
- Fix:
```rust
let err = result.unwrap_err().to_string();
assert!(
    err.contains("array") || err.contains("string") || err.contains("iterate"),
    "error should mention expected type, got: {err}"
);
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **No direct unit tests for `strip_trailing_directive_colon`** - `crates/mds-core/src/parser_helpers.rs:33` (Confidence: 70%) -- This function handles complex edge cases (colons inside strings, inside parens, unclosed parens). It is covered indirectly through integration tests (`parse_if_colon_in_string_arg`, `parse_for_colon_as_separator`), but targeted unit tests for edge cases like `func(a:` (unclosed paren with colon), empty input, and multiple bare colons would strengthen coverage of this critical helper. applies ADR-008

- **No direct unit tests for `parse_expr_inner`** - `crates/mds-core/src/parser_helpers.rs:139` (Confidence: 65%) -- The function has 9 branches (string literal, unterminated string, booleans, null, number, simple call, qualified call, member access, simple var). All branches are exercised through the condition/iterable integration tests, but direct unit tests for each branch would catch regressions faster and provide better error localization.

- **Node-API tests use `try/catch` without error content assertion** - `examples/node-api-test.mjs:503` (Confidence: 65%) -- The `expression @if: error cases (undefined function)` test catches any error without verifying the error message. A wrong error (e.g., a parse error instead of an undefined-function error) would pass silently.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The test suite for this PR is strong overall: 31 new parser tests, 9 Node-API integration tests, 2 edge-case example files, resource-limit security tests, and backward-compatibility regression tests. The PR follows good practices -- tests cover happy paths, error paths, edge cases (colon-in-string), and backward compatibility. The conditions are minor: tighten a few assertions that currently pass but would not catch subtle regressions (wrong inner AST shape, wrong sort order, missing error message checks).

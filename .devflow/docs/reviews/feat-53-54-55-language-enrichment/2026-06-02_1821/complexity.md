# Complexity Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicated function-resolution logic in `validate_var_args` re-implements `validate_expr`'s Call arm** - `validator.rs:302-343`
**Confidence**: 90%
- Problem: The `Arg::Call` arm inside `validate_var_args` (lines 302-343) is a near-exact copy of the `Expr::Call` arm in `validate_expr` (lines 201-236). Both perform the same three-step resolution: (1) check user-defined function, (2) check builtin, (3) reject as undefined -- each with identical arity-range checks. This is 42 lines of duplicated decision logic that must be kept in sync manually. Any future change to function resolution (e.g., adding a third resolution source or changing arity error format) must be replicated in both locations, creating a maintenance burden and a high risk of drift.
- Fix: Extract a shared `validate_call_arity` helper that takes `(name, args_len, scope, file, source, offset, len)` and performs the user-defined-then-builtin-then-error resolution. Both `validate_expr::Call` and `validate_var_args::Arg::Call` call this single helper. Example:
```rust
fn validate_call_arity(
    name: &str,
    num_args: usize,
    scope: &Scope,
    file: &str,
    source: &str,
    offset: usize,
    len: usize,
) -> Result<(), MdsError> {
    if let Some(func) = scope.get_function(name) {
        let required = required_param_count(&func.params);
        let total = func.params.len();
        if num_args < required || num_args > total {
            return Err(MdsError::arity_at(name, required, total, num_args, file, source, offset, len));
        }
    } else if let Some(meta) = crate::builtins::get_builtin(name) {
        if num_args < meta.min_args || num_args > meta.max_args {
            return Err(MdsError::arity_at(name, meta.min_args, meta.max_args, num_args, file, source, offset, len));
        }
    } else {
        return Err(MdsError::undefined_fn_at(name, file, source, offset, len));
    }
    Ok(())
}
```

**`builtin_sort` has high cyclomatic complexity with 4 nesting levels** - `builtins.rs:412-470`
**Confidence**: 82%
- Problem: `builtin_sort` is 58 lines with a cyclomatic complexity around 11 (match on `arr[0]` type, then for-loop with nested match for homogeneity/finiteness checks, then sort-by with inner match, repeated for both String and Number arms). The String and Number arms share the same validate-then-clone-then-sort structure but differ in comparison logic and the extra finiteness check for numbers. The deepest nesting reaches 4 levels (fn > match > for > match). While each arm is individually understandable, the overall function requires scanning 58 lines to verify correctness.
- Fix: Extract type-specific sort helpers to reduce the match arms to delegation calls:
```rust
fn sort_strings(arr: &[Value]) -> Result<Value, MdsError> {
    for item in arr {
        if !matches!(item, Value::String(_)) {
            return Err(MdsError::builtin_error(format!(
                "sort() requires a homogeneous array; found {} mixed with string",
                item.type_name()
            )));
        }
    }
    let mut sorted = arr.to_vec();
    sorted.sort_by(|a, b| match (a, b) {
        (Value::String(a), Value::String(b)) => a.cmp(b),
        _ => unreachable!(),
    });
    Ok(Value::Array(sorted))
}
// Similar for sort_numbers, then builtin_sort becomes ~15 lines.
```

### MEDIUM

**Three quote-tracking scanner functions share the same state machine pattern** - `parser_helpers.rs:126-173`, `parser_helpers.rs:207-248`, `parser_helpers.rs:842-887`
**Confidence**: 85%
- Problem: `find_unquoted_operator`, `split_on_unquoted_op`, and `split_on_unquoted_commas` each implement their own in_string/string_char/escape tracking loop. The core scanning state machine (track open/close quotes, handle backslash escapes, skip content inside quotes) is identical across all three. A fourth function `find_unquoted_equals` (lines 892-925) also duplicates this pattern. Adding a new quote-aware scanner means copying the state machine a fifth time. This is the highest concentration of duplicated complexity in the PR.
- Fix: Consider a shared `QuoteAwareScanner` iterator or a callback-based `scan_unquoted` helper that yields (position, byte) only for bytes outside quoted strings. Each caller provides only its operator-matching logic. This would reduce each function to 5-10 lines plus the shared scanner.

**`parse_single_arg_inner` has high cyclomatic complexity with 7 distinct branches** - `parser_helpers.rs:775-836`
**Confidence**: 83%
- Problem: `parse_single_arg_inner` is 62 lines with 7 classification branches (string literal, nested call, boolean true, boolean false, null, numeric literal, member access, variable, error). The numeric literal detection (lines 805-817) has a particularly dense condition: `s.chars().next().is_some_and(|c| c.is_ascii_digit() || (c == '-' && s.len() > 1 && s[1..].starts_with(|d: char| d.is_ascii_digit())))`. This function is the single point of argument classification for the entire parser, so every new argument type adds another branch here.
- Fix: Extract the numeric literal detection into a named predicate `fn looks_like_number(s: &str) -> bool` to make the main function's branching structure clearer. The overall branch count is acceptable for a classifier function, but the inline condition makes it harder to scan.

**`parse_args_inner` has 4 nesting levels with mixed state machine and recursion** - `parser_helpers.rs:687-759`
**Confidence**: 80%
- Problem: `parse_args_inner` is 72 lines implementing a character-by-character state machine with 5 state variables (`current`, `in_string`, `string_char`, `escaped`, `paren_depth`) and 4 nesting levels at the deepest point (fn > for > if-in_string > match). The function combines two responsibilities: comma-separated tokenization (with quote/paren awareness) and recursive argument parsing via `parse_single_arg_inner`. While well-commented, the state variable count makes it difficult to reason about edge cases at a glance.
- Fix: The state machine is inherently complex for this kind of parsing. Consider extracting the tokenization (splitting on unquoted top-level commas) into a separate function that returns `Vec<&str>`, then `parse_args_inner` becomes a simple map over tokens. This is similar to the existing `split_on_unquoted_commas` but with paren-depth tracking -- another argument for a unified quote-aware scanner.

**`arity_at` has 8 parameters, exceeding recommended maximum** - `error.rs:344-363`
**Confidence**: 88%
- Problem: `MdsError::arity_at` takes 8 parameters: `name`, `expected_min`, `expected_max`, `got`, `file`, `source`, `offset`, `len`. This is annotated with `#[allow(clippy::too_many_arguments)]` which acknowledges the issue but suppresses rather than fixes it. The v0.2.0 change (splitting `expected` into `expected_min` + `expected_max`) pushed this past the threshold.
- Fix: The 4-parameter `(file, source, offset, len)` group is a location tuple used by every `_at` constructor. Consider a `SourceLocation` struct to bundle these, reducing all `_at` constructors by 3 parameters. The `at()` helper already exists and produces the result -- a struct input would be the symmetric counterpart.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`invoke_function` is 73 lines with 5 concerns interleaved** - `evaluator.rs:258-330`
**Confidence**: 80%
- Problem: `invoke_function` handles recursion detection, arity checking, scope setup (captured vars/functions/namespaces), parameter binding with defaults, call stack management, body evaluation, and LIFO invariant checking -- all in one function. At 73 lines it is above the 50-line threshold. The function is well-structured with clear sequential phases, but the interleaving of error paths (recursion guard, arity guard, LIFO guard, pop guard) with the happy path makes it the most complex function in the evaluator.
- Fix: The scope-restoration phase (lines 282-293, restoring namespaces/functions/vars) could be extracted to a `restore_captured_scope(scope, &func.captured)` helper, reducing `invoke_function` to ~55 lines and making the main flow clearer.

## Pre-existing Issues (Not Blocking)

_No pre-existing CRITICAL issues found in reviewed files._

## Suggestions (Lower Confidence)

- **`validate_for_node` mixes static type checking with scope management** - `validator.rs:89-148` (Confidence: 70%) -- The function has a long conditional block (lines 116-139) for static type checking that includes a nested sub-check for objects vs. other types. The accepted-limitation comment (6 lines) suggests this complexity is understood, but the function could benefit from extracting the type-check into a named helper.

- **`builtins.rs` file length (994 lines, 537 non-test)** - `builtins.rs` (Confidence: 65%) -- The file is at the upper boundary of the 500-line file-length guideline when including tests. The non-test code (537 lines) is within bounds, and the tests are co-located by design. As more built-ins are added, consider splitting tests into a separate `builtins_tests.rs` file (the project already uses this pattern with `error_tests.rs`).

- **`error.rs` constructor proliferation** - `error.rs:257-575` (Confidence: 62%) -- The `impl MdsError` block is 318 lines of largely repetitive constructor pairs (`variant` + `variant_at`). Each follows the same pattern. A macro could reduce this to one definition per variant, though the trade-off is macro complexity vs. boilerplate clarity. The `at()` helper already reduces each `_at` body to 3 lines, so this is more a readability concern than a correctness one.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The code is generally well-structured with clear module boundaries (applies ADR-008 -- bundling related features into one PR kept the architecture cohesive). Individual functions are mostly under the 50-line threshold, nesting rarely exceeds 3 levels, and the new builtins module uses a clean data-driven registry pattern that minimizes per-function boilerplate.

The primary complexity concerns are: (1) duplicated function-resolution logic in the validator that should be extracted, and (2) four separate quote-tracking state machines in parser_helpers that share identical scanning logic. Both are maintenance risks as the language grows. The `builtin_sort` function is the most complex individual function but is bounded and well-tested.

Conditions for approval:
- The duplicated call-resolution logic in `validate_var_args` / `validate_expr` should be consolidated before merge to prevent drift (HIGH).
- The remaining items (sort extraction, scanner unification, parameter count) are recommended but not blocking.

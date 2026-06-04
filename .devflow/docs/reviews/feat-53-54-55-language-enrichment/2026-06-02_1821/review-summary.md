# Code Review Summary

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02_1821

## Merge Recommendation: CHANGES_REQUESTED

This PR implements three tightly coupled language features (built-in functions, default arguments, logical operators) across ~2800 lines with excellent test coverage (703 tests). However, there are 5 blocking issues that must be resolved before merge:

- **1 CRITICAL blocking issue** (Performance: double linear scan in builtin dispatch)
- **1 HIGH blocking issue** (Testing: missing CLI-level end-to-end tests)
- **3 MEDIUM blocking issues** (Architecture, Complexity, Reliability)

The PR is well-structured, thoroughly tested, and has strong security and regression practices. All blocking issues are straightforward to fix. Approval is conditional on addressing these items.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 1 | 1 | 3 | 0 | 5 |
| Should Fix | 0 | 0 | 6 | 1 | 7 |
| Pre-existing | 0 | 0 | 2 | 1 | 3 |

---

## Blocking Issues (Must Fix Before Merge)

### CRITICAL

**Double linear scan in built-in dispatch** — `evaluator.rs:343-352` (Performance, 85% confidence)
- Problem: `call_function` calls `crate::builtins::get_builtin(name)` to check arity, then calls `crate::builtins::call_builtin(name, args)`, which internally calls `get_builtin(name)` again. This performs a linear scan of the 18-element BUILTINS array twice per built-in invocation. At 18 entries the wall-clock cost is negligible, but this becomes relevant when built-ins are called inside `@for` loops (up to 100,000 iterations).
- Fix: Use the meta reference from the first lookup to call the handler directly:
```rust
if let Some(meta) = crate::builtins::get_builtin(name) {
    if args.len() < meta.min_args || args.len() > meta.max_args {
        return Err(MdsError::arity(name, meta.min_args, meta.max_args, args.len()));
    }
    return (meta.handler)(args); // direct dispatch, no second lookup
}
```

### HIGH

**Missing CLI-level / end-to-end integration tests for all three new features** — `crates/mds-cli/tests/` (Testing, 85% confidence)
- Problem: The PR adds 113 new tests (all unit-level in `mod tests` or `compile_str()` integration), but the CLI test suite (`crates/mds-cli/tests/language.rs`, `errors.rs`, etc.) was not updated. The features are not tested through the actual CLI entry point — `run_build`/`run_check` with file I/O, `--vars`, `--set`, error exit codes, and diagnostic output formatting.
- Fix: Add tests to `crates/mds-cli/tests/language.rs` exercising:
  1. Built-in function calls in `.mds` fixture files compiled via `mds build`
  2. Default arguments with `mds check` validation (valid and invalid arity)
  3. Logical operators `&&`/`||` in `@if`/`@elseif` conditions
  4. Error diagnostics for arity mismatches on builtins (verify exit code 1 and error message format)
  5. Interaction with `--vars`/`--set` overrides

### MEDIUM

**`required_param_count` is misplaced in `evaluator.rs`** — `evaluator.rs:253-256` (Architecture, 82% confidence)
- Problem: `required_param_count` operates solely on `&[Param]` (defined in `ast.rs`) with no dependency on the evaluator module. It is a pure data-query function on an AST type. Placing it in evaluator creates a layering inversion: the validator must `use crate::evaluator::required_param_count`, creating a dependency from validator to evaluator. In a layered pipeline (parser → validator → evaluator), the validator should not depend on the evaluator.
- Fix: Move `required_param_count` to `ast.rs` as a `pub(crate)` function near the `Param` struct. Both evaluator and validator can import from `ast` — which they already depend on — eliminating the layering violation.

**Duplicated function-resolution logic in validator** — `validator.rs:302-343` and `validator.rs:201-236` (Complexity, 90% confidence)
- Problem: The `Arg::Call` arm in `validate_var_args` (42 lines) is a near-exact copy of the `Expr::Call` arm in `validate_expr`. Both perform identical three-step resolution (user-defined → builtin → undefined) with identical arity checks. This creates a maintenance burden: any future change to function resolution must be replicated in both locations, creating drift risk.
- Fix: Extract a shared `validate_call_arity` helper taking `(name, args_len, scope, file, source, offset, len)` that both sites call. Example structure provided in complexity review.

**`replace()` can amplify output size without bound** — `builtins.rs:237` (Reliability, 82% confidence)
- Problem: `builtin_replace` calls Rust's `str::replace(from, to)` with no guard on output length. If `from` is a single character and `to` is a long string on a 10 MB input string with many occurrences, the result could exceed available memory. The `MAX_OUTPUT_SIZE` (50 MB) only guards final accumulated output, not intermediate `Value::String` allocations.
- Fix: Add a post-replacement size guard:
```rust
let result = s.replace(from, to);
if result.len() > MAX_OUTPUT_SIZE {
    return Err(MdsError::resource_limit("replace() result exceeds maximum output size"));
}
Ok(Value::String(result))
```

---

## Should Fix Issues (Recommended Before Merge)

### HIGH

(none)

### MEDIUM

**Diagnostic code naming inconsistency for `BuiltinError`** — `error.rs:142` (Consistency, 85% confidence)
- Problem: The `BuiltinError` variant uses diagnostic code `mds::builtin_type_error`, but the variant name is `BuiltinError` (not `BuiltinTypeError`). Additionally, `BuiltinError` handles more than type errors (e.g., "replace() search string must not be empty"). The diagnostic code is misleading.
- Fix: Rename to `mds::builtin` or `mds::builtin_error` to match the variant name and update the label from "type mismatch" to "built-in function error".

**`builtin_sort` has high cyclomatic complexity** — `builtins.rs:412-470` (Complexity, 82% confidence)
- Problem: 58 lines with cyclomatic complexity ~11, deepest nesting 4 levels (fn > match > for > match). String and Number arms duplicate validate-then-clone-then-sort structure.
- Fix: Extract type-specific sort helpers (`sort_strings`, `sort_numbers`) to reduce the main function to ~15 lines.

**Stale version reference in `validate_for_node` comment** — `validator.rs:115` (Consistency, 92% confidence)
- Problem: Comment says "out of scope for v0.1" but this PR ships v0.2.0. This breaks consistency with the rest of the codebase.
- Fix: Update to "out of scope for v0.2" or remove version reference.

**`reverse()` corrupts strings with Unicode combining characters** — `builtins.rs:397` (Reliability, 83% confidence, Rust, 85% confidence)
- Problem: `s.chars().rev()` reverses Unicode scalar values, breaking grapheme clusters. Reversing `"e\u{0301}"` (e + combining acute accent) produces `"\u{0301}e"` (accent + bare e), which renders incorrectly. Flag emoji similarly break.
- Fix: Document the scalar-value semantics in the spec/docs, or use the `unicode-segmentation` crate for grapheme-aware reversal if desired.

**`unique_key` generates O(m) string keys for nested arrays/objects** — `builtins.rs:476-489` (Performance, 82% confidence)
- Problem: For nested arrays/objects, complexity degrades from O(n) to O(n*m) where m is element Display length. Not merge-blocking given 10 MB file size limit, but worth noting.
- Fix: Add doc comment noting complexity caveat for nested values (bounded in practice by MAX_FILE_SIZE).

**`join` clones all strings before joining** — `builtins.rs:358-368` (Performance, 80% confidence)
- Problem: Iterates the array twice in effect: first to clone into a `Vec<String>`, then to join. Doubles transient string allocation for large arrays.
- Fix: Use a manual fold or itertools to avoid intermediate vector allocation.

**Stale `v0.1` references in `spec.md`** — `spec.md:304` (Consistency, 88% confidence)
- Problem: Line 304 says "no bare module names in v0.1" but spec header is v0.2.0, creating inconsistency.
- Fix: Remove version pin or update to current version.

### LOW

**Duplicated arity range display tests** — `parser_tests.rs:731-758` vs `error_tests.rs:282-290` (Testing, 85% confidence)
- Problem: Three tests in `parser_tests.rs` test identical behavior to tests in `error_tests.rs` for arity display formatting.
- Fix: Remove duplicates from `parser_tests.rs` since arity formatting belongs in the error module.

---

## Additional Should-Fix Issues (Lower Priority)

**Missing builtin type-error tests** — (Testing)
- `contains` on non-string/non-array first argument (82% confidence)
- `reverse` type error path (82% confidence)
- `slice` type error path (82% confidence)
- `number()` with array/object input (80% confidence)

**Missing `condvalue_to_value` conversion tests** — `evaluator.rs` (Testing, 80% confidence)
- No direct unit test for conversion of all four `CondValue` variants (String, Number, Boolean, Null).

**Missing `contains` array-not-found test** — `builtins.rs` (Testing, 85% confidence)
- Tests cover string and array found cases, but missing array not-found.

---

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`debug_assert` for And/Or nesting invariant is invisible in release builds** — `evaluator.rs:418-420` (Reliability, 80% confidence)
- Parser guarantees max depth 2, and leaf count is bounded to 16, so recursion is safe. However, the assertion provides no protection in production. Accepted tradeoff but worth documenting.

**`call_stack` uses O(n) linear scan for recursion detection** — `evaluator.rs:265` (Performance, pre-existing, 85% confidence)
- At MAX_CALL_DEPTH=128, the O(n) scan is acceptable. Already acknowledged in feature knowledge as intentional design choice.

---

## Strengths of This PR

1. **Security** (9/10) — All 18 built-in functions validate argument types. Input validation at boundaries, defense-in-depth limits (MAX_LOGICAL_OPERANDS, NaN guards, empty-separator checks), proper Unicode handling for scalar values.

2. **Architecture** (8/10) — `builtins.rs` is a textbook deep module (simple interface, rich internals). `BuiltinMeta` registry eliminates dispatch match. `Param` with `Option<CondValue>` default reuses existing abstractions cleanly. Logical operators integrate well into recursive condition model.

3. **Regression** (9/10) — All type changes are internal and all call sites updated. `MdsError` is `#[non_exhaustive]`. New variants handled everywhere. Behavior preserved for simple conditions and fixed-arity functions. 703 tests pass (up from ~590).

4. **Testing** (7/10) — Impressive 113 new tests covering all 18 builtins, new Arg variants, default parsing, operator precedence, edge cases (Unicode, NaN, infinity, empty arrays, type errors, resource limits). High quality with AAA structure and behavior-focused assertions. Gap: CLI-level testing.

5. **Consistency** (8/10) — Error handling pattern follows established conventions. Arity checking consistent across validator/evaluator. Resolution order consistent (user-defined first, then builtin). AST exhaustive matching enforced by compiler.

---

## Action Plan

1. **Fix double linear scan** — Use `meta.handler` directly (CRITICAL)
2. **Add CLI integration tests** — Add to `crates/mds-cli/tests/language.rs` (HIGH)
3. **Move `required_param_count` to ast.rs** — Eliminate validator→evaluator dependency (MEDIUM)
4. **Extract `validate_call_arity` helper** — Consolidate validator's duplicated logic (MEDIUM)
5. **Add `replace()` output size guard** — Prevent unbounded amplification (MEDIUM)
6. **Fix diagnostic code naming** — Rename `builtin_type_error` to `builtin` (MEDIUM)
7. **Update version references** — Fix stale v0.1 comments in code and spec (MEDIUM)
8. **Document `reverse()` Unicode behavior** — Or implement grapheme-aware reversal (MEDIUM)
9. **Add missing builtin tests** — Cover type-error paths and missing cases (should-fix)
10. **Optimize `sort()` and `join()` if time permits** — Extract sort helpers, optimize join (should-fix)

---

## Convergence Status

**Cycle**: 1
**Prior Resolution**: none
**Prior FP Ratio**: N/A (first cycle)
**Assessment**: First cycle — 5 blocking issues identified across all domains. All are straightforward fixes with clear resolution paths. No high false-positive ratio or architectural concerns.

---

## Summary by Reviewer

| Reviewer | Score | Issues | Recommendation |
|----------|-------|--------|-----------------|
| Security | 9/10 | 0 blocking, 3 suggestions | APPROVED |
| Architecture | 8/10 | 1 MEDIUM blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Performance | 7/10 | 1 HIGH blocking, 2 MEDIUM, 3 suggestions | APPROVED_WITH_CONDITIONS |
| Complexity | 7/10 | 1 HIGH blocking, 3 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Consistency | 8/10 | 2 MEDIUM blocking, 1 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Regression | 9/10 | 0 blocking | APPROVED |
| Testing | 7/10 | 1 HIGH blocking, 5 MEDIUM should-fix | CHANGES_REQUESTED |
| Reliability | 8/10 | 2 MEDIUM blocking, 1 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Rust | 9/10 | 1 MEDIUM blocking | APPROVED_WITH_CONDITIONS |

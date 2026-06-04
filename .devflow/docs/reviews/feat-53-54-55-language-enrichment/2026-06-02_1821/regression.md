# Regression Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02
**PR**: #70
**Applies**: ADR-008 (bundled language features in single PR)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Analysis

### 1. Return Type Change: `call_function` (`Result<String>` -> `Result<Value>`)

**Confidence**: 95% (no regression)

This is the highest-risk change in the PR. `call_function` and `call_qualified_function` changed from returning `Result<String, MdsError>` to `Result<Value, MdsError>`. All three call sites are correctly updated:

- `evaluate_expr` (line 163, 171): applies `.to_string()` after `?` -- preserves prior behavior for interpolation output.
- `resolve_args` Arg::Call arm (line 220): returns raw `Value` -- this is an intentional behavioral change allowing nested calls to preserve type (e.g., `{join(split(csv, ","), " | ")}` passes the array from `split` to `join` as `Value::Array`, not as a stringified representation). Previously, all nested results were wrapped in `Value::String`, which would have broken built-in composition. Test `builtin_compose_join_split` validates this.
- `call_qualified_function` (line 376): wraps user-defined function result with `.map(Value::String)` -- preserves prior behavior.

Verified: 703 tests pass.

### 2. `DefineBlock.params` Type Change: `Vec<String>` -> `Vec<Param>`

**Confidence**: 95% (no regression)

This is a breaking API change for any code constructing `DefineBlock` directly. However:

- `DefineBlock` is a `pub(crate)` type used only internally.
- WASM and NAPI bindings do not reference `DefineBlock` or `FunctionDef`.
- All internal construction sites are updated: parser (`parse_define_block`), evaluator tests, validator tests, scope tests.
- `Param::required(name)` convenience constructor makes the migration mechanical.
- `FunctionDef.params` in scope.rs is similarly updated.

### 3. `ArityMismatch` Variant Change: `expected: usize` -> `expected_min: usize` + `expected_max: usize`

**Confidence**: 95% (no regression)

The `MdsError::ArityMismatch` variant changed from a single `expected` field to `expected_min`/`expected_max`. This is a field-level breaking change, but:

- `MdsError` is `#[non_exhaustive]`, so downstream consumers should not be pattern-matching on variant fields.
- The `arity()` and `arity_at()` constructors are `pub(crate)`, not public API.
- Error serialization via `serialize()` uses `Display` (which uses `format_arity`), not field extraction -- so JSON error output format changes are in the message text only.
- `format_arity` correctly handles: single arg (1 == 1 -> "expected 1 argument"), plural (N == N -> "expected N arguments"), range (min != max -> "expected M-N arguments").
- The `api_surface.rs` test is updated to use both fields.

### 4. New `BuiltinError` Variant

**Confidence**: 100% (no regression)

Adding a new variant to a `#[non_exhaustive]` enum is backward compatible by definition. The variant is:
- Included in the `serialize()` match arm for span extraction.
- Tested in `error_tests.rs` for display and serialization.
- Listed in `api_surface.rs` exhaustive match.

### 5. New `Condition::And` / `Condition::Or` Variants

**Confidence**: 95% (no regression)

- `Condition::path()` returns `&[]` for compound variants -- callers that used `path()` only on leaf conditions are unaffected.
- `Condition::root()` returns `Err` for compound variants -- this is a behavior change, but the only caller (`validate_condition`) now pattern-matches on the variant directly and only calls `root()` in the leaf arm.
- `validate_condition` recursively validates all operands (conservative -- no short-circuit), matching the documented gotcha in the feature knowledge.
- `evaluate_condition` implements short-circuit evaluation with `debug_assert!` guards against unexpected nesting patterns.
- `parse_condition` still falls through to `parse_simple_condition` for non-compound conditions, preserving all v0.1 behavior.
- `MAX_LOGICAL_OPERANDS = 16` cap is enforced after parsing in `parse_condition`.

### 6. New `Arg` Variants: `NumberLiteral`, `BooleanLiteral`, `NullLiteral`

**Confidence**: 100% (no regression)

All three match sites are exhaustive (compiler enforced):
- `parse_single_arg_inner` (parser_helpers.rs): constructs the new variants. `true`/`false`/`null` keywords are checked before identifier fallback, and numeric literals are checked before member access (dot-path), preventing `3.14` from being parsed as a dot-path.
- `resolve_args` (evaluator.rs): maps to corresponding `Value` variants.
- `validate_var_args` (validator.rs): grouped with `StringLiteral` as no-op validation.

### 7. Parser Refactoring: `parse_define_block` parameter parsing

**Confidence**: 95% (no regression)

The inline parameter parsing in `parse_define_block` was extracted to `parse_define_params` in `parser_helpers.rs`. The new function:
- Uses `split_on_unquoted_commas` to correctly handle commas inside default value strings (e.g., `name = "a, b"`).
- Preserves duplicate-param-name rejection.
- Preserves invalid-identifier rejection.
- Adds new functionality: default value parsing, required-before-optional ordering enforcement.
- The old `HashSet` import in `parser.rs` was removed (moved to `parser_helpers.rs`).

### 8. Validator Refactoring: Extracted `validate_if_node` and `validate_for_node`

**Confidence**: 100% (no regression)

Pure extract-method refactoring. The bodies of the `Node::If` and `Node::For` arms in `validate_node` were moved verbatim to separate functions. The only difference is the match arm now delegates to the extracted function. All comments and invariants are preserved, including the ACCEPTED LIMITATION note about dot-path iterables.

### 9. Built-in Function Shadowing

**Confidence**: 95% (no regression)

User-defined functions shadow built-ins by design. The lookup order in `call_function` is:
1. User-defined (via `scope.get_function`)
2. Built-in (via `get_builtin`)
3. Error (undefined)

This means existing templates that define functions named `upper`, `lower`, etc. will continue to use their custom definitions. Test `builtin_shadowed_by_user_function` validates this.

### 10. Spec Version Bump

The specification was updated from v0.1 to v0.2 with accurate documentation of all new features. No existing spec text was removed -- only additive changes for:
- Logical operators (`&&`, `||`) in section 4.3
- Default arguments in section 4.5
- Built-in function table in section 4.5
- Literal argument types in section 4.5

## Regression Checklist

- [x] No exports removed without deprecation
- [x] Return types backward compatible (internal change only, all call sites updated)
- [x] Default values unchanged for existing behavior
- [x] Side effects preserved (warning collection pattern unchanged)
- [x] All consumers of changed code updated
- [x] Migration complete across codebase
- [x] CLI options preserved (no CLI changes in this PR)
- [x] API endpoints preserved (no public API changes -- all changes are `pub(crate)`)
- [x] Commit messages match implementation
- [x] Breaking changes documented in spec.md (v0.1 -> v0.2)
- [x] `api_surface.rs` updated with new variants and field changes
- [x] All 703 tests pass

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

This PR introduces three tightly coupled language features (built-in functions, default arguments, logical operators) across 15 files with ~2800 lines of changes. Despite the scope, regression risk is well-managed:

1. All type changes (`DefineBlock.params`, `ArityMismatch`, `call_function` return type) are internal (`pub(crate)`) and all call sites are updated.
2. `MdsError` is `#[non_exhaustive]`, making the new `BuiltinError` variant backward compatible.
3. The `Condition` enum additions (`And`/`Or`) are handled everywhere: parser, validator, evaluator, and the `path()`/`root()` methods.
4. Existing behavior is preserved for simple conditions (no `&&`/`||`), fixed-arity functions (no defaults), and string-only arguments. The new code paths only activate for the new syntax forms.
5. Comprehensive test coverage: 703 tests pass (up from ~590 on main), with dedicated tests for all new features, edge cases (NaN, infinity, empty arrays, Unicode), and integration scenarios.

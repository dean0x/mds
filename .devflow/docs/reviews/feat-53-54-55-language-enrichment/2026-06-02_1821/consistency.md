# Consistency Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Diagnostic code naming inconsistency for `BuiltinError`** - `crates/mds-core/src/error.rs:142`
**Confidence**: 85%
- Problem: The `BuiltinError` variant uses diagnostic code `mds::builtin_type_error`, but the variant name is `BuiltinError` (not `BuiltinTypeError`). Every other variant in this enum has its diagnostic code closely mirror the variant name: `Syntax` -> `mds::syntax`, `UndefinedVariable` -> `mds::undefined_var`, `ArityMismatch` -> `mds::arity`, `TypeError` -> `mds::type_error`, etc. Additionally, `BuiltinError` handles more than type errors -- e.g., `replace() search string must not be empty` and `split() separator must not be empty` are validation errors, not type mismatches. The diagnostic code `builtin_type_error` is misleading for these cases.
- Fix: Rename the diagnostic code to `mds::builtin` or `mds::builtin_error` to match the variant name and accurately cover all error cases:
  ```rust
  #[diagnostic(code(mds::builtin))]
  BuiltinError {
  ```
  Also update the `#[label]` from `"type mismatch"` to something more general like `"built-in function error"`, since not all `BuiltinError` instances are type mismatches.

**Stale version reference in `validate_for_node` comment** - `crates/mds-core/src/validator.rs:115`
**Confidence**: 92%
- Problem: The comment says "out of scope for v0.1" but this PR ships v0.2.0. The validator refactored inline code into `validate_for_node` and carried forward the old comment verbatim. Since the spec header is now v0.2 and the feature knowledge references v0.2.0, this stale version reference breaks consistency with the rest of the codebase.
- Fix: Update the comment to reflect the current version:
  ```rust
  // out of scope for v0.2.
  ```
  Or remove the version reference entirely and use a more timeless phrasing:
  ```rust
  // out of scope for the current release.
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Stale `v0.1` references in `spec.md`** - `spec.md:304`
**Confidence**: 88%
- Problem: `spec.md` line 304 says "no bare module names in v0.1" but the spec header was updated to v0.2 in this PR. The spec version history at line 811 correctly documents v0.2.0, creating an inconsistency within the same file. This is the only stale `v0.1` reference in prose rules (the other at line 813 is in the version history entry and is correctly historical).
- Fix: Update to remove the version pin or reference the current version:
  ```
  - Relative paths only (no bare module names)
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`BuiltinError` label says "type mismatch" for all cases** - `crates/mds-core/src/error.rs:145` (Confidence: 75%) -- The `#[label("type mismatch")]` annotation is used even when the error is "replace() search string must not be empty" or "split() separator must not be empty", which are not type mismatches. This is a label mismatch that surfaces in diagnostic output with source spans. Currently not triggered because `builtin_error()` (no `_at` variant) never attaches spans, but `builtin_error_at()` exists and would show the wrong label once used.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This is a well-executed v0.2.0 language enrichment PR (applies ADR-008 -- bundling related language features into a single PR). The consistency across the three features is strong:

- **Error handling pattern**: The new `BuiltinError` and updated `ArityMismatch` follow the established `_at` constructor pattern with the same `(span, src)` fields structure. The `builtin_error` / `builtin_error_at` pair mirrors all other variant constructors.
- **Arity checking**: The evaluator (`call_function`, `invoke_function`), validator (`validate_expr`, `validate_var_args`), and qualified call paths all use consistent range-based arity checks (`args.len() < required || args.len() > total`).
- **Resolution order**: Both validator and evaluator consistently check user-defined functions first, then fall back to builtins, maintaining the documented shadowing behavior.
- **AST exhaustive matching**: All three new `Arg` variants (`NumberLiteral`, `BooleanLiteral`, `NullLiteral`) are correctly handled in all three match sites (parser, evaluator, validator) per the documented integration pattern.
- **Condition handling**: `And`/`Or` are consistently handled recursively in both validator (conservative -- all branches) and evaluator (short-circuit), with appropriate `debug_assert!` guards for parser invariants.
- **Param migration**: The `Vec<String>` -> `Vec<Param>` change is propagated consistently through all sites: AST, scope, parser, validator, evaluator, and tests.
- **Spec sync**: The EBNF grammar, rules sections, and function table in `spec.md` are updated to match the implementation.

The three blocking items are minor naming/documentation inconsistencies that do not affect correctness.

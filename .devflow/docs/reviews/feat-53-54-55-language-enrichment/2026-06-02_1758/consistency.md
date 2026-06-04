# Consistency Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent string semantics across built-in functions: `length()` uses byte count, `reverse()` and `slice()` use char/char-boundary semantics** - `crates/mds-core/src/builtins.rs:341`
**Confidence**: 85%
- Problem: `length()` on strings returns `s.len()` (byte count), but `reverse()` operates on `.chars()` (code points) and `slice()` uses `snap_to_char_boundary` (char-aware). For ASCII-only strings these are equivalent, but for multi-byte UTF-8 strings they diverge. A user calling `slice(s, 0, length(s))` would get the full string, but `length("café")` returns 6 (bytes) while the string has 5 chars. The spec says "String byte length" which matches the implementation, but the API is inconsistent with itself: `slice` is char-boundary-aware while `length` is byte-aware. This creates a confusing user experience where `slice(s, 0, N)` with N from `length(s)` works, but intermediate values of N may silently snap to different boundaries than expected.
- Fix: Either (a) make `length()` return char count (`s.chars().count()`) and update the spec to say "character count", making all string built-ins consistently char-aware, or (b) add a `/// NOTE:` comment in `length()` explicitly documenting why byte count is chosen and how it interacts with `slice()`'s char-boundary snapping. Option (a) is preferred for user-facing consistency.

### MEDIUM

**`BuiltinError` variant missing `_at` constructor, breaking error constructor pattern** - `crates/mds-core/src/error.rs:365`
**Confidence**: 85%
- Problem: Every other span-bearing `MdsError` variant has both a bare constructor (e.g. `builtin_error()`) and a `_at` variant (e.g. `builtin_error_at()`) that accepts `file`, `source`, `offset`, and `len` for source-span diagnostics. `BuiltinError` has only the bare constructor. This means built-in function type errors can never carry source location information, producing worse diagnostics than other error types. All 11 other span-bearing variants follow the pair pattern: `syntax`/`syntax_at`, `undefined_var`/`undefined_var_at`, etc.
- Fix: Add `builtin_error_at` constructor following the established pattern:
  ```rust
  pub(crate) fn builtin_error_at(
      msg: impl Into<String>,
      file: &str,
      source: &str,
      offset: usize,
      len: usize,
  ) -> Self {
      let (span, src) = at(file, source, offset, len);
      MdsError::BuiltinError {
          message: msg.into(),
          span,
          src,
      }
  }
  ```

**Function-level `use` import in `parse_define_params` inconsistent with module-level import pattern** - `crates/mds-core/src/parser_helpers.rs:935`
**Confidence**: 82%
- Problem: `parse_define_params` contains `use std::collections::HashSet;` inside the function body. Every other `std::collections` import in the codebase is at the module level (7 occurrences across `evaluator.rs`, `resolver.rs`, `scope.rs`, `value.rs`, `fs.rs`, `options.rs`, `lib.rs`). The `HashSet` import was previously at module level in `parser.rs` before this PR refactored the code into `parse_define_params`. This function-level import breaks the project's consistent module-level import convention.
- Fix: Move the import to the top of `parser_helpers.rs` alongside the other imports:
  ```rust
  use std::collections::HashSet;
  ```
  And remove the function-level import on line 935.

**Duplicated arity-check logic across three sites in validator without extraction** - `crates/mds-core/src/validator.rs:180-213`, `crates/mds-core/src/validator.rs:229-242`, `crates/mds-core/src/validator.rs:285-321`
**Confidence**: 80%
- Problem: The validator has three near-identical blocks implementing the "user-defined first, then built-in, then undefined" resolution pattern with arity range checking. The same pattern exists in `evaluator.rs:337-353` (`call_function`). While the evaluator was cleanly extracted into `call_function`, the validator repeats the pattern at `validate_expr` (Expr::Call), `validate_expr` (Expr::QualifiedCall), and `validate_var_args` (Arg::Call). Each block duplicates the `required_param_count`/`total`/`min_args`/`max_args` branching. This creates a maintenance risk: if the resolution order changes (e.g. adding a third lookup layer), all three sites must be updated in lockstep. Applies ADR-008 rationale -- bundled features touching the same layers benefit from shared implementation.
- Fix: Extract a helper `validate_call_arity(name, args_len, scope, file, source, offset, len) -> Result<(), MdsError>` in the validator that encapsulates the user-defined/built-in/undefined lookup and arity check.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Two quote-aware splitting implementations using different scanning strategies** - `crates/mds-core/src/parser_helpers.rs:205` and `crates/mds-core/src/parser_helpers.rs:840`
**Confidence**: 80%
- Problem: `split_on_unquoted_op` uses byte-level scanning (`as_bytes()`, `while` loop) while `split_on_unquoted_commas` uses char-level scanning (`for ch in s.chars()`). Both handle the same concerns (tracking in-string state, escape sequences, quote characters) but with different idioms. The byte-level approach has explicit safety documentation; the char-level approach does not. This is not a bug -- both are correct for their ASCII delimiters -- but the inconsistency makes the codebase harder to reason about. The pre-existing `find_unquoted_operator` and `find_unquoted_equals` also use the byte-level idiom, making `split_on_unquoted_commas` the outlier.
- Fix: Consider rewriting `split_on_unquoted_commas` to use byte-level scanning consistent with the other three quote-aware scanning functions, or at minimum add a `# Char-level scan safety` doc comment explaining why char-level scanning was chosen here.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`call_function` and `call_qualified_function` return type asymmetry** - `crates/mds-core/src/evaluator.rs:339,375` (Confidence: 70%) -- Both functions wrap `invoke_function`'s `Result<String, _>` into `Value::String`, but `call_function` also handles built-ins that return arbitrary `Value` types directly. The result is that user-defined functions always produce `Value::String` while built-ins can produce `Value::Number`, `Value::Boolean`, etc. This is intentional and correct, but the `.map(Value::String)` wrapping at line 339 and 375 means user-defined function results are always stringified before being passed to a calling context. If user-defined functions ever need to return typed values, this wrapping will need to be revisited.

- **`reverse()` on strings reverses by Unicode scalar values, not grapheme clusters** - `crates/mds-core/src/builtins.rs:363` (Confidence: 65%) -- `s.chars().rev().collect()` reverses by code points, which can break combining character sequences (e.g. `e` + combining accent). This is a known limitation of `.chars()` in Rust. The spec does not specify grapheme-awareness, but users may expect `reverse("cafe\u{0301}")` to preserve the accent on the `e`.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR bundles three well-scoped features (applies ADR-008) with mostly consistent implementation patterns across all touched files. The main consistency concern is the semantic mismatch in how built-in string functions treat string indexing (byte vs char level), which will surface as user-facing confusion with non-ASCII strings. The missing `_at` constructor, function-level import, and duplicated arity validation are lower-impact but break established codebase patterns. None of these issues are critical -- the code is functional and well-tested -- but they should be addressed before merge to maintain the project's strong pattern consistency.

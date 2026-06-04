# Rust Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`reverse()` corrupts strings containing Unicode combining characters or multi-codepoint grapheme clusters** - `builtins.rs:397`
**Confidence**: 85%
- Problem: `s.chars().rev().collect()` reverses individual Unicode scalar values, which breaks grapheme clusters. For example, `reverse("e\u{0301}")` (e + combining acute accent) produces `"\u{0301}e"` (accent followed by bare e), which renders incorrectly. Similarly, flag emoji like the U.S. flag (two regional indicator symbols) will break when reversed. The same issue applies to `slice` and `length` which also operate at the `char` level, but those degrade more gracefully (just wrong index/count) while `reverse` produces visually corrupted output.
- Fix: This is an accepted design choice documented in KNOWLEDGE.md ("Unicode scalar value indices"). However, consider at minimum documenting this limitation in the spec or adding a doc comment noting that `reverse` operates on scalar values, not grapheme clusters. If grapheme-correct behavior is desired, use the `unicode-segmentation` crate:
  ```rust
  use unicode_segmentation::UnicodeSegmentation;
  fn builtin_reverse(args: &[Value]) -> Result<Value, MdsError> {
      match &args[0] {
          Value::String(s) => {
              let reversed: String = s.graphemes(true).rev().collect();
              Ok(Value::String(reversed))
          }
          // ...
      }
  }
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Duplicate arity check in `call_function` and `call_builtin`** - `evaluator.rs:344`, `builtins.rs:172` (Confidence: 65%) -- `call_function` checks `args.len() < meta.min_args || args.len() > meta.max_args` before calling `call_builtin`. The individual builtin handler functions then rely on this check having happened, indexing `args[0]`, `args[1]`, etc. without bounds checks. If `call_builtin` were ever called directly without the outer arity guard (currently it is not -- only `call_function` calls it), the handlers would panic on out-of-bounds access. The `call_builtin` doc comment claims it returns `MdsError::arity` for wrong arg count, but it does not actually perform that check. Consider adding a defensive arity check inside `call_builtin` or updating the doc comment to clarify the precondition.

- **`unique_key` for Array/Object uses `Display` output which may collide across types** - `builtins.rs:486-487` (Confidence: 62%) -- The `unique_key` function uses `format!("a:{v}")` for arrays and `format!("o:{v}")` for objects, where `v` is the `Display` representation. Two structurally different arrays could produce the same Display string if their elements' Display outputs happen to coincide in a way that looks identical when concatenated. This is unlikely in practice for a template language, and the comment acknowledges it.

- **`arity_at` has 8 parameters with `#[allow(clippy::too_many_arguments)]`** - `error.rs:343` (Confidence: 60%) -- The function takes `(name, expected_min, expected_max, got, file, source, offset, len)`. An `ArityRange { min, max }` struct would reduce this to 7 parameters and make call sites more readable. Low priority since this is a `pub(crate)` constructor.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates strong Rust idioms throughout (applies ADR-008 -- bundling related features). Error handling is clean with proper `Result` propagation, no `.unwrap()` in library code, and the `thiserror`/`miette` integration is well-structured. The type system is used effectively -- `Param` struct with `Option<CondValue>` default, `BuiltinMeta` registry, range-based arity via `expected_min`/`expected_max`, and enum-based AST conditions with `And`/`Or` variants. The single blocking issue is the `reverse()` grapheme cluster behavior, which is MEDIUM severity because it produces corrupted output for a subset of Unicode inputs. All 703 tests pass. The defense-in-depth limits (`MAX_LOGICAL_OPERANDS`, NaN/infinity guards, empty-separator rejection) are thorough. The condition is: document the scalar-value semantics of `reverse`/`slice`/`length` in the spec or function docs so users understand the behavior with combining characters.

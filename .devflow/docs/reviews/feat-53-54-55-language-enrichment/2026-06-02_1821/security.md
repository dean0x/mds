# Security Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

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

- **`replace()` output amplification via large replacement strings** - `crates/mds-core/src/builtins.rs:237` (Confidence: 65%) -- `replace()` performs unbounded Rust `str::replace()` on the input string. A template author could call `replace(large_str, "a", very_long_string)` producing an intermediate `Value::String` much larger than the input before the evaluator's `MAX_OUTPUT_SIZE` check fires. The 10MB `MAX_FILE_SIZE` input cap and 50MB `MAX_OUTPUT_SIZE` evaluator check bound this in practice, but the intermediate allocation is uncapped within `call_builtin`. Unlikely to be exploitable in the template-compiler threat model since template authors are trusted.

- **`unique_key` collision potential with nested arrays/objects** - `crates/mds-core/src/builtins.rs:486-487` (Confidence: 60%) -- For arrays and objects, `unique_key` uses `format!("a:{v}")` / `format!("o:{v}")` which relies on the `Display` implementation. Two structurally different nested arrays could produce the same display output (depending on the `Display` impl for `Value`), causing false-positive deduplication. This is a correctness rather than security issue and is already documented in the code comments as accepted behavior.

- **`number()` accepts locale-dependent edge cases** - `crates/mds-core/src/builtins.rs:519` (Confidence: 60%) -- Rust's `f64::parse` accepts `"NaN"`, `"inf"`, and `"-inf"` strings, but the `is_finite()` guard at line 522 correctly rejects them. The guard is present and tested. No action needed; this is a positive observation.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

This PR adds ~2800 lines implementing three language enrichment features for a Rust-based template compiler. The security analysis covered:

### 1. Input Validation (OWASP A03 - Injection)

All 18 built-in functions validate argument types at entry. Type errors return `MdsError::BuiltinError` rather than panicking. Key defenses observed:

- `require_string` / `require_string_at` enforce string type at each argument position
- `require_number_index` validates finite numbers, clamps negatives to 0, and guards against `usize::MAX` overflow (line 339)
- `replace()` rejects empty search strings (line 232), preventing infinite-loop behavior
- `split()` rejects empty separators (line 243)
- `sort()` rejects non-finite numbers (NaN, Infinity) and mixed-type arrays
- `number()` rejects NaN/Infinity results after string parsing (line 522)
- `parse_single_arg_inner` rejects NaN/Infinity numeric literals at parse time (line 812)

### 2. Resource Exhaustion (OWASP A04 - Insecure Design)

Defense-in-depth limits are well-applied:

- `MAX_LOGICAL_OPERANDS = 16` (new, `limits.rs:24`) caps condition tree complexity -- prevents adversarial `&&`/`||` chains. `count_leaf_operands` counts recursively to prevent circumvention via nesting (applies ADR-008 -- features bundled together share the same limits infrastructure)
- `MAX_CALL_DEPTH = 128` limits nested function call chains
- `MAX_OUTPUT_SIZE = 50MB` caps final output
- `MAX_NESTING_DEPTH = 64` limits parser recursion
- `MAX_ELSEIF_BRANCHES = 256` caps branch count
- `MAX_FILE_SIZE = 10MB` caps input

The condition parser enforces `MAX_LOGICAL_OPERANDS` after parsing (lines 300-305 and 312-317 in `parser_helpers.rs`), which means it parses first then checks -- but since parsing a flat condition list is O(n) with n capped by string length (which is capped by `MAX_FILE_SIZE`), this is safe.

### 3. Numeric Safety

- `f64` to `usize` conversion in `require_number_index` (line 335-344) properly floors, clamps negatives to 0, and rejects values exceeding `usize::MAX`
- `sort()` uses `total_cmp` for number comparison (line 460), which handles NaN correctly (NaN is rejected before this point)
- `length()` returns char count as `f64` (line 375) -- `usize as f64` is lossless for any realistic string length

### 4. Unicode / String Safety

- `slice()` uses character-based indexing via `.chars()` iterator (not byte offsets), preventing panics on multi-byte UTF-8 sequences
- `length()` counts chars, not bytes -- consistent and safe
- `reverse()` reverses chars, not bytes -- safe for multi-byte characters but note: combining characters / grapheme clusters will be reordered (documented behavior, not a security issue)
- `split_on_unquoted_op` in `parser_helpers.rs` (line 207-248) scans bytes but only for ASCII operators (`&&`, `||`, `=`, `,`), which is sound because these are single-byte characters that cannot appear as continuation bytes in multi-byte UTF-8 sequences

### 5. Operator Parsing Security

- The `split_on_unquoted_op` function correctly tracks string state (single/double quotes) and escape sequences when splitting on logical operators
- Operators inside quoted strings are not treated as operators (test at line 970: `parse_condition_string_with_operator_inside_quotes`)
- Empty operands produce clear syntax errors (lines 268-269, 294-295)

### 6. Scope / Shadowing Safety

- User-defined functions shadow built-ins (intentional design, line 339 of evaluator) -- this is safe because it gives template authors control. The validator checks user-defined functions first, then falls back to built-ins (line 202-234 of validator)
- Default parameter values are bound after closure scope restoration but overwrite captured vars (line 294-309 of evaluator) -- params correctly shadow captured vars
- The `required_param_count` function is used consistently in both validator and evaluator for arity checking

### 7. Error Information Disclosure

- `BuiltinError` messages include function names and argument types but no filesystem paths or internal state -- appropriate for a template compiler
- The `debug-panics` Cargo feature (mentioned in CLAUDE.md gotchas) is not touched by this PR

### Positive Security Patterns

- All built-in functions are `pub(crate)` -- not directly accessible from external code
- The `BUILTINS` registry is a `static` slice -- immutable at runtime
- Handler functions are plain `fn` pointers, not closures -- no captured state that could leak
- Arity is double-checked: once by the caller in `call_function` (evaluator line 344) and once implicitly by argument indexing in each handler. The validator also independently checks arity at compile time

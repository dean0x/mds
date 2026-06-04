# Security Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**`split("")` enables O(n) memory amplification from user input** - `crates/mds-core/src/builtins.rs:219-223`
**Confidence**: 82%
- Problem: `builtin_split` passes the separator directly to `str::split()`. When `sep` is an empty string, Rust's `split("")` produces N+2 parts for a string of length N (one per byte boundary). A 10MB input string (within MAX_FILE_SIZE) would produce ~10 million `Value::String` allocations, each with String heap overhead (~24 bytes + 1 char), totaling ~250-300MB of heap usage -- well beyond the input size. This is a memory amplification vector for adversarial inputs.
- Fix: Reject empty separator with a clear error, or cap the result count:
```rust
fn builtin_split(args: &[Value]) -> Result<Value, MdsError> {
    let s = require_string_at(args, 0, "split", "first")?;
    let sep = require_string_at(args, 1, "split", "second")?;
    if sep.is_empty() {
        return Err(MdsError::builtin_error(
            "split() separator must not be empty".to_string(),
        ));
    }
    let parts: Vec<Value> = s.split(sep).map(|p| Value::String(p.to_string())).collect();
    Ok(Value::Array(parts))
}
```

**`replace("", x)` enables O(n) output amplification** - `crates/mds-core/src/builtins.rs:212-216`
**Confidence**: 80%
- Problem: `builtin_replace` passes `from` directly to `str::replace()`. When `from` is an empty string, Rust inserts the replacement before every character and at the end. For a 10MB input with `replace(s, "", "XX")`, the output grows to ~30MB. Longer replacement strings amplify further. While MAX_OUTPUT_SIZE (50MB) provides an upper bound at the evaluator level, the 50MB allocation happens inside the builtin before the evaluator can check -- the amplification occurs within the `str::replace()` call itself.
- Fix: Reject empty `from` string:
```rust
fn builtin_replace(args: &[Value]) -> Result<Value, MdsError> {
    let s = require_string_at(args, 0, "replace", "first")?;
    let from = require_string_at(args, 1, "replace", "second")?;
    if from.is_empty() {
        return Err(MdsError::builtin_error(
            "replace() search string must not be empty".to_string(),
        ));
    }
    let to = require_string_at(args, 2, "replace", "third")?;
    Ok(Value::String(s.replace(from, to)))
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`unique()` has O(n^2) time complexity via linear scan** - `crates/mds-core/src/builtins.rs:430-443`
**Confidence**: 80%
- Problem: The `unique()` implementation uses `Vec::contains()` for deduplication, which is O(n) per element, yielding O(n^2) overall. For a large array (e.g., 10K+ elements produced by `split`), this becomes a CPU-bound denial-of-service vector. While arrays are bounded by input size (10MB), a split producing 10M single-char strings piped through `unique()` would cause ~10^14 comparisons.
- Fix: Use a `HashSet` (or `IndexSet` from the `indexmap` crate) for O(1) membership checks while preserving insertion order. Alternatively, cap array size for builtin operations.

**`sort()` treats NaN as equal via `unwrap_or(Ordering::Equal)`** - `crates/mds-core/src/builtins.rs:413-416`
**Confidence**: 82%
- Problem: The sort comparator uses `partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)` which treats NaN comparisons as "equal". While the parser blocks NaN literals, NaN can be introduced at runtime through `number()` on edge-case strings (though `number()` rejects non-finite -- this path is currently blocked). The `unwrap_or(Equal)` pattern is a known footgun that produces implementation-defined sort order when NaN is present, violating the sort's total ordering contract. This is not exploitable today because `number()` rejects NaN, but it is a latent defect.
- Fix: Add a pre-sort check that rejects arrays containing NaN:
```rust
for item in &sorted {
    if let Value::Number(n) = item {
        if n.is_nan() {
            return Err(MdsError::builtin_error(
                "sort() cannot sort arrays containing NaN values".to_string(),
            ));
        }
    }
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`length()` returns byte length for strings, not character count** - `crates/mds-core/src/builtins.rs:339-345`
**Confidence**: 70% (moved to Suggestions -- this is a design choice, not a vulnerability)

## Suggestions (Lower Confidence)

- **`length()` byte-vs-char semantic mismatch** - `crates/mds-core/src/builtins.rs:341` (Confidence: 70%) -- `s.len()` returns byte count, not character count. For multi-byte UTF-8 strings like "café", `length("café")` returns 6 instead of the expected 5 characters. The spec documents this as "String byte length" which is technically correct, but users of a template language will expect character count. Combined with `slice()` using byte indices via `snap_to_char_boundary`, this creates an internally consistent but potentially confusing byte-oriented string API. Not a security issue, but worth noting.

- **No array size limit on builtin return values** - `crates/mds-core/src/builtins.rs` (Confidence: 65%) -- Built-in functions like `split()` can produce unbounded arrays (bounded only by input size). While the evaluator's MAX_OUTPUT_SIZE catches oversized final output, intermediate Value::Array objects are not size-checked. A chain like `split(large_str, ",")` could produce millions of elements that exist in memory before the output limit kicks in. Consider adding a MAX_ARRAY_LENGTH limit.

- **Error messages in `number()` echo user input** - `crates/mds-core/src/builtins.rs:456` (Confidence: 62%) -- The error message `"number() cannot convert string '{s}' to a number"` echoes the input string `s` verbatim. In a web context where MDS errors might be displayed to users, this could facilitate XSS if error messages are rendered as HTML. For a template compiler this is low risk, but worth noting for defense in depth.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR demonstrates good security awareness: resource limits (MAX_LOGICAL_OPERANDS, MAX_CALL_DEPTH), input validation (NaN/infinity rejection in number literals, required-before-optional parameter ordering enforcement, duplicate parameter detection), and safe UTF-8 handling (snap_to_char_boundary for string slicing). The `number()` builtin correctly rejects non-finite values using `is_finite()`. The logical operator parser correctly handles quoted strings to prevent operator injection. User-defined functions correctly shadow built-ins, preventing built-in override attacks.

The two blocking MEDIUM findings (`split("")` and `replace("", x)` amplification) are straightforward to fix with empty-string guards. The should-fix items (`unique()` O(n^2) and `sort()` NaN handling) are lower urgency but worth addressing to harden against adversarial inputs.

# Performance Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02
**Applies**: ADR-008 (bundle related small language features into a single PR)

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`unique()` is O(n^2) due to Vec::contains linear scan** - `builtins.rs:436-441`
**Confidence**: 85%
- Problem: `builtin_unique` uses `result.contains(item)` inside a loop over the input array. `Vec::contains` is O(n) per call, making the total complexity O(n^2). For large arrays this becomes a bottleneck. There is no array size limit applied before calling built-in functions -- the only relevant limit is `MAX_LOOP_ITERATIONS` (100k) for `@for` loops, but arrays constructed via `split()` or passed as frontmatter data have no cap.
- Fix: Use a `HashSet` (or `IndexSet` from the `indexmap` crate for order-preserving dedup) as a seen-set for O(1) lookups. Since `Value` does not implement `Hash`, a pragmatic approach is to use a string-serialized key or implement `Hash` for the subset of types that `unique` supports:
```rust
fn builtin_unique(args: &[Value]) -> Result<Value, MdsError> {
    let arr = match &args[0] {
        Value::Array(a) => a,
        other => return Err(type_err("unique", "", "array", other.type_name())),
    };
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result: Vec<Value> = Vec::new();
    for item in arr {
        let key = item.to_string();
        if seen.insert(key) {
            result.push(item.clone());
        }
    }
    Ok(Value::Array(result))
}
```

**`split()` produces unbounded output without element count limit** - `builtins.rs:219-224`
**Confidence**: 82%
- Problem: `builtin_split` calls `s.split(sep).collect()` with no cap on the number of resulting elements. A 10 MB input string (allowed by `MAX_FILE_SIZE`) split on a single-character separator could produce millions of `Value::String` allocations, each with its own heap allocation. This is a potential OOM vector for adversarial templates.
- Fix: Either cap the number of split parts (e.g., `s.splitn(MAX_SPLIT_PARTS, sep)`) or add a general array size limit in `limits.rs`:
```rust
const MAX_ARRAY_ELEMENTS: usize = 100_000;
// In builtin_split:
let parts: Vec<Value> = s.split(sep)
    .take(MAX_ARRAY_ELEMENTS)
    .map(|p| Value::String(p.to_string()))
    .collect();
```

**`replace()` can amplify output size without bounds checking** - `builtins.rs:212-217`
**Confidence**: 80%
- Problem: `s.replace(from, to)` with an empty `from` string inserts `to` between every character and at both ends. For a 10 MB input string and a non-trivial replacement string, this can amplify output size by orders of magnitude, bypassing `MAX_OUTPUT_SIZE` because the check only runs in the evaluator's `evaluate_nodes` loop, not inside built-in functions. The resulting allocation happens all at once in a single `String::replace` call.
- Fix: Either reject empty `from` strings or check the result length against `MAX_OUTPUT_SIZE`:
```rust
fn builtin_replace(args: &[Value]) -> Result<Value, MdsError> {
    let s = require_string_at(args, 0, "replace", "first")?;
    let from = require_string_at(args, 1, "replace", "second")?;
    let to = require_string_at(args, 2, "replace", "third")?;
    if from.is_empty() {
        return Err(type_err("replace", "second", "non-empty string", "empty string"));
    }
    Ok(Value::String(s.replace(from, to)))
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`sort()` does two passes over the array: validation then sort** - `builtins.rs:378-428`
**Confidence**: 82%
- Problem: `builtin_sort` first iterates the entire array to validate type homogeneity (O(n)), then sorts it (O(n log n)). The validation pass is redundant because the sort comparator could detect and reject mixed types on first encounter. This doubles the constant factor on large arrays. Additionally, `arr.clone()` is called unconditionally before the type check, meaning a type error on a large array still pays the clone cost.
- Fix: Move the type check into the comparator and bail on first mismatch, or at minimum move the clone after the validation pass:
```rust
// Validate first without cloning
match &arr[0] {
    Value::String(_) => {
        for item in arr { /* validate */ }
    }
    // ...
}
// Clone only after validation passes
let mut sorted = arr.clone();
```

**`get_builtin()` linear scan on every function call** - `builtins.rs:126-128`
**Confidence**: 80%
- Problem: `get_builtin` does a linear scan of the 18-element `BUILTINS` array on every function call that is not user-defined. The evaluator calls `get_builtin` once and the validator calls it separately, so in the common case of a built-in call, two linear scans happen per call site. With 18 elements this is not critical, but as more built-ins are added this will degrade.
- Fix: Use a `phf` (perfect hash function) map or a `HashMap` initialized via `lazy_static`/`once_cell` for O(1) lookup. Alternatively, a sorted array with binary search would be a minimal-dependency improvement:
```rust
// BUILTINS sorted by name, then:
pub(crate) fn get_builtin(name: &str) -> Option<&'static BuiltinMeta> {
    BUILTINS.binary_search_by_key(&name, |b| b.name).ok().map(|i| &BUILTINS[i])
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`length()` returns byte count for strings, not character count** - `builtins.rs:341`
**Confidence**: 85%
- Problem: `s.len()` returns byte length, not character count. For UTF-8 strings with multi-byte characters, `length("cafe\u{0301}")` returns 6, not 5 (or 4 depending on normalization). Since `slice()` also uses byte indices (with `snap_to_char_boundary`), this creates a semantic mismatch: `length` reports a count that cannot be reliably used as a `slice` endpoint for character-based operations. This is a correctness/semantic issue more than performance, but the performance implication is that users who iterate `length` times calling `slice` will get unexpected truncation on multi-byte strings, potentially causing retry loops or re-processing.
- Note: This is a design decision, not necessarily a bug. The approach should be documented -- either commit to byte-based semantics (and document it) or switch both `length` and `slice` to character-based indexing (`s.chars().count()` for length, `char_indices` for slicing).

## Suggestions (Lower Confidence)

- **`reverse()` on strings is O(n) in chars but clones via iterator** - `builtins.rs:363` (Confidence: 65%) -- `s.chars().rev().collect()` allocates a new String by iterating all chars. For very large strings an in-place byte reversal (char-boundary-aware) would avoid the extra allocation, but this is micro-optimization territory for a template language.

- **`sort()` uses `partial_cmp(...).unwrap_or(Equal)` for NaN handling** - `builtins.rs:414-416` (Confidence: 70%) -- If an array contains NaN values, they will sort non-deterministically (NaN compares as Equal to everything). Consider rejecting NaN before sorting or using `total_cmp()` (stable since Rust 1.62) for deterministic ordering.

- **`condvalue_to_value` clones String unnecessarily when called in default-param fill** - `evaluator.rs:246` (Confidence: 60%) -- Each default parameter fill clones the CondValue::String. For functions called in tight loops with default args, this could accumulate. An `into` variant that takes ownership would avoid the clone, but the frequency is low in practice.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 3 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The core implementations are straightforward and the algorithmic choices are reasonable for typical template workloads. The three blocking MEDIUM issues (`unique` O(n^2), `split` unbounded output, `replace` output amplification) all share a common theme: built-in functions operate outside the evaluator's resource limit checks (`MAX_OUTPUT_SIZE`, `MAX_LOOP_ITERATIONS`), creating potential amplification vectors for adversarial inputs. These should be addressed with bounds checks or caps before merge, or explicitly documented as accepted limitations with a follow-up issue filed.

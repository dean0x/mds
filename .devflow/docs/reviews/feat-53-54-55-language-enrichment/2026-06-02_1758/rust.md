# Rust Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02
**Applies**: ADR-008 (bundle related language features into single PR)

## Issues in Your Changes (BLOCKING)

### HIGH

**`length()` returns byte count, not character count for strings** - `crates/mds-core/src/builtins.rs:341`
**Confidence**: 90%
- Problem: `builtin_length` uses `s.len()` which returns byte length, not character count. For multi-byte UTF-8 strings (e.g. "café" is 5 bytes but 4 characters, emojis are 4 bytes but 1 character), users will get surprising results. This is inconsistent with `slice()` which was carefully fixed with `snap_to_char_boundary` to handle multi-byte characters correctly, and `reverse()` which uses `s.chars().rev()` (character-aware). The three functions present an inconsistent mental model: `reverse("café")` operates on chars, `length("café")` returns bytes.
- Fix: Use `s.chars().count()` for character-level semantics consistent with the rest of the string builtins, or document this explicitly as byte-length and add a `char_count`/`chars` builtin:
```rust
Value::String(s) => Ok(Value::Number(s.chars().count() as f64)),
```

**`sort()` treats NaN as equal to everything via `unwrap_or(Equal)`** - `crates/mds-core/src/builtins.rs:414-416`
**Confidence**: 85%
- Problem: The numeric sort uses `a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)`. If a NaN value enters the array (e.g. produced by `number()` of a computed result or future arithmetic), `partial_cmp` returns `None` for any NaN comparison, and the fallback to `Equal` means NaN silently occupies an arbitrary position in the sorted output. The sort's contract says "homogeneous array of numbers" but does not filter NaN. The existing `number()` builtin rejects non-finite values, but a NaN could still enter via the `Value::Number(f64)` type from external data (JSON frontmatter, for example).
- Fix: Add a NaN check in the sort validation pass or use `total_cmp` (stable since Rust 1.62) which provides a total ordering including NaN:
```rust
sorted.sort_by(|a, b| match (a, b) {
    (Value::Number(a), Value::Number(b)) => a.total_cmp(b),
    _ => unreachable!(),
});
```

### MEDIUM

**`expect()` panic in default parameter binding** - `crates/mds-core/src/evaluator.rs:305`
**Confidence**: 82%
- Problem: The `.expect("BUG: non-optional param missing but arity check passed")` call will panic if the invariant is violated. While the comment explains why the invariant should hold, a library crate panicking is not ideal. The CLAUDE.md notes `catch_unwind` at the JS boundary, so this won't crash a host process, but it produces an opaque abort rather than a structured `MdsError`. All other error paths in the evaluator return `Result`, making this the only panic path in new code.
- Fix: Replace with an explicit error return:
```rust
condvalue_to_value(
    param.default.as_ref().ok_or_else(|| {
        MdsError::syntax(format!(
            "internal error: non-optional param '{}' missing but arity check passed",
            param.name
        ))
    })?
)
```

**`slice()` semantics mismatch: byte-based indexing exposed to users** - `crates/mds-core/src/builtins.rs:256-268`
**Confidence**: 82%
- Problem: The `slice()` function accepts numeric indices that are interpreted as byte offsets (after `snap_to_char_boundary` adjustment). Users writing `slice("café", 0, 4)` likely expect the first 4 characters, but get the first 3 because byte 4 falls inside the 2-byte `é` and snaps back to byte 3. The `snap_to_char_boundary` fix prevents panics (good), but the semantics are still byte-based, which is surprising for a template language targeting non-systems-programmers. This creates a confusing API where `slice("hello", 0, 3)` gives "hel" (correct) but `slice("café", 0, 4)` gives "caf" (surprising).
- Fix: Convert indices to character offsets using `s.char_indices()`:
```rust
Value::String(s) => {
    let start_idx = require_number_index(&args[1], "slice", "second")?;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let char_len = chars.len();
    let start = start_idx.min(char_len);
    let byte_start = chars.get(start).map_or(s.len(), |(i, _)| *i);
    if args.len() == 3 {
        let end_idx = require_number_index(&args[2], "slice", "third")?;
        let end = end_idx.clamp(start, char_len);
        let byte_end = chars.get(end).map_or(s.len(), |(i, _)| *i);
        Ok(Value::String(s[byte_start..byte_end].to_string()))
    } else {
        Ok(Value::String(s[byte_start..].to_string()))
    }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`unique()` has O(n^2) complexity with no size bound** - `crates/mds-core/src/builtins.rs:436-441`
**Confidence**: 80%
- Problem: `builtin_unique` uses `result.contains(item)` inside a loop, giving O(n^2) behavior. For arrays approaching the implicit size limit (derived from MAX_FILE_SIZE ~10MB), this could become very slow. The `contains` call on `Vec<Value>` does a linear scan each iteration.
- Fix: For a template language this is acceptable for small arrays, but consider either (a) adding a length check that rejects arrays above a reasonable threshold (e.g. 10,000 elements), or (b) using a `HashSet`-based approach if `Value` implements `Hash`. Since `Value` contains `f64` (which is not `Hash`), option (a) is simpler:
```rust
if arr.len() > 10_000 {
    return Err(MdsError::builtin_error(
        "unique() array exceeds maximum size of 10,000 elements"
    ));
}
```

## Pre-existing Issues (Not Blocking)

No pre-existing CRITICAL issues identified in unchanged code.

## Suggestions (Lower Confidence)

- **`split("")` can produce N+2 elements from an N-byte string** - `crates/mds-core/src/builtins.rs:222` (Confidence: 65%) -- Splitting on empty string is a well-known edge case. With MAX_FILE_SIZE of 10MB, this could produce ~10M Value::String allocations. Consider clamping the split result or rejecting empty separator.

- **`replace()` with empty `from` string replaces between every character** - `crates/mds-core/src/builtins.rs:216` (Confidence: 60%) -- `s.replace("", to)` inserts `to` before every character and at the end. For a 10MB string with a non-empty `to`, this could produce enormous output. The MAX_OUTPUT_SIZE (50MB) in the evaluator would catch this downstream, but the intermediate allocation could be large.

- **`Condition::path()` returns empty slice for `And`/`Or` variants** - `crates/mds-core/src/ast.rs:71` (Confidence: 70%) -- Returning `&[]` for compound conditions is a silent fallback that could mislead callers into thinking the condition has no path. Returning `Option<&[String]>` would force callers to handle the compound case explicitly. However, the `root()` method already returns `Err` for compound conditions, and the only caller of `path()` appears to be `root()` itself, so this is low-risk.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR delivers a well-structured set of language features (applies ADR-008) with good test coverage (690 tests passing, clippy clean). The AST extensions, parser changes, evaluator updates, and validator mirroring are all consistent and well-documented. The `snap_to_char_boundary` fix shows awareness of UTF-8 safety.

The primary concern is the byte-vs-character semantics inconsistency across string builtins: `length()` returns byte count while `reverse()` operates on characters, and `slice()` uses byte indices (with snap-back). This creates a confusing API surface for template authors. The `sort()` NaN ordering and the `expect()` panic in library code are secondary concerns. None of these are data-loss or security risks, but they represent correctness and API consistency issues that should be addressed before merge.

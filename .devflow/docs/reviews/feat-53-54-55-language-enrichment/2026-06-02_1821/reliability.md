# Reliability Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**replace() can amplify output size without bound** - `crates/mds-core/src/builtins.rs:237`
**Confidence**: 82%
- Problem: `builtin_replace` calls Rust's `str::replace(from, to)` with no guard on the output length. If `from` is a single character and `to` is a long string, and the input string is close to `MAX_FILE_SIZE` (10 MB) with many occurrences, the result can far exceed the original string size. For example, `replace(s, "a", "<very long string>")` on a 10 MB string of `a`s could produce gigabytes of output. The `MAX_OUTPUT_SIZE` (50 MB) limit in the evaluator only guards the final accumulated output buffer, not intermediate `Value::String` allocations inside built-in functions. A single `replace()` call could allocate enough memory to OOM the process before the output-size guard fires.
- Fix: Add a post-replacement size guard in `builtin_replace`:
```rust
fn builtin_replace(args: &[Value]) -> Result<Value, MdsError> {
    let s = require_string_at(args, 0, "replace", "first")?;
    let from = require_string_at(args, 1, "replace", "second")?;
    let to = require_string_at(args, 2, "replace", "third")?;
    if from.is_empty() {
        return Err(MdsError::builtin_error(
            "replace() search string must not be empty",
        ));
    }
    let result = s.replace(from, to);
    if result.len() > MAX_OUTPUT_SIZE {
        return Err(MdsError::resource_limit(
            "replace() result exceeds maximum output size",
        ));
    }
    Ok(Value::String(result))
}
```

**reverse() breaks multi-codepoint grapheme clusters** - `crates/mds-core/src/builtins.rs:397`
**Confidence**: 83%
- Problem: `builtin_reverse` reverses by Unicode scalar values (`s.chars().rev()`), which correctly handles single-codepoint characters but breaks multi-codepoint grapheme clusters. For example, reversing a string with combining diacriticals (e.g., `"e\u{0301}"` = e with acute accent) swaps the combining mark to precede a different base character, producing corrupted output. Similarly, flag emoji like the regional indicator sequences will break. This is a data corruption issue, not a crash, but it violates the principle of least surprise for a user-facing string function.
- Fix: Document this limitation explicitly in the spec or use the `unicode-segmentation` crate for grapheme-aware reversal:
```rust
// Option 1: Document the limitation in spec.md and add a comment
// "reverse() reverses by Unicode scalar values. Multi-codepoint
//  grapheme clusters (combining marks, flag emoji) may be reordered."

// Option 2: Use unicode-segmentation
use unicode_segmentation::UnicodeSegmentation;
fn builtin_reverse(args: &[Value]) -> Result<Value, MdsError> {
    match &args[0] {
        Value::String(s) => {
            let reversed: String = s.graphemes(true).rev().collect();
            Ok(Value::String(reversed))
        }
        // ... array arm unchanged
    }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**debug_assert for And/Or nesting invariant is invisible in release builds** - `crates/mds-core/src/evaluator.rs:418-420`
**Confidence**: 80%
- Problem: The `debug_assert!` at lines 418-420 and 433-434 guards parser invariants (And operands should be leaf, Or operands should not be Or). In release builds, `debug_assert!` is a no-op, so if a future grammar change or programmatic AST construction violates this invariant, `evaluate_condition` will silently recurse without any warning or bound. While the parser currently guarantees max depth 2 (Or -> And -> leaf), and the leaf count is bounded to 16, the `debug_assert` provides no protection in production. The recursion itself is safe today (bounded by structure), but the assertion's intent -- catching invariant violations -- is defeated in release mode.
- Fix: Keep the `debug_assert!` for development but also add a brief depth parameter or consider promoting to a production assertion for the nesting invariant:
```rust
// Option 1: Accept the current risk (depth is bounded by parser structure)
// and document that the debug_assert is a dev-only canary. No change needed.

// Option 2: Add a defensive depth bound
fn evaluate_condition_inner(condition: &Condition, scope: &Scope, depth: usize) -> Result<bool, MdsError> {
    if depth > MAX_LOGICAL_OPERANDS {
        return Err(MdsError::syntax("condition evaluation depth exceeded"));
    }
    // ... existing match with depth + 1 on recursive calls
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**call_stack recursion detection uses O(n) linear scan** - `crates/mds-core/src/evaluator.rs:265`
**Confidence**: 85%
- Problem: As documented in the feature knowledge (gotcha: "call_stack is Vec, not HashSet -- recursion detection uses O(n) scan at MAX_CALL_DEPTH=128"), the call stack is a `Vec<String>` with `.iter().any(|s| s == call_key)` for recursion detection. At MAX_CALL_DEPTH=128, each function call scans up to 128 entries. This is O(n^2) in the depth of indirect call chains. Not a production issue at current bounds, but noted for awareness.

## Suggestions (Lower Confidence)

- **split() output size is unbounded** - `crates/mds-core/src/builtins.rs:248` (Confidence: 70%) -- `split()` with a single-character separator on a large string produces an array with as many elements as there are separator occurrences. A 10 MB string of commas could produce ~10 million single-element strings. The existing `MAX_OUTPUT_SIZE` does not bound intermediate `Value::Array` allocations. Lower confidence because the input string is already bounded by `MAX_FILE_SIZE`.

- **unique_key uses Display for Array/Object equality** - `crates/mds-core/src/builtins.rs:486-487` (Confidence: 65%) -- `unique()` uses `format!("a:{v}")` and `format!("o:{v}")` for Array and Object deduplication keys. Two structurally different arrays/objects could theoretically produce the same Display output if their elements happen to format identically with different nesting. This is an edge case unlikely in practice.

- **number() accepts "Infinity"/"-Infinity" strings from trim().parse()** - `crates/mds-core/src/builtins.rs:519` (Confidence: 60%) -- Rust's `f64::parse` accepts "inf" and "infinity" (case-insensitive). The subsequent `is_finite()` check at line 522 correctly rejects these, so this is not a bug, but the error message "produced a non-finite value from 'inf'" could be confusing to users who may not understand why their string input is rejected. Consider adding a pre-parse check or a more specific error message.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The code demonstrates strong reliability practices overall:
- All new limits (`MAX_LOGICAL_OPERANDS`) are properly enforced at parse time
- The `count_leaf_operands` recursive counter correctly bounds condition tree complexity
- `require_number_index` properly handles NaN, infinity, negative values, and overflow
- `sort()` validates homogeneity and finiteness before cloning (allocation discipline)
- `unique()` uses O(n) HashSet-based deduplication instead of O(n^2)
- `replace()` and `split()` guard against empty separator/search strings
- `slice()` clamps indices to collection bounds (no panics)
- Parser structure guarantees that `And`/`Or` conditions have bounded recursion depth (max 2 levels)
- `invoke_function` properly handles the arity range for optional params with defensive internal error

The two MEDIUM blocking issues (replace amplification and reverse grapheme handling) are genuine but low-probability in typical prompt template workloads. They should be addressed before v0.2.0 release but are not merge-blocking for the feature branch.

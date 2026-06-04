# Reliability Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02
**PR**: #70 (applies ADR-008 -- bundle related language features)

## Issues in Your Changes (BLOCKING)

### HIGH

**`require_number_index` converts f64 to usize without overflow guard** - `crates/mds-core/src/builtins.rs:295`
**Confidence**: 90%
- Problem: `n.max(0.0).floor() as usize` performs a saturating-but-undefined cast for values exceeding `usize::MAX` (approx 1.8e19). On current Rust (1.45+), `as usize` on out-of-range f64 saturates to `usize::MAX`, which when used as a slice index causes the subsequent `.min(len)` clamp to save correctness. However, `f64::NAN.max(0.0)` returns `0.0` (correct) and `f64::INFINITY.max(0.0).floor()` returns `inf` -- casting `inf as usize` yields `usize::MAX` which then clamps to `len`. The practical outcome is safe today due to the `.min(len)` / `.clamp(start, len)` in `builtin_slice`, but the cast itself is a footgun: if any caller uses the result without a bounds clamp, it silently produces `usize::MAX`. A defensive clamp or explicit check before the cast would make the safety guarantee self-contained rather than relying on downstream code.
- Fix: Add an explicit upper bound before the cast:
  ```rust
  fn require_number_index(val: &Value, fn_name: &str, pos: &str) -> Result<usize, MdsError> {
      match val {
          Value::Number(n) => {
              let clamped = n.max(0.0).floor();
              if !clamped.is_finite() || clamped > usize::MAX as f64 {
                  return Err(type_err(fn_name, pos, "a finite number", "infinity or NaN"));
              }
              Ok(clamped as usize)
          }
          other => Err(type_err(fn_name, pos, "number", other.type_name())),
      }
  }
  ```

**`sort()` with NaN values produces non-deterministic ordering** - `crates/mds-core/src/builtins.rs:413-416`
**Confidence**: 85%
- Problem: `partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)` treats NaN as equal to everything. Since `sort_by` requires a total order, using `Equal` for NaN comparisons violates the transitivity requirement (NaN == 1.0 and NaN == 2.0 but 1.0 != 2.0), which can cause non-deterministic output from `sort_by` depending on the sort algorithm's pivot selection. While the parser rejects NaN literals, `number()` conversion could theoretically produce NaN (though the `is_finite()` check blocks that path). The risk is low but the code does not reject NaN in the input array -- it only checks type homogeneity, not finiteness. If a NaN enters the array through future runtime arithmetic, sorting becomes non-deterministic.
- Fix: Reject or filter NaN values before sorting:
  ```rust
  Value::Number(_) => {
      for item in &sorted {
          match item {
              Value::Number(n) if !n.is_finite() => {
                  return Err(MdsError::builtin_error(
                      "sort() cannot sort array containing NaN or infinity".to_string()
                  ));
              }
              Value::Number(_) => {}
              _ => return Err(MdsError::builtin_error(format!(
                  "sort() requires a homogeneous array; found {} mixed with number",
                  item.type_name()
              ))),
          }
      }
      sorted.sort_by(|a, b| match (a, b) {
          (Value::Number(a), Value::Number(b)) => a.partial_cmp(b).unwrap(),
          _ => unreachable!(),
      });
  }
  ```

### MEDIUM

**`unique()` has O(n^2) complexity with no size bound** - `crates/mds-core/src/builtins.rs:436-441`
**Confidence**: 82%
- Problem: `builtin_unique` uses `result.contains(item)` inside a loop over the array, giving O(n^2) time complexity. Since `Value` does not implement `Hash`, a `HashSet`-based approach is not trivially available. For MDS template use cases, arrays are typically small. However, there is no size limit on the input array, and a sufficiently large array (e.g., 100K+ elements from a large data source) could cause noticeable CPU hang. The existing `MAX_OUTPUT_SIZE` (50MB) and `MAX_FILE_SIZE` (10MB) provide indirect bounds, but a direct input-size assertion would make the bound explicit.
- Fix: Add an assertion at the top of the function:
  ```rust
  fn builtin_unique(args: &[Value]) -> Result<Value, MdsError> {
      let arr = match &args[0] {
          Value::Array(a) => a,
          other => return Err(type_err("unique", "", "array", other.type_name())),
      };
      if arr.len() > 10_000 {
          return Err(MdsError::builtin_error(
              "unique() input exceeds 10,000 elements".to_string()
          ));
      }
      // ... rest unchanged
  }
  ```

**`length()` returns byte length for strings, not character count** - `crates/mds-core/src/builtins.rs:341`
**Confidence**: 80%
- Problem: `s.len() as f64` returns the byte length of the string, not the character count. For ASCII strings these are identical, but for multi-byte UTF-8 content (e.g. `"cafe\u{0301}"` is 6 bytes but 5 chars, `"hello"` in Chinese `"\u{4f60}\u{597d}"` is 6 bytes but 2 chars), the result may surprise users. The spec says "String byte length or array count" (spec.md line 260), so this is spec-conformant, but the spec itself may be a reliability concern since users calling `length("cafe\u{0301}")` and getting 6 instead of 5 is non-obvious. This is not a bug per se but a documented behavior that may lead to surprising results in multi-byte contexts, especially combined with `slice()` which also operates on byte indices.
- Fix: Either document prominently that `length()` returns byte count (not char count) in error messages/warnings, or consider adding a `char_length()` builtin in a future version. If byte-length is the intended design, no code change needed -- but the spec line 260 should be more explicit: "String **byte** length" (already present, but could be bolded for emphasis).

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`evaluate_condition` recursive dispatch has no depth bound** - `crates/mds-core/src/evaluator.rs:412-428`
**Confidence**: 80%
- Problem: `evaluate_condition` recursively dispatches for `And`/`Or` variants. While the parser limits leaf operands to 16 (`MAX_LOGICAL_OPERANDS`), the condition tree depth is bounded to 2 levels by the grammar (Or wrapping And wrapping leaf). However, the evaluator itself has no independent assertion that the tree is shallow. If future grammar changes allow nested `(a || b) && (c || d)` producing deeper trees, the evaluator would recurse without an explicit bound. Today this is safe because the parser constrains the structure, but adding a depth parameter would make the invariant self-documenting.
- Fix: Add a `depth` parameter with a static limit, or add a `debug_assert!` on the match arm that the operands are always leaf conditions (not further nested):
  ```rust
  Condition::And(operands) => {
      for operand in operands {
          debug_assert!(!matches!(operand, Condition::And(_) | Condition::Or(_)),
              "grammar invariant: And operands should be leaf conditions");
          if !evaluate_condition(operand, scope)? {
              return Ok(false);
          }
      }
      Ok(true)
  }
  ```

## Pre-existing Issues (Not Blocking)

No critical pre-existing reliability issues found in unchanged code within reviewed files.

## Suggestions (Lower Confidence)

- **`split()` on empty separator produces unbounded output** - `crates/mds-core/src/builtins.rs:222` (Confidence: 70%) -- `s.split("")` returns an iterator that yields one element per character plus empty strings at boundaries. For a 10MB string (the file-size limit), this could produce millions of array elements. Consider rejecting empty separators.

- **`replace()` with empty `from` string** - `crates/mds-core/src/builtins.rs:216` (Confidence: 65%) -- `s.replace("", to)` inserts `to` between every character plus at boundaries, which could produce output 2x+ the input size. The existing `MAX_OUTPUT_SIZE` guard provides an indirect bound, but the intermediate allocation could be large.

- **`builtin_reverse` on multi-byte strings reverses by char, not grapheme cluster** - `crates/mds-core/src/builtins.rs:363` (Confidence: 60%) -- `s.chars().rev().collect()` reverses Unicode code points, which produces incorrect results for strings with combining characters (e.g. `"e\u{0301}"` reversed becomes `"\u{0301}e"` instead of keeping the accent on the 'e'). This is a correctness nuance rather than a crash risk.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR demonstrates strong reliability discipline overall: resource limits are enforced for logical operands (`MAX_LOGICAL_OPERANDS=16`), recursion is depth-bounded, call stacks are LIFO-checked, UTF-8 char-boundary safety is explicitly handled in `slice()`, and all builtins use `Result` types consistently. The two HIGH findings are defensive hardening improvements -- the code works correctly today but relies on downstream invariants (`min(len)` clamps, NaN rejection at parse time) rather than being self-contained. The MEDIUM findings address quadratic complexity in `unique()` and a missing depth assertion in `evaluate_condition`.

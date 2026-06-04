# Performance Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Double linear scan in built-in dispatch** - `evaluator.rs:343-352`
**Confidence**: 85%
- Problem: `call_function` calls `crate::builtins::get_builtin(name)` to check arity (line 343), then calls `crate::builtins::call_builtin(name, args)` (line 352), which internally calls `get_builtin(name)` again (builtins.rs:173). This performs the linear scan of the 18-element `BUILTINS` array twice per built-in invocation.
- Impact: At 18 entries the wall-clock cost is negligible, but when a built-in is called inside a `@for` loop (up to 100,000 iterations), this doubles the lookup overhead unnecessarily. More importantly it is a missed optimization that becomes relevant as the built-in count grows.
- Fix: Use the `meta` reference already obtained from the first lookup to call the handler directly, eliminating the second scan:
  ```rust
  if let Some(meta) = crate::builtins::get_builtin(name) {
      if args.len() < meta.min_args || args.len() > meta.max_args {
          return Err(MdsError::arity(name, meta.min_args, meta.max_args, args.len()));
      }
      return (meta.handler)(args); // direct dispatch, no second lookup
  }
  ```

### MEDIUM

**`unique_key` generates O(m) string keys for nested arrays/objects** - `builtins.rs:476-489`
**Confidence**: 82%
- Problem: For `Value::Array` and `Value::Object` elements, `unique_key` calls `format!("a:{v}")` / `format!("o:{v}")`, which invokes `Value::Display`. For an array of m elements, this produces a string of O(m) length. When `builtin_unique` is called on an array of n elements, each containing a nested array of size m, the total string allocation is O(n*m) and each `HashSet::insert` compares keys of length O(m), making overall complexity O(n*m) rather than the claimed O(n).
- Impact: For flat arrays of scalars (the common case), complexity is genuinely O(n). But for arrays of nested structures, this degrades. Given the MAX_LOOP_ITERATIONS=100,000 bound, adversarial input could trigger significant string allocation. The input file is capped at 10 MB, which bounds practical damage.
- Fix: No code change required for v0.2 since the 10 MB file size limit bounds the total data. Add a doc-comment noting the complexity caveat for nested values:
  ```rust
  /// NOTE: For arrays/objects, key generation is O(m) per element where m is
  /// the element's Display length. Total complexity for nested values is O(n*m),
  /// not O(n). Bounded in practice by MAX_FILE_SIZE.
  ```

**`join` clones all strings before joining** - `builtins.rs:358-368`
**Confidence**: 80%
- Problem: `builtin_join` iterates the array twice in effect: first to clone each `String` into a `Vec<String>` (via `.collect()`), then to join them with `Vec::join`. This doubles memory usage for the string data.
- Impact: For a 100,000-element array of strings (the loop iteration limit), this doubles transient string allocation. The 50 MB output limit bounds practical damage, but the extra allocation is unnecessary.
- Fix: Use itertools or a manual fold to avoid the intermediate `Vec<String>`:
  ```rust
  let mut out = String::new();
  for (i, v) in arr.iter().enumerate() {
      if i > 0 { out.push_str(sep); }
      match v {
          Value::String(s) => out.push_str(s),
          other => return Err(MdsError::builtin_error(format!(
              "join() requires an array of strings, but found {} in array",
              other.type_name()
          ))),
      }
  }
  Ok(Value::String(out))
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`call_stack` uses O(n) linear scan for recursion detection** - `evaluator.rs:34,265`
**Confidence**: 85%
- Problem: `ctx.call_stack` is a `Vec<String>`, and recursion detection uses `call_stack.iter().any(|s| s == call_key)` which is O(n) where n is the call depth. At MAX_CALL_DEPTH=128, this means up to 128 string comparisons per function call.
- Impact: This is pre-existing code (not changed in this PR). With MAX_CALL_DEPTH=128, the O(n) scan is acceptable. The comment at line 34 explicitly acknowledges and accepts this tradeoff. A `HashSet` would improve asymptotic performance but add allocation overhead that is not justified at this scale.
- Note: Feature knowledge gotcha confirms this is an intentional design choice: "call_stack is Vec, not HashSet -- recursion detection uses O(n) scan at MAX_CALL_DEPTH=128."

## Suggestions (Lower Confidence)

- **`builtin_slice` for strings iterates chars twice** - `builtins.rs:286-301` (Confidence: 70%) -- `s.chars().count()` iterates the entire string to get `char_count`, then `s.chars().skip(start_idx).take(end_idx - start_idx)` iterates again from the beginning. For large strings near the 10 MB file size limit, a single-pass approach using `char_indices` would halve the iteration cost.

- **`builtin_sort` validates homogeneity then clones** - `builtins.rs:420-463` (Confidence: 65%) -- The homogeneity check iterates the array, then `arr.to_vec()` clones the entire array. These could be combined into a single pass that validates while cloning. However, the current approach avoids cloning on type-error inputs (the comment at line 420-422 documents this as intentional), so this is a deliberate correctness-over-perf tradeoff.

- **No array size limit on built-in return values** - `builtins.rs` (Confidence: 60%) -- `split()` can produce an unbounded number of array elements from a single call. A 10 MB string split on a 1-byte separator yields ~10 million elements. The existing MAX_OUTPUT_SIZE (50 MB) and MAX_FILE_SIZE (10 MB) provide indirect bounds, but there is no direct guard on array length in built-ins. This is mitigated by the file size cap making input bounded.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The performance characteristics of this PR are solid for a template compiler. The built-in functions correctly use character-based (not byte-based) string operations, the `unique` function uses O(n) HashSet-based deduplication, and `sort` validates before cloning. The existing resource limits (MAX_FILE_SIZE=10MB, MAX_OUTPUT_SIZE=50MB, MAX_LOOP_ITERATIONS=100K, MAX_TOTAL_ITERATIONS=1M) effectively bound all built-in operations.

The one actionable blocking item is the double linear scan in built-in dispatch -- a simple fix that avoids redundant work on every built-in call. The two MEDIUM blocking items (`unique_key` complexity and `join` double-allocation) are worth addressing but not merge-blocking given the existing resource limits.

Applies ADR-008 (bundled language features in single PR). The linear scan of the 18-element BUILTINS array is explicitly documented as intentional and appropriate at this cardinality (builtins.rs:38-41).

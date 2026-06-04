# Performance Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Value cloning in `resolve_dot_path` traversal** - `src/evaluator.rs:101-108`
**Confidence**: 82%
- Problem: `resolve_dot_path` clones the root value on line 103 (`.cloned()`) and then clones each intermediate field on line 108 (`map.get(field).cloned()`). For deeply nested objects (`a.b.c.d`), each traversal step clones the entire sub-tree at that level. If an intermediate object is large (many keys with nested values), this clones the full HashMap including all its children — only to immediately discard the parent container on the next iteration.
- Impact: For shallow paths (1-2 fields) on small objects this is negligible. For paths traversing large intermediate objects (hundreds of keys with nested sub-objects), each step allocates and drops a full HashMap clone. Given the template compiler domain (frontmatter YAML configs), objects are typically small (< 50 keys), so this is unlikely to be a real bottleneck in practice.
- Fix: Restructure to avoid intermediate clones by using references until the final value is needed. Only clone at the terminal value:
  ```rust
  fn resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError> {
      let root_val = scope
          .get_var(root)
          .ok_or_else(|| MdsError::undefined_var(root))?;
      let mut current = root_val;
      for field in fields {
          match current {
              Value::Object(map) => {
                  current = map.get(field).ok_or_else(|| {
                      MdsError::syntax(format!("field '{field}' not found on object '{root}'"))
                  })?;
              }
              _ => {
                  return Err(MdsError::syntax(format!(
                      "cannot access field '{field}' on {}",
                      current.type_name()
                  )));
              }
          }
      }
      Ok(current.clone())  // single clone at the end
  }
  ```
  This requires changing the signature to work with `&Value` references internally. The borrow checker should allow this since `scope` is immutable in this function.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`format!` allocation in `prepend_frontmatter`** - `src/lib.rs:374` (Confidence: 65%) — The `format!("---\n{cleaned}---\n{body}")` allocates a new string combining cleaned frontmatter and body. For large body strings this creates a full copy. A `String::with_capacity` + push_str chain would avoid the intermediate formatting allocation and give exact capacity. However, `format!` is likely optimized by the compiler and frontmatter is small relative to body, so the practical impact is minimal.

- **Sorted key collection in `Value::Object` Display** - `src/value.rs:236-237` (Confidence: 62%) — `Value::Object` Display sorts keys on every call by collecting all keys into a Vec and sorting. If the same object is displayed repeatedly (e.g., in a loop that interpolates a field whose parent is Object — though this is currently blocked by the object-interpolation guard), the sort would be redundant. Since direct object interpolation is disallowed (produces an error), this code path only executes in diagnostic/debug contexts, making it a non-issue in practice.

- **Key-value iteration sorts then iterates** - `src/evaluator.rs:363-364` (Confidence: 70%) — `evaluate_for` collects the HashMap into a Vec of tuples and sorts by key. This is O(n log n) per object iteration. For deterministic output this is necessary and correct. For very large objects (thousands of keys), pre-sorting at parse/construction time (e.g., using BTreeMap instead of HashMap for `Value::Object`) would amortize the cost. However, given the 100,000 iteration limit and typical frontmatter sizes (< 100 keys), the current approach is adequate for the use case.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The performance characteristics of this branch are solid for the domain (template compilation with small-to-medium frontmatter configs). Key positives:

1. **The prior `Vec` allocation in `resolve_dot_path` was already eliminated** (commit `be8fde3`) — the function now takes `&[String]` slices rather than allocating a new Vec.
2. **Resource limits are properly enforced** — `MAX_LOOP_ITERATIONS` applies to both array and key-value object iteration, preventing runaway sorting costs.
3. **`strip_type_mds` pre-allocates with capacity** — `String::with_capacity(raw.len())` avoids reallocations.
4. **`from_yaml`/`from_json` depth limits** prevent stack overflow from pathological inputs.

The single MEDIUM finding (intermediate cloning in `resolve_dot_path`) is a potential optimization opportunity but not a blocking concern given the expected input sizes. The condition for approval is: acknowledge the clone-per-field behavior is acceptable for v0.1 (small configs), or refactor to reference-based traversal if larger objects are anticipated.

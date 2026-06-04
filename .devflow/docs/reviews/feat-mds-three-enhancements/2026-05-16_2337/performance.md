# Performance Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**String allocation in `resolve_dot_path` error path tracking** - `src/evaluator.rs:112`
**Confidence**: 82%
- Problem: `path_so_far` is allocated as `root.to_string()` on every call to `resolve_dot_path`, even when no error occurs. For the common case (successful traversal of 1-2 segments), this heap allocation is wasted. This function is called in four hot paths: interpolation, `@if`, `@for`, and argument resolution.
- Fix: Use a lazy approach — only build the path string when an error occurs. This avoids the allocation on the happy path:
  ```rust
  fn resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError> {
      if fields.len() > MAX_DOT_SEGMENTS {
          return Err(MdsError::syntax(format!(
              "dot path depth exceeds maximum of {MAX_DOT_SEGMENTS} segments"
          )));
      }
      let mut current = scope
          .get_var(root)
          .cloned()
          .ok_or_else(|| MdsError::undefined_var(root))?;
      for (i, field) in fields.iter().enumerate() {
          match current {
              Value::Object(ref map) => {
                  current = map.get(field).cloned().ok_or_else(|| {
                      let path_so_far = std::iter::once(root)
                          .chain(fields[..i].iter().map(|s| s.as_str()))
                          .collect::<Vec<_>>()
                          .join(".");
                      MdsError::syntax(format!(
                          "field '{field}' not found on '{path_so_far}'"
                      ))
                  })?;
              }
              _ => {
                  let path_so_far = std::iter::once(root)
                      .chain(fields[..i].iter().map(|s| s.as_str()))
                      .collect::<Vec<_>>()
                      .join(".");
                  return Err(MdsError::syntax(format!(
                      "cannot access field '{field}' on {} '{path_so_far}'",
                      current.type_name()
                  )));
              }
          }
      }
      Ok(current)
  }
  ```

**`evaluate_for_key_value` takes `HashMap` by value, immediately collects to Vec** - `src/evaluator.rs:368-383`
**Confidence**: 80%
- Problem: `evaluate_for_key_value` accepts `map: HashMap<String, Value>` by value, then immediately calls `map.into_iter().collect()` into a `Vec<(String, Value)>` and sorts it. The `HashMap` is consumed from the `Value::Object(m)` match arm above, so the move is zero-cost. However, the intermediate `Vec` allocation and sort could be avoided by using a `BTreeMap` for `Value::Object` directly — the KNOWLEDGE.md notes that object iteration uses sorted keys, and `Display` for objects already sorts alphabetically. Using `BTreeMap` throughout would eliminate the per-iteration sort cost.
- Fix: This is a design-level suggestion that would touch `Value::Object`. For this PR, the current approach is acceptable for typical object sizes (most YAML frontmatter objects have < 20 keys). The sort is O(n log n) and bounded by `MAX_LOOP_ITERATIONS`. No immediate fix required, but note for future optimization: switching `Value::Object` from `HashMap<String, Value>` to `BTreeMap<String, Value>` would eliminate sorting at iteration and display time.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`.cloned()` on HashMap value lookup in `resolve_dot_path`** - `src/evaluator.rs:116` (Confidence: 70%) — `map.get(field).cloned()` clones the `Value` at each dot-path segment traversal. For deeply nested objects with large `Value::Array` or `Value::Object` fields at intermediate levels, this clones the entire subtree at each step. A reference-based traversal (`&Value`) would avoid intermediate clones, only cloning the final leaf. However, this would require rethinking the function signature and borrow relationships.

- **`strip_type_mds` scans all lines on every compile** - `src/lib.rs:343-366` (Confidence: 65%) — For large frontmatter blocks (many YAML keys), `strip_type_mds` iterates every line and rebuilds a new string. This runs on every compilation. The cost is negligible for typical frontmatter (< 20 lines), but could matter if frontmatter grows large. The `filtered.trim().is_empty()` call at the end also scans the full filtered string. Low concern given realistic input sizes.

- **`run_loop_body` allocates a scope frame per iteration** - `src/evaluator.rs:349-362` (Confidence: 62%) — Each loop iteration calls `scope.push()` and `scope.pop()`, allocating and deallocating a `Frame` (with three `HashMap`s) per iteration. For large arrays (up to 100,000 items), this creates significant allocation pressure. A pre-allocated frame that gets cleared per iteration would reduce allocator traffic. This is pre-existing behavior that was merely refactored into a helper.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED

The changes are well-structured from a performance perspective. The new `resolve_dot_path` traversal is O(depth) with depth bounded by `MAX_DOT_SEGMENTS=32`. The key-value iteration sorts alphabetically which is O(n log n) but bounded by `MAX_LOOP_ITERATIONS=100,000`. The `strip_type_mds` enhancement adds two extra string comparisons per line (for quoted variants) which is negligible. The only actionable finding is the `path_so_far` allocation on every `resolve_dot_path` call — this is a minor hot-path optimization that could be deferred. No blocking performance issues.

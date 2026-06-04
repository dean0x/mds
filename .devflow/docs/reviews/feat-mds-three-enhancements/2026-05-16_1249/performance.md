# Performance Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Redundant Vec allocation on every MemberAccess evaluation** - `src/evaluator.rs:164-166`, `src/evaluator.rs:207-209`
**Confidence**: 85%
- Problem: Both `evaluate_expr(Expr::MemberAccess)` and `resolve_args(Arg::MemberAccess)` allocate a new `Vec<String>` on every call by cloning the `object` string and all `fields` strings into a fresh Vec, solely to pass it to `resolve_dot_path`. This allocation occurs on every interpolation or argument resolution involving dot-notation access, which in a loop iterating over object entries could add up to significant allocation pressure.
- Fix: Refactor `resolve_dot_path` to accept the root name and fields slice separately, avoiding the intermediate Vec allocation:
```rust
fn resolve_dot_path_parts(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError> {
    let mut current = scope
        .get_var(root)
        .cloned()
        .ok_or_else(|| MdsError::undefined_var(root))?;
    for field in fields {
        match current {
            Value::Object(ref map) => {
                current = map.get(field).cloned().ok_or_else(|| {
                    MdsError::syntax(format!("field '{field}' not found on object '{root}'"))
                })?;
            }
            _ => {
                return Err(MdsError::syntax(format!(
                    "cannot access field '{field}' on {}", current.type_name()
                )));
            }
        }
    }
    Ok(current)
}
```
Then call as `resolve_dot_path_parts(object, fields, scope)` from `MemberAccess` arms and `resolve_dot_path(&path[0], &path[1..], scope)` from `evaluate_if`/`evaluate_for`. This eliminates two allocation sites per dot-notation evaluation.

### MEDIUM

**`strip_type_mds` creates a per-line `format!` allocation** - `src/lib.rs:351`
**Confidence**: 82%
- Problem: The line `.map(|line| format!("{line}\n"))` allocates a new String for every line of frontmatter content just to append a newline. While frontmatter is typically small (< 20 lines), this is an unnecessary allocation pattern.
- Fix: Use `push_str` into a pre-sized buffer instead of per-line format allocations:
```rust
fn strip_type_mds(raw: &str) -> Option<String> {
    let mut filtered = String::with_capacity(raw.len());
    for line in raw.lines() {
        if !line.trim().strip_prefix("type:").is_some_and(|v| v.trim() == "mds") {
            filtered.push_str(line);
            filtered.push('\n');
        }
    }
    if filtered.trim().is_empty() {
        None
    } else {
        Some(filtered)
    }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Deep clone of `Value` on every dot-path resolution step** - `src/evaluator.rs:102-113`
**Confidence**: 82%
- Problem: `resolve_dot_path` clones the root value from scope (line 104: `.cloned()`) and then clones each intermediate value during traversal (line 109: `map.get(field).cloned()`). For deeply nested objects with large sub-trees, each `.cloned()` copies the entire sub-tree. A 3-level path `a.b.c` where `b` is a large object with many entries will clone the full `b` object just to extract field `c` from it.
- Fix: This is a known consequence of Rust's ownership model with the current `Scope` borrow patterns. The immediate impact is bounded by `MAX_VALUE_DEPTH = 64`, and typical MDS templates have shallow objects. A future optimization could use `Cow` or return references where the borrow checker allows. No change required for this PR, but worth noting as a scaling concern if objects become large.

## Pre-existing Issues (Not Blocking)

No pre-existing performance issues of CRITICAL or HIGH severity were identified in the reviewed files.

## Suggestions (Lower Confidence)

- **Object key sorting in `evaluate_for` and `Display`** - `src/evaluator.rs:366-367`, `src/value.rs:214-215` (Confidence: 70%) -- Both the key-value `@for` loop and `Value::Object` Display sort keys on every invocation. For the `@for` loop, sorting is necessary for deterministic output and happens once per loop. For Display, it runs every time an object is rendered as a string. If objects are frequently displayed, consider using `BTreeMap` instead of `HashMap` to maintain sorted order without repeated sorts. However, this is a design tradeoff (insert cost vs. iteration cost) and objects are unlikely to be displayed directly in practice (the evaluator blocks direct object interpolation).

- **`HashMap::new()` without capacity hint in `from_yaml`/`from_json`** - `src/value.rs:64`, `src/value.rs:104` (Confidence: 65%) -- When converting YAML mappings or JSON objects, `HashMap::new()` starts with zero capacity and grows through reallocations. Using `HashMap::with_capacity(mapping.len())` or `HashMap::with_capacity(obj.len())` would avoid rehashing for known-size inputs.

- **`prepend_frontmatter` uses `format!` for string concatenation** - `src/lib.rs:371` (Confidence: 62%) -- `format!("---\n{cleaned}---\n{body}")` allocates a new String combining the frontmatter and body. For large bodies, this doubles memory usage momentarily. Using `String::with_capacity` and manual `push_str` calls would be more efficient, though this runs once per compilation so the absolute impact is small.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The new features (object support, dot-notation access, frontmatter preservation) are implemented with appropriate resource limits (MAX_LOOP_ITERATIONS, MAX_VALUE_DEPTH, MAX_TOTAL_ITERATIONS) that prevent unbounded resource consumption. The algorithmic complexity of the core paths is sound: dot-path resolution is O(depth), key-value iteration is O(n log n) for the sort which is acceptable, and frontmatter stripping is O(lines). The two blocking items are allocation inefficiencies that create unnecessary garbage on hot paths. The HIGH-severity Vec allocation in MemberAccess evaluation is the most impactful -- it runs on every dot-notation access inside loops. Fixing these two items would make the performance fully clean.

# Performance Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**`Arc::new(f.clone())` on every function invocation rebuilds captured functions from owned data** - `src/evaluator.rs:178-179`
**Confidence**: 85%
- Problem: Each call to `invoke_function` iterates `func.captured.functions` and performs a deep clone of every `FunctionDef` (which itself contains `Vec<Node>` body, `CapturedScope` with nested HashMaps) only to immediately wrap it in a new `Arc`. This runs on every function call, not just at definition time. If a function is called N times (e.g., inside a `@for` loop iterating 100,000 items), this clones every captured function N times. The comment says "captured.functions are owned FunctionDef (not Arc) -- wrap in Arc for scope insertion", which reveals the root cause: `CapturedScope.functions` stores owned `FunctionDef` to break reference cycles, but the cost is paid per-invocation rather than being amortized.
- Fix: Consider storing `Arc<FunctionDef>` in `CapturedScope.functions` instead of owned `FunctionDef` (if the cycle concern documented in `scope.rs:9-11` can be addressed -- e.g., by using `Weak` for back-references or accepting the minor leak risk for the performance win). Alternatively, lazily construct the `Arc`-wrapped versions once at definition time and cache them in a parallel field, so repeated invocations share the same `Arc` instances.

```rust
// Current (per-invocation deep clone):
for (name, f) in &func.captured.functions {
    scope.set_function(name, Arc::new(f.clone()));
}

// Alternative: store pre-wrapped Arcs in CapturedScope
// (requires careful cycle analysis)
pub struct CapturedScope {
    pub functions_arc: HashMap<String, Arc<FunctionDef>>,  // pre-wrapped
    // ...
}
// Then at invocation:
for (name, f) in &func.captured.functions_arc {
    scope.set_function(name, Arc::clone(f));  // O(1) per entry
}
```

### MEDIUM

**Linear scan for recursion detection in `call_stack`** - `src/evaluator.rs:159`
**Confidence**: 82%
- Problem: `ctx.call_stack.iter().any(|s| s == call_key)` performs an O(n) linear scan on every function call, where n is the current call depth (up to MAX_CALL_DEPTH=128). While the comment at line 30 acknowledges this ("Vec is used for O(n) contains at MAX_CALL_DEPTH=128 -- acceptable"), the change from `debug_assert!` to `assert!` on line 196 adds a string comparison in release mode on the pop path too. For a template compiler this is unlikely to be a bottleneck in practice given the 128-depth bound, but it is worth noting as a known O(n) operation in a hot path.
- Fix: If profiling shows this matters, a parallel `HashSet<String>` alongside the `Vec` would give O(1) contains while preserving LIFO ordering. Given MAX_CALL_DEPTH=128, this is a minor concern -- the comment documents the trade-off well.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Full scope capture clones all visible symbols for every `@define`** - `src/resolver.rs:313-321`
**Confidence**: 80%
- Problem: Each `@define` calls `scope.get_all_namespaces()`, `scope.get_all_functions()`, and `scope.get_all_vars()`, each of which calls `collect_all()` which iterates all frames and clones every entry into a new HashMap. For N definitions in a module, this is O(N * S) where S is the total number of symbols in scope. When combined with the `(*v).clone()` on line 319 (which deep-clones each `FunctionDef` out of its `Arc`), modules with many definitions and large scopes will do substantial redundant work. This is pre-existing but the PR's closure capture changes (Arc + CapturedScope) make it more load-bearing.
- Fix: Consider capturing lazily or sharing scope snapshots between consecutive definitions when the scope has not changed between them. This is a deeper refactor and may be tracked as a future optimization.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`canonicalize_and_check` still runs two `canonicalize` syscalls on cache hits** - `src/resolver.rs:73-138, 170-176`
**Confidence**: 82%
- Problem: The decomposition of `validate_and_read_file` into `canonicalize_and_check` + `read_validated_file` is a genuine performance improvement (file I/O is skipped on cache hits). However, `canonicalize_and_check` still performs two `canonicalize()` syscalls (parent and full path) before the cache lookup at line 174. For deeply nested import graphs where the same module is imported many times, this means repeated syscalls for paths that are already cached. The improvement is real -- file reads are the expensive part -- but further optimization would move the cache check before canonicalization using a raw-path-to-canonical-path cache.
- Fix: No action needed for this PR. The current decomposition already eliminates the most expensive operation (file I/O) on cache hits. A future optimization could add a `raw_path -> canonical_path` lookup cache to eliminate the redundant syscalls.

## Suggestions (Lower Confidence)

- **`to_vec()` on iterable array in `evaluate_for`** - `src/evaluator.rs:276` (Confidence: 70%) -- `iterable.as_array().ok_or_else(...)?.to_vec()` clones the entire array before iteration. If the array is large (up to MAX_LOOP_ITERATIONS = 100,000 items), this is a non-trivial allocation. Borrowing the slice directly could avoid the clone, though this may require lifetime changes.

- **Test generates ~14KB YAML frontmatter in-memory** - `tests/integration.rs:3036-3044` (Confidence: 65%) -- The `exit_code_resource_limit` test builds 2,002 YAML array items via string concatenation in a loop. Using `String::with_capacity` or a single `format!` would reduce reallocations, though test performance is rarely critical.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR delivers a clear performance win by splitting `validate_and_read_file` into `canonicalize_and_check` + `read_validated_file`, eliminating file I/O on cache hits. The `IndexSet.pop()` change (O(1) vs O(n) `shift_remove`) in the resolver is also a sound improvement. The `Arc<ResolvedModule>` wrapping enables O(1) cache returns.

The main performance concern is the per-invocation deep clone of captured functions in `invoke_function` (line 178-179). In hot loops that call the same function many times, this creates significant allocation pressure. This should be addressed before merge or tracked as a known performance debt item with a clear issue.

The `debug_assert!` to `assert!` promotion in `invoke_function` (line 196) adds a negligible cost (one string comparison per function return) and is justified by the safety comment -- this is acceptable.

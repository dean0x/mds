# Performance Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**File read before cache check in `ModuleCache::resolve`** - `src/resolver.rs:162`
**Confidence**: 92%
- Problem: `validate_and_read_file(path)` reads the entire file from disk (up to 10 MB) and performs two `canonicalize()` syscalls before the cache lookup on line 165. On cache hits, all of that I/O is wasted. The previous code checked the cache immediately after canonicalization, before reading the file. In projects with deep import graphs where the same module is imported by many files, this turns O(1) cache hits into O(file-size) I/O operations per hit.
- Fix: Split `validate_and_read_file` into two phases: (1) canonicalize and perform security checks that don't require file content, then check the cache; (2) read the file content only on cache miss. For example:

```rust
pub fn resolve(&mut self, path: &Path, ...) -> Result<Arc<ResolvedModule>, MdsError> {
    let canonical = self.canonicalize_and_check_security(path)?;

    // Check cache BEFORE reading file content
    if let Some(cached) = self.modules.get(&canonical) {
        return Ok(Arc::clone(cached));
    }

    // Check for circular imports
    if self.resolving.contains(&canonical) { ... }

    // Only now read the file
    let (source, is_md) = self.read_validated_file(&canonical)?;
    // ... rest of resolution
}
```

### MEDIUM

**`evaluate_for` clones the entire iterable array via `.to_vec()`** - `src/evaluator.rs:265`
**Confidence**: 82%
- Problem: `iterable.as_array()...to_vec()` clones every `Value` in the array before iteration. For large arrays (up to 100,000 items as allowed by `MAX_LOOP_ITERATIONS`), this allocates and copies the full array contents even though the original `Value::Array` is borrowed from scope and only individual items need to be moved into the loop body's scope.
- Fix: Consider iterating over the borrowed slice directly and cloning individual items only when pushing them into the loop scope. This avoids the upfront bulk clone:

```rust
let items = iterable
    .as_array()
    .ok_or_else(|| MdsError::type_error(iterable.type_name()))?;

if items.len() > MAX_LOOP_ITERATIONS { ... }

let mut output = String::new();
for item in items {
    // item is &Value here; clone only when setting the loop var
    scope.push();
    scope.set_var(&block.var, item.clone());
    ...
}
```

Note: The `.to_vec()` is needed because `iterable` borrows from `scope` and `scope.push()` would conflict. A refactoring to extract the array before the mutable borrow (e.g., via `scope.get_var(...).and_then(Value::as_array).map(|s| s.to_vec())`) is what's currently done. The performance impact is proportional to array size and depends on `Value` clone cost (strings clone their heap allocation). For small arrays this is negligible; for large arrays near the 100k limit it could matter.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`shift_remove` on `IndexSet` is O(n) per call** - `src/resolver.rs:191`
**Confidence**: 85%
- Problem: `IndexSet::shift_remove` preserves insertion order by shifting all subsequent elements, making it O(n) where n is the number of elements after the removed one. Since the resolver always removes the *last* inserted element (LIFO unmark after `process_module` returns), `swap_remove_full` or `pop()` would be O(1). The `shift_remove` only matters for correctness when removing from the middle, but the resolve/unmark pattern is strictly LIFO.
- Fix: Replace `shift_remove` with `pop()` since the current code always removes the element that was most recently inserted (the current canonical path). Add a debug assertion to verify the LIFO invariant:

```rust
// Unmark regardless of success or failure.
// pop() is O(1) — safe because resolve/unmark is strictly LIFO.
let popped = self.resolving.pop();
debug_assert!(
    popped.as_ref() == Some(&canonical),
    "resolving stack LIFO invariant violated"
);
```

This changes O(n) to O(1) for every module resolution. With `MAX_IMPORT_DEPTH = 64`, the worst case shifts 63 elements per resolution, which while small in absolute terms is unnecessary work.

**Closure capture round-trip: `Arc -> owned clone -> Arc` on every function invocation** - `src/evaluator.rs:178-179`
**Confidence**: 80%
- Problem: In `invoke_function`, each captured function is wrapped in `Arc::new(f.clone())` on every call. If a function has N captured sibling functions, each invocation performs N full `FunctionDef` clones (including their own captures). The feature knowledge notes this round-trip is "intentional to break reference cycles," which is a valid design choice. However, the cost is paid per-invocation rather than per-definition. For functions called many times in loops (up to 100k iterations), this multiplies.
- Fix: This is documented as intentional for cycle-breaking. A potential optimization would be to cache the `Arc`-wrapped captures once at definition time (a `Vec<(String, Arc<FunctionDef>)>` alongside the owned captures) to avoid re-wrapping on every call. However, this adds complexity and the current approach is correct. Flagging as should-fix only if profiling shows this is a bottleneck in real workloads.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Lexer allocates `Vec<char>` and `Vec<usize>` for the entire source** - `src/lexer.rs:46-51`
**Confidence**: 82%
- Problem: The lexer eagerly collects all chars and byte offsets into two heap-allocated vectors before scanning begins. For a 10 MB file (the maximum allowed), this allocates ~40 MB of char data (4 bytes per char) plus ~80 MB of usize offsets (8 bytes per usize) for a total of ~120 MB. While correct, a byte-oriented lexer operating directly on the `&str` slice would avoid this overhead entirely. This is a pre-existing architectural choice that the PR refactored (moving from a closure to a struct) but did not introduce.
- Impact: For typical MDS files (a few KB), this is negligible. For files approaching the 10 MB limit, memory usage is 12x the file size.

### LOW

**`collect_all` in `Scope` clones all keys and values across all frames** - `src/scope.rs:170-178`
**Confidence**: 80%
- Problem: `get_all_vars()`, `get_all_functions()`, and `get_all_namespaces()` each flatten all scope frames into a new `HashMap`, cloning every key and value. This runs once per `@define` during closure capture. With `Arc<FunctionDef>`, function clones are O(1), but var and namespace clones copy heap data. For deeply nested scopes with many bindings, this could be noticeable.
- Impact: Low in practice since scope depth and binding count are typically small in MDS templates.

## Suggestions (Lower Confidence)

- **String allocation in `evaluate_nodes` output accumulation** - `src/evaluator.rs:59` (Confidence: 65%) -- Each node's output is appended to a `String::new()` via `push_str`. For large documents with many nodes, pre-sizing the output string (e.g., `String::with_capacity(source.len())`) could reduce reallocations.

- **`format!` in hot path for qualified function name** - `src/evaluator.rs:220` (Confidence: 62%) -- `format!("{namespace}.{name}")` allocates a new String on every qualified call. If the same qualified function is called in a loop, this repeats the allocation. A `SmallString` or caching approach could help in tight loops.

- **`canonical.clone()` before `resolving.insert` and `modules.insert`** - `src/resolver.rs:184,197` (Confidence: 60%) -- Two clones of the `PathBuf` canonical path are made per module resolution. For deep import chains this is minor but could be avoided by consuming the original.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 1 |

**Performance Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The most impactful finding is the file-read-before-cache-check regression in `ModuleCache::resolve`. The previous code checked the cache after canonicalization but before reading file content; the refactoring into `validate_and_read_file` bundled the read with the security checks, causing cache hits to perform unnecessary disk I/O. This should be fixed before merge as it degrades performance for projects with shared modules.

The `Arc<FunctionDef>` and `Arc<ResolvedModule>` changes are solid performance improvements -- O(1) clone on cache hits and shared function definitions are clear wins. The `IndexSet` replacing `HashSet+Vec` is also a good consolidation, though the `shift_remove` should be changed to `pop()` for the LIFO pattern.

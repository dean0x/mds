# Performance Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### HIGH

**`evaluate_for` clones the entire iterable array before iteration** - `src/evaluator.rs:288`
**Confidence**: 85%
- Problem: `evaluate_for` calls `.to_vec()` on the iterable slice at line 288, allocating a full clone of the array before iterating. For arrays approaching `MAX_LOOP_ITERATIONS` (100,000 elements), this is a non-trivial allocation that copies every `Value` element. While this code is pre-existing, the PR is a release-readiness hardening pass, and the evaluator is in the set of touched files (test changes and the `make_err` fix at line 512).
- Impact: For large arrays (10k-100k elements with string values), this produces a full heap copy that doubles peak memory for the iterable data. The allocation is O(n) in array length.
- Fix: Iterate over borrowed items directly, cloning only the individual item for `scope.set_var`:
```rust
let items = iterable
    .as_array()
    .ok_or_else(|| MdsError::type_error(iterable.type_name()))?;

if items.len() > MAX_LOOP_ITERATIONS {
    return Err(MdsError::resource_limit(...));
}

for item in items {
    ctx.total_iterations += 1;
    // ...
    scope.set_var(&block.var, item.clone());
    // ...
}
```
This avoids the upfront `.to_vec()` allocation while still cloning each element individually for scope insertion (which is necessary since `set_var` takes owned `Value`).

### MEDIUM

**`collect_define` eagerly snapshots entire scope for every `@define`** - `src/resolver.rs:594-602`
**Confidence**: 82%
- Problem: Lines 594-602 call `scope.get_all_namespaces()`, `scope.get_all_functions()`, and `scope.get_all_vars()` for every `@define` node. Each call uses `collect_all` (scope.rs:172-180) which iterates all frames and clones every entry into a new `HashMap`. The `get_all_functions()` result is then further cloned via `(*v).clone()` (Arc deref + deep clone of the `FunctionDef` including its own captures) at line 600. In a file with N definitions and a scope of M entries, this is O(N*M) work with deep clones.
- Impact: For modules with many `@define` blocks and rich scope (imported namespaces, many frontmatter vars), each definition pays a high cost. The deep clone of `FunctionDef` captures creates nested allocations. This is an existing pattern but the import helper extraction in this PR touches `collect_define` through its enclosing `collect_definitions_and_imports`.
- Fix: This is an architectural issue that would require a more sophisticated lazy capture or copy-on-write scheme. For v0.1 release it is acceptable since typical modules have single-digit `@define` counts and modest scope sizes. Consider for future optimization if profiling shows this as a hotspot.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`canonicalize_and_check` performs two `canonicalize` syscalls on every resolve including cache hits** - `src/resolver.rs:128-144`
**Confidence**: 83%
- Problem: The `resolve` method calls `canonicalize_and_check` (line 177) before checking the module cache (line 180). `check_symlink` inside `canonicalize_and_check` performs two `canonicalize()` syscalls — one for the parent directory and one for the full path. Cache hits pay this syscall cost unnecessarily. On a project with 20+ imports that resolve to the same handful of cached files, this means dozens of unnecessary kernel calls.
- Impact: Each `canonicalize()` syscall involves stat operations on every path component. For deep directory trees, this is measurable. The cost is per-resolve-call, not per-unique-file.
- Fix: Move the cache lookup before the full security check by doing a cheaper canonical path computation first:
```rust
pub fn resolve(&mut self, path: &Path, ...) -> Result<Arc<ResolvedModule>, MdsError> {
    let canonical = Self::check_symlink(path)?;
    
    // Cache hit — return immediately (skip remaining security checks for known files)
    if let Some(cached) = self.modules.get(&canonical) {
        return Ok(Arc::clone(cached));
    }
    
    // Full security checks only on cache misses
    if self.root_dir.is_none() {
        let entry_dir = canonical.parent().unwrap_or(Path::new("."));
        self.root_dir = Some(find_project_root(entry_dir));
    }
    self.check_import_depth()?;
    self.check_path_traversal(&canonical)?;
    // ...
}
```
Note: this preserves security since `check_symlink` still runs (rejecting symlinks), and `check_import_depth` / `check_path_traversal` are correctly skipped for cached files that already passed these checks on first resolve. The `is_md` extension check can be deferred similarly.

**`format!` allocation in `call_qualified_function` on every qualified call** - `src/evaluator.rs:243`
**Confidence**: 80%
- Problem: `call_qualified_function` allocates `format!("{namespace}.{name}")` on every qualified function call (line 243), even when the call succeeds and the string is only used as the call_key for the call_stack. This is a per-invocation heap allocation.
- Impact: LOW for typical usage (qualified calls are less common than regular calls, and MAX_CALL_DEPTH is 128). Becomes measurable in templates with heavy use of aliased module calls in loops.
- Fix: This is a minor optimization opportunity. Could use a stack-allocated `SmallString` or compute the qualified name only when needed for error reporting. Not blocking for v0.1.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`evaluate_nodes` output string grows with no size hint** - `src/evaluator.rs:59`
**Confidence**: 80%
- Problem: `evaluate_nodes` creates `String::new()` (zero capacity) and grows it by repeated `push_str` calls. For templates with many nodes, this triggers multiple reallocations as the string grows (amortized O(n) but with multiple allocation/copy rounds).
- Fix: Use `String::with_capacity(hint)` where a reasonable estimate can be made (e.g., sum of text node lengths, or a fixed 4KB default).

**`build_cycle_string` allocates intermediate `Vec<String>` for display names** - `src/resolver.rs:719-725`
**Confidence**: 80%
- Problem: `build_cycle_string` maps all paths to display names via `.collect::<Vec<_>>()` then joins them. This creates an intermediate allocation.
- Fix: Use an iterator chain with `itertools::join` or write directly to a `String` buffer. Minor since cycle detection is an error path only.

## Suggestions (Lower Confidence)

- **`scope.push()` per `@for` iteration in evaluator** - `src/evaluator.rs:308` (Confidence: 65%) — Each loop iteration pushes/pops a scope frame (allocating a `Frame` with three empty `HashMap`s). For tight loops with 10k+ iterations, this is 10k HashMap allocations. Could be optimized by reusing a single frame and clearing it between iterations.

- **`HashMap` vs `IndexMap` for `ModuleCache::modules`** - `src/resolver.rs:55` (Confidence: 60%) — Using `HashMap<PathBuf, Arc<ResolvedModule>>` for module cache is fine, but `PathBuf` hashing involves full path string hashing. For projects with many modules, a pre-hashed key or interned path could reduce lookup cost.

- **`String::from_utf8` after `fs::read` in `load_config` and `read_validated_file`** - `src/main.rs:69`, `src/resolver.rs:164` (Confidence: 70%) — Both locations read bytes then convert to UTF-8. The conversion validates the entire byte buffer. This is correct but means the full file content is scanned twice (once for read, once for UTF-8 validation). Using `fs::read_to_string` would let the OS handle this in one pass, but that conflicts with the TOCTOU-safe size check pattern. Acceptable trade-off.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The branch makes a strong positive performance change: replacing `scope.clone()` with `push()`/`pop()` in the validator eliminates per-block heap allocation of entire scope frames — this is the right pattern and aligns with the feature knowledge documentation. The `Arc<ResolvedModule>` caching and `EvalContext` bundling are well-designed.

The main actionable finding is the `.to_vec()` in `evaluate_for` which copies the entire iterable array before iteration. This is a straightforward fix (iterate borrowed items, clone individually) that avoids doubling peak memory for large arrays. The `canonicalize_and_check` ordering issue with cache hits is worth addressing for import-heavy projects but is not blocking.

Conditions for approval:
1. Fix the `.to_vec()` in `evaluate_for` to avoid the full array clone (HIGH severity, straightforward fix)

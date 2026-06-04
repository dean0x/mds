# Performance Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13

## Issues in Your Changes (BLOCKING)

### HIGH

**Excessive cloning in closure capture -- entire scope cloned per function definition** - `src/resolver.rs:243-245`
**Confidence**: 90%
- Problem: When defining a function, `get_all_namespaces()`, `get_all_functions()`, and `get_all_vars()` each iterate all scope frames and clone every entry into a new HashMap. This happens for *every* `@define` block in every module. Each `FunctionDef` clone includes its own captured closures recursively (captured_functions contains FunctionDefs which themselves have captured_functions), creating a tree of clones that grows multiplicatively with the number of definitions. In a module with N functions, each function captures all N-1 previously-defined sibling functions, yielding O(N^2) cloning.
- Fix: Consider using `Arc` (or `Rc`) for `FunctionDef` and `NamespaceScope` so that captures share references instead of deep-cloning entire trees. For example:
  ```rust
  pub captured_functions: HashMap<String, Arc<FunctionDef>>,
  ```
  This reduces the per-definition capture cost from O(total_captured_data) to O(number_of_keys) since only the Arc pointer is cloned.

**Full ResolvedModule clone on every cache hit** - `src/resolver.rs:92-93`
**Confidence**: 92%
- Problem: `ModuleCache::resolve` returns `Ok(cached.clone())` which deep-clones the entire `ResolvedModule` -- including all `HashMap<String, FunctionDef>` entries and the `prompt_body` String -- on every cache hit. For modules imported by multiple consumers (e.g. a shared `utils.mds`), this multiplies the allocation cost by the number of importers.
- Fix: Wrap cached modules in `Arc<ResolvedModule>` so cache hits return an `Arc::clone()` (pointer bump) instead of a deep copy:
  ```rust
  pub struct ModuleCache {
      modules: HashMap<PathBuf, Arc<ResolvedModule>>,
      // ...
  }
  ```
  Callers that need mutation can `Arc::make_mut` or extract data selectively.

**Redundant clone of ResolvedModule before caching** - `src/resolver.rs:163`
**Confidence**: 90%
- Problem: `self.modules.insert(canonical, resolved.clone())` clones the entire module a second time right before returning it. Combined with the clone on cache hit (line 93), every freshly-resolved module is cloned once to store and once to return.
- Fix: Insert the module into the cache first, then clone from cache (or better, use `Arc` as suggested above):
  ```rust
  self.modules.insert(canonical.clone(), resolved);
  Ok(self.modules.get(&canonical).unwrap().clone())
  ```
  With the `Arc` approach this becomes zero-copy.

**Lexer collects all chars into a Vec plus a separate byte-offset Vec** - `src/lexer.rs:27-32`
**Confidence**: 85%
- Problem: `source.chars().collect::<Vec<char>>()` allocates 4 bytes per character (32-bit `char`) for the entire source, and `source.char_indices().map(...).collect()` allocates another `usize` per character. For a 1 MB file this is ~12 MB of upfront allocation. Since MDS files are overwhelmingly ASCII, this is wasteful.
- Fix: For a first improvement without a full rewrite, consider operating on `&[u8]` with UTF-8 boundary checks only when non-ASCII bytes are encountered. Alternatively, use `str::char_indices()` directly in a streaming fashion rather than pre-collecting both arrays. A simpler incremental fix: keep only `byte_offsets` and index into the source `&str` directly using byte positions, eliminating the `chars` Vec.

### MEDIUM

**`get_all_exports()` clones every exported function into a Vec** - `src/resolver.rs:426-431`
**Confidence**: 85%
- Problem: `get_all_exports()` creates a `Vec<(String, FunctionDef)>` by cloning every exported function name and definition. This is called from `to_namespace()` (line 448), wildcard re-exports (line 292), and merge imports (line 361). Each call site then iterates the Vec and inserts into another HashMap, meaning data is cloned into a temporary collection only to be immediately moved elsewhere.
- Fix: Return an iterator instead of a collected Vec to avoid the intermediate allocation:
  ```rust
  pub fn exported_iter(&self) -> impl Iterator<Item = (&String, &FunctionDef)> {
      self.functions.iter()
          .filter(move |(name, _)| !self.has_explicit_exports || self.explicit_exports.contains(name.as_str()))
  }
  ```
  Call sites can then clone only what they need.

**Scope cloned for validation of `@for` and `@define` bodies** - `src/validator.rs:59, 64`
**Confidence**: 82%
- Problem: `let mut inner = scope.clone()` deep-clones the entire scope (all frames, all variables, all functions, all namespaces) to add a single loop variable or parameter placeholder. For modules with many imports and definitions, the scope can be substantial.
- Fix: Push a temporary frame onto the existing scope, validate, then pop -- the same pattern the evaluator already uses:
  ```rust
  scope.push();
  scope.set_var(&block.var, Value::Null);
  let result = validate(&block.body, scope, file, source);
  scope.pop()?;
  result
  ```
  This requires changing `scope` to `&mut Scope` in the validator signature, but eliminates the deep clone entirely.

**`evaluate_for` clones the entire iterable array** - `src/evaluator.rs:318`
**Confidence**: 80%
- Problem: `.clone()` on the items array duplicates all elements before iteration. For an array of 100,000 strings, this doubles peak memory for that loop.
- Fix: Use a borrowed slice and clone individual items only when binding to the loop variable, or better, take ownership of the Value from scope if it won't be needed after the loop:
  ```rust
  let items = iterable.as_array()
      .ok_or_else(|| MdsError::type_error(iterable.type_name()))?;
  // iterate by reference, clone only the loop variable binding
  for item in items {
      scope.push();
      scope.set_var(&block.var, item.clone());
      // ...
  }
  ```
  This avoids the upfront full-array clone; individual item clones are still needed for the mutable scope but the intermediate Vec is eliminated.

**`call_function` clones the entire FunctionDef on every call** - `src/evaluator.rs:231-234`
**Confidence**: 82%
- Problem: `get_function(name)?.clone()` deep-clones the function definition (including its captured closure data) every time a function is invoked. If a function is called in a loop, this clones on every iteration.
- Fix: With the `Arc<FunctionDef>` approach suggested above, function lookup returns an `Arc` clone (cheap pointer bump). If `Arc` is not adopted, consider restructuring `invoke_function` to accept a reference by temporarily removing the borrow conflict (e.g., extract captured data first, then call evaluate_nodes).

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`invoke_function` restores captured scope with per-entry clone** - `src/evaluator.rs:202-215`
**Confidence**: 80%
- Problem: Three loops iterate captured_namespaces, captured_functions, and captured_vars, cloning each value to set it in scope. When a function has many captured items (e.g. a utility module with 10+ imports and functions), every invocation pays this clone cost.
- Fix: This is a natural consequence of the deep-clone capture model. With `Arc`-based sharing, these clones become reference count bumps. As a nearer-term fix, captured data could be stored as `Arc<HashMap<...>>` and set into the scope frame as a shared reference.

**`error::at()` copies entire source into every error** - `src/error.rs:11-17`
**Confidence**: 80%
- Problem: `source.to_string()` inside `miette::NamedSource::new(file, source.to_string())` copies the entire source file content into every error constructed with `_at` variants. If multiple errors are generated during validation of a large file, each one carries a full copy of the source.
- Fix: Pre-allocate the source into an `Arc<String>` once per module resolution and pass the Arc to error constructors:
  ```rust
  fn at(file: &str, source: Arc<String>, offset: usize, len: usize) -> ...
  ```
  This way all errors from the same file share one allocation.

## Pre-existing Issues (Not Blocking)

(No pre-existing issues -- this is a new codebase on the branch.)

## Suggestions (Lower Confidence)

- **String building in evaluator could use capacity hints** - `src/evaluator.rs:47` (Confidence: 65%) -- `String::new()` starts with zero capacity. For templates that produce substantial output, a `String::with_capacity()` hint (e.g., estimated from node count) could reduce reallocations.

- **`canonical.clone()` called twice in resolve()** - `src/resolver.rs:150-151` (Confidence: 70%) -- The canonical PathBuf is cloned twice (into `resolving` HashSet and `resolving_stack` Vec). One clone could be avoided by inserting into the set and pushing a reference or by using a single indexed collection.

- **`clean_output` allocates a trimmed copy then another owned string** - `src/lib.rs:293-299` (Confidence: 62%) -- `trimmed.to_string()` followed by `out.push('\n')` allocates twice. Could trim in-place by truncating `result` to the trimmed length and appending the newline.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 4 | 3 | - |
| Should Fix | - | - | 2 | - |
| Pre-existing | - | - | - | - |

**Performance Score**: 5/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The primary performance concern is pervasive deep-cloning of `FunctionDef`, `ResolvedModule`, and scope data throughout the compilation pipeline. The current design clones entire data trees where shared references (`Arc`) would suffice. For small templates this is invisible, but it scales poorly:

1. **Module cache hits** clone everything instead of sharing via Arc.
2. **Closure captures** clone the entire visible scope per function definition, yielding O(N^2) behavior for N definitions in a module.
3. **Validation** clones the scope to add a single variable placeholder.
4. **Lexer** pre-allocates two full-source-length arrays for char/byte mapping.

None of these are correctness issues, and for typical prompt templates (small files, few functions) the impact is negligible. However, for the compiler to handle large module graphs or templates with many definitions, adopting `Arc`-based sharing for `FunctionDef`, `NamespaceScope`, and `ResolvedModule` would eliminate the most significant allocation hotspots. This is a structural change best addressed as a deliberate refactoring pass rather than piecemeal fixes.

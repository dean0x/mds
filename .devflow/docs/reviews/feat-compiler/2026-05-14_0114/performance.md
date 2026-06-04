# Performance Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**Lexer converts entire source to `Vec<char>` plus a parallel `Vec<usize>` byte-offset array** - `src/lexer.rs:27-31`
**Confidence**: 85%
- Problem: `tokenize()` allocates two full vectors up front — `chars: Vec<char>` (4 bytes per char) and `byte_offsets: Vec<usize>` (8 bytes per char on 64-bit). For a 10 MB file (the maximum accepted), this is ~12 bytes per codepoint, totaling roughly 120 MB of heap just for these two vectors. The lexer could instead iterate over `char_indices()` directly.
- Impact: 3x memory amplification during tokenization for large files. Most templates are small, but the 10 MB limit allows adversarial inputs. For files of any non-trivial Unicode content, the materialized `Vec<char>` also defeats cache locality vs. byte-level iteration.
- Fix: Replace `chars.collect()` + `byte_offsets.collect()` with a streaming `char_indices()` iterator that provides both `(byte_offset, char)` per iteration, or use a `Peekable<CharIndices>` approach. This eliminates both allocations entirely. Example sketch:
  ```rust
  let mut chars = source.char_indices().peekable();
  // Instead of chars[pos], use chars.next() / chars.peek()
  // Instead of byte_pos(pos), the byte offset comes from the iterator tuple
  ```

**Excessive cloning of `FunctionDef` (including full AST bodies) during closure capture** - `src/resolver.rs:238-241`, `src/scope.rs:126-154`
**Confidence**: 85%
- Problem: Every `@define` block triggers `scope.get_all_vars()`, `scope.get_all_functions()`, and `scope.get_all_namespaces()`, each of which clones every entry across all scope frames into a new `HashMap`. Since `FunctionDef` contains `Vec<Node>` (the full AST body), `HashMap<String, FunctionDef>` for captured functions, and `HashMap<String, Value>` for captured vars, each clone is deep and recursive. In a module with N function definitions, the Nth function captures clones of all N-1 prior functions (including their own captures), resulting in O(N^2) deep clones.
- Impact: Quadratic growth in memory and time as the number of functions in a single module increases. A module with 50 functions would clone ~1,225 function definitions, each potentially carrying the captured state of earlier functions.
- Fix: Wrap `FunctionDef` in `Arc<FunctionDef>` (or at minimum `Rc<FunctionDef>`) so that closure capture is a reference-count bump rather than a deep clone. The AST body (`Vec<Node>`) could similarly be `Arc<[Node]>`. The scope's `get_all_*` methods would then return `HashMap<String, Arc<FunctionDef>>`, making capture nearly free.
  ```rust
  pub struct FunctionDef {
      pub params: Vec<String>,
      pub body: Arc<[Node]>,
      pub captured_namespaces: HashMap<String, Arc<NamespaceScope>>,
      pub captured_functions: HashMap<String, Arc<FunctionDef>>,
      pub captured_vars: HashMap<String, Value>,
  }
  ```

### MEDIUM

**String concatenation via `push_str` in evaluator hot path without pre-sizing** - `src/evaluator.rs:47`
**Confidence**: 82%
- Problem: `evaluate_nodes` creates `let mut output = String::new()` with zero capacity. For each text node, interpolation, if-branch, and for-iteration, it calls `push_str` which may trigger multiple reallocations as the string grows. For large templates, this causes repeated memcpy operations during the geometric reallocation pattern.
- Impact: Moderate — Rust's `String` doubles capacity, so the amortized cost is O(n), but the constant factor from repeated reallocations matters for large outputs (up to the 50 MB limit). Each for-loop iteration also creates its own `String::new()` (line 329), which is concatenated back via `push_str`.
- Fix: Use `String::with_capacity()` with a heuristic size estimate. For the top-level call, a reasonable heuristic is the source length or 2x source length. For for-loops, estimate `items.len() * estimated_body_size`.
  ```rust
  // Top-level: estimate output is ~same size as input
  let mut output = String::with_capacity(source.len());
  
  // For loop: estimate based on item count
  let mut output = String::with_capacity(items.len() * 64);
  ```

**`ResolvedModule` is fully cloned on every cache hit** - `src/resolver.rs:93-95`
**Confidence**: 80%
- Problem: `ModuleCache::resolve()` returns `Ok(cached.clone())` which deep-clones the entire `ResolvedModule` including its `HashMap<String, FunctionDef>` (which contains AST bodies), the `prompt_body: Option<String>`, and export metadata. Every import of a cached module pays this cost.
- Impact: Proportional to module size and import fan-out. A utility module imported by 10 files gets cloned 9 times (first hit is cached, subsequent 9 are clones). Combined with the deep `FunctionDef` cloning issue above, this compounds.
- Fix: Return `Arc<ResolvedModule>` from the cache instead of cloning. Change the cache to `HashMap<PathBuf, Arc<ResolvedModule>>` and return `Arc::clone()` which is just a reference count increment.
  ```rust
  pub struct ModuleCache {
      modules: HashMap<PathBuf, Arc<ResolvedModule>>,
      // ...
  }
  ```

**Scope chain walk is O(depth) per lookup with linear scan** - `src/scope.rs:94-96`
**Confidence**: 80%
- Problem: `get_var`, `get_function`, and `get_namespace` each iterate `self.frames.iter().rev()` doing a HashMap lookup at each frame. While individual HashMap lookups are O(1), the frame walk is O(d) where d is the nesting depth. In deeply nested for-loops (up to 100K iterations), each variable reference inside the loop walks the scope chain.
- Impact: Low for typical usage (scope depth is usually 2-5 frames). However, inside nested for-loops with function calls, the constant factor adds up. The scope push/pop per loop iteration (line 339-343 of evaluator.rs) also allocates a new `Frame` with three empty `HashMap`s each time.
- Fix: Consider a flat scope with a scope-ID overlay for common cases, or at minimum pre-allocate `Frame` fields with zero capacity (which `HashMap::new()` already does, so this is actually fine). The scope chain walk is reasonable for the expected depth bounds. No immediate fix required — this is a note for future profiling.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`to_namespace()` double-collects: `get_all_exports()` allocates a `Vec`, then `.into_iter().collect()` builds a HashMap** - `src/resolver.rs:442-444`
**Confidence**: 82%
- Problem: `to_namespace()` calls `get_all_exports()` which allocates a `Vec<(String, FunctionDef)>` via `.map().collect()`, then immediately converts it back to a `HashMap` via `.into_iter().collect()`. This creates an intermediate allocation that is immediately consumed.
- Fix: Add a direct `get_all_exports_map()` method that returns `HashMap<String, FunctionDef>` directly, or inline the filter+clone logic.
  ```rust
  fn to_namespace(&self) -> NamespaceScope {
      let functions = self.functions.iter()
          .filter(|(name, _)| !self.has_explicit_exports || self.explicit_exports.contains(*name))
          .map(|(name, func)| (name.clone(), func.clone()))
          .collect();
      NamespaceScope {
          functions,
          prompt_body: self.prompt_body.clone(),
      }
  }
  ```

**`evaluate_for` calls `.to_vec()` on the iterable array, cloning all values** - `src/evaluator.rs:318-319`
**Confidence**: 82%
- Problem: `iterable.as_array()` returns `Option<&[Value]>` (a borrow), but then `.to_vec()` immediately clones the entire array. This is done because the `scope` borrow is released, but it means every for-loop over N items clones N `Value` objects before iteration even begins.
- Fix: This clone exists to release the borrow on `scope` before the loop body mutates it. The current approach is correct for borrow-checker compliance. A future improvement would be to use `Arc<[Value]>` for array storage so the clone is cheap. No immediate fix needed — the clone is required by the current ownership model.

## Pre-existing Issues (Not Blocking)

(No pre-existing issues — this is a greenfield PR.)

## Suggestions (Lower Confidence)

- **Validator clones the entire scope for `@for` and `@define` body validation** - `src/validator.rs:59,64` (Confidence: 70%) — `scope.clone()` performs a deep copy of all frames including all HashMaps. For validation-only purposes, a `Cow`-like approach or temporary scope extension would avoid the clone. However, validation runs once per compile, so the cost is bounded.

- **`clean_output` does two passes (trim then rebuild)** - `src/lib.rs:299-305` (Confidence: 65%) — After the main character loop, `clean_output` calls `result.trim_end()` which scans backwards, then `trimmed.to_string()` which allocates a new string. This could be done in a single pass by tracking the last non-whitespace position. Impact is minimal since this runs once per compile on the final output.

- **Character-by-character text accumulation in lexer** - `src/lexer.rs:209-229` (Confidence: 60%) — The text token loop pushes characters one at a time into a `String`. For long text segments, this is slower than computing start/end byte offsets and slicing the source string. However, the mixed-concern boundary checks (escaped braces, directives, code fences) make a slice-based approach significantly more complex.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | - | 0 | 2 | 0 |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 6/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The compiler has sensible resource limits (MAX_FILE_SIZE, MAX_LOOP_ITERATIONS, MAX_OUTPUT_SIZE) which bound worst-case behavior. The primary performance concern is the excessive deep cloning pattern throughout the resolver/scope/evaluator pipeline — `FunctionDef` objects containing full AST bodies and captured closure state are cloned repeatedly during both module resolution and function invocation. Wrapping these in `Arc` would be the single highest-impact optimization. The lexer's `Vec<char>` materialization is a secondary concern that matters primarily at the 10 MB file size limit. For typical template sizes (< 100 KB with < 20 functions), current performance should be acceptable.

Conditions for approval:
1. The `Arc<FunctionDef>` refactor (HIGH) should be tracked as a follow-up task — it is not a correctness issue but will cause measurable slowdowns as module complexity grows.
2. The lexer `Vec<char>` allocation (HIGH) should be tracked similarly, particularly if the compiler will be used in hot-path scenarios (e.g., watch mode, CI pipelines with many templates).

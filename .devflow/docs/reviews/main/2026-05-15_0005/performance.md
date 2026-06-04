# Performance Review Report

**Branch**: main (ba816b5)
**Date**: 2026-05-15
**Scope**: Full codebase review (d0624a2...HEAD)

## Issues in Your Changes (BLOCKING)

### HIGH

**Closure capture clones entire scope for every @define** - `src/resolver.rs:552-560`
**Confidence**: 95%
- Problem: `collect_define()` calls `scope.get_all_namespaces()`, `scope.get_all_functions()`, and `scope.get_all_vars()` for every `@define` block. Each of these methods clones the entire scope chain into a new `HashMap` via `collect_all()`. In a module with N definitions, scope capture is O(N^2) in total because each successive `@define` captures all prior definitions. Furthermore, `get_all_functions()` returns `Arc<FunctionDef>` values but line 558 then dereferences and deep-clones each one (`(*v).clone()`) to produce owned `FunctionDef` values for the `captured.functions` field. This means every function body (including its AST nodes) is deep-copied for every subsequent `@define`.
- Impact: For modules with many `@define` blocks (e.g., a utility library with 20+ functions), this creates quadratic allocation growth. Each function captures an ever-growing snapshot, and the deep clone of `FunctionDef` (which contains `Vec<Node>`, `CapturedScope` with nested `HashMap`s) is expensive.
- Fix: Consider a lazy capture strategy: instead of eagerly cloning the entire scope, capture only the names referenced in the function body. Alternatively, use `Arc<FunctionDef>` in the captured scope (the cycle-breaking comment explains why owned values are used, but a `Weak<FunctionDef>` or a separate "capture set" of just the referenced names would avoid the quadratic clone cost). At minimum, the `get_all_functions()` result could be filtered to only include functions actually referenced in the body before cloning:
  ```rust
  // Only capture functions referenced in the body
  let referenced = find_referenced_names(&def.body);
  func.captured.functions = scope
      .get_all_functions()
      .into_iter()
      .filter(|(k, _)| referenced.contains(k))
      .map(|(k, v)| (k, (*v).clone()))
      .collect();
  ```

**Scope chain is fully cloned during validation of @for and @define bodies** - `src/validator.rs:59-61,63-68`
**Confidence**: 90%
- Problem: `validate()` calls `scope.clone()` to create a temporary inner scope for both `@for` blocks (line 59) and `@define` blocks (line 64). `Scope::clone()` deep-clones the entire frame stack including all `HashMap<String, Value>`, `HashMap<String, Arc<FunctionDef>>`, and `HashMap<String, NamespaceScope>`. For nested `@for` loops or `@define` blocks, this multiplies.
- Impact: Validation allocates a full scope copy for every `@for` and `@define` node. In templates with deeply nested loops or many function definitions, this adds measurable allocation overhead.
- Fix: Instead of cloning the entire scope, use `scope.push()` / `scope.pop()` pattern that the evaluator already uses. The validator is read-only after setting the loop variable, so push/pop with a rollback on error would avoid the clone:
  ```rust
  Node::For(block) => {
      // ... iterable checks ...
      scope.push();
      scope.set_var(&block.var, Value::Null);
      let result = validate(&block.body, scope, file, source);
      scope.pop()?;
      result
  }
  ```
  This requires changing the `scope` parameter from `&Scope` to `&mut Scope`, but the validator runs before evaluation so there are no aliasing concerns.

### MEDIUM

**Lexer converts entire source to Vec<char> upfront** - `src/lexer.rs:46-51`
**Confidence**: 85%
- Problem: The lexer eagerly collects all source chars into `Vec<char>` and builds a parallel `Vec<usize>` byte-offset mapping. For a 1 MB source file (near the 10 MB limit), this allocates two large vectors: one of ~250K-1M chars (4 bytes each on most platforms) and one of the same count of `usize` values (8 bytes each). This is 12 bytes per character overhead.
- Impact: For typical template files (1-10 KB), this is negligible. For files near the 10 MB limit, this adds ~120 MB of overhead. The char vector also prevents zero-copy string slicing since every text extraction (frontmatter, code content, directives, interpolations) builds a new `String` by collecting from the char array.
- Fix: Consider operating directly on byte slices with `str` methods and `char_indices()` for the rare cases where character boundaries matter. This is a larger refactor but would eliminate the upfront allocation and enable zero-copy `&str` slicing for text tokens. For an incremental improvement, the `byte_offsets` vector could be lazily computed or replaced with on-demand `str::char_indices()` lookups when needed:
  ```rust
  // Instead of pre-computing all byte offsets:
  fn byte_pos_from_source(source: &str, char_pos: usize) -> usize {
      source.char_indices().nth(char_pos).map(|(i, _)| i).unwrap_or(source.len())
  }
  ```
  Note: this trades O(1) lookup for O(n) per call, so it's only better if `byte_pos()` is called infrequently relative to source size. A better approach for the common ASCII case: detect ASCII-only sources and skip the char vector entirely.

**Lexer builds strings character-by-character** - `src/lexer.rs:177-189,200-212,283-313`
**Confidence**: 82%
- Problem: `scan_code_content()`, `scan_directive()`, and `scan_text()` all build output strings by pushing one character at a time from the `chars` array. This means each text segment performs N `push()` calls with potential reallocations, when a single `&str` slice from the source would suffice.
- Impact: For large text blocks or code fences, the character-by-character push pattern causes multiple reallocations. Since the source string is already in memory, this work is redundant.
- Fix: Track start/end positions and slice directly from `self.source`:
  ```rust
  fn scan_text(&mut self) {
      let start_char = self.pos;
      // ... advance self.pos past text ...
      let start_byte = self.byte_pos(start_char);
      let end_byte = self.byte_pos(self.pos);
      let text = &self.source[start_byte..end_byte];
      if !text.is_empty() {
          self.tokens.push(Token::Text(text.to_string(), start_byte));
      }
  }
  ```

**Array items cloned via .to_vec() in evaluate_for** - `src/evaluator.rs:288`
**Confidence**: 83%
- Problem: `evaluate_for()` calls `iterable.as_array()` which returns `&[Value]`, then immediately calls `.to_vec()` which clones every element. For arrays of strings, this clones every string in the array before iteration begins.
- Impact: For large arrays (up to 100K elements per the limit), this means cloning all 100K values upfront. If the array contains large strings, this doubles memory usage temporarily.
- Fix: Iterate over the borrowed slice and clone only the current loop variable:
  ```rust
  let items = iterable
      .as_array()
      .ok_or_else(|| MdsError::type_error(iterable.type_name()))?;
  // Check length on the borrowed slice
  if items.len() > MAX_LOOP_ITERATIONS { ... }
  for item in items {
      scope.set_var(&block.var, item.clone()); // Clone one at a time
      ...
  }
  ```

**String allocation in evaluator output building** - `src/evaluator.rs:59,298`
**Confidence**: 80%
- Problem: `evaluate_nodes()` and `evaluate_for()` create new `String` accumulators without capacity hints. For loops with many iterations, the output string undergoes repeated reallocations as it grows. The output size check at line 84 (`output.len() > MAX_OUTPUT_SIZE`) runs on every node, which is correct for safety but the string growth pattern could be smoother.
- Impact: Without `String::with_capacity()`, Rust's default doubling strategy means up to O(log n) reallocations for an n-byte output. This is acceptable for most cases but suboptimal for large outputs.
- Fix: For `evaluate_for`, estimate capacity from the number of items:
  ```rust
  let mut output = String::with_capacity(items.len() * 64); // rough estimate
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**collect_all flattens scope by cloning every key and value** - `src/scope.rs:172-180`
**Confidence**: 85%
- Problem: `collect_all()` iterates all frames outer-to-inner and clones every key-value pair into a new `HashMap`. Since `HashMap::collect()` processes items sequentially, duplicate keys from outer frames are first inserted and then overwritten by inner frames, meaning outer-frame entries are cloned wastefully.
- Impact: For deep scope chains with many variables, this clones values that are immediately discarded when shadowed. The method is called three times per `@define` (namespaces, functions, vars), amplifying the waste.
- Fix: Iterate inner-to-outer and use `entry()` API to skip already-shadowed keys:
  ```rust
  fn collect_all<T: Clone>(
      &self,
      get: impl Fn(&Frame) -> &HashMap<String, T>,
  ) -> HashMap<String, T> {
      let mut result = HashMap::new();
      for frame in self.frames.iter().rev() {
          for (k, v) in get(frame) {
              result.entry(k.clone()).or_insert_with(|| v.clone());
          }
      }
      result
  }
  ```
  This avoids cloning values that would be overwritten and reduces to a single clone per visible binding.

**prompt_body cloned in evaluate_include** - `src/evaluator.rs:328`
**Confidence**: 80%
- Problem: `evaluate_include()` clones the entire `prompt_body` string (`body.clone()`) every time an `@include` is evaluated. If the same module is included multiple times or the prompt body is large, this creates redundant copies.
- Impact: For typical use (small prompt bodies, single include), this is minor. For prompt bodies that are large compiled templates included repeatedly, the clones add up.
- Fix: Consider using `Arc<String>` for `prompt_body` in `NamespaceScope` so cloning is O(1). Alternatively, since the result is immediately returned and concatenated into the output, this is somewhat inherent to the string-building approach.

## Pre-existing Issues (Not Blocking)

(No pre-existing issues -- all code is new in this branch.)

## Suggestions (Lower Confidence)

- **Token enum carries owned Strings** - `src/lexer.rs:5-22` (Confidence: 70%) -- All token variants carry `String` (owned). A lifetime-parameterized `Token<'a>` with `&'a str` borrows from the source would eliminate most lexer allocations, but would require pervasive lifetime annotations through the parser.

- **Error type carries Arc<NamedSource<String>> for source context** - `src/error.rs:10-17` (Confidence: 65%) -- The `at()` helper allocates a new `Arc<NamedSource<String>>` for every error with a source span, which clones the entire source string into the error. For error paths this is fine, but if validation produces many errors the copies could add up.

- **clean_output double-processes the output** - `src/lib.rs:332-361` (Confidence: 60%) -- `clean_output()` first iterates all chars to build a result string, then calls `trim_end()` (another scan) and `to_string()` (another allocation). A single-pass approach that tracks trailing whitespace would avoid the second pass and allocation.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 4 | 0 |
| Should Fix | - | 0 | 2 | 0 |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 6/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates good performance awareness overall: module caching with `Arc<ResolvedModule>` is effective, functions are stored as `Arc<FunctionDef>` for O(1) cloning, resource limits are properly bounded (loop iterations, output size, file size, call depth), and the `IndexSet` for cycle detection is well-chosen.

The two HIGH-severity findings are the main concerns:
1. **Quadratic closure capture** in `collect_define()` is the most impactful issue -- it deep-clones an ever-growing scope for every `@define`. For utility modules with many function definitions, this creates O(N^2) allocation growth with large constant factors (deep-cloning AST nodes).
2. **Scope cloning in validation** duplicates the entire scope chain unnecessarily when `push()/pop()` would suffice.

These should be addressed before public release, especially if users are expected to create library modules with many function definitions. The MEDIUM findings are standard optimization opportunities that would benefit from attention but are not blocking.

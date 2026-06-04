# Rust Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13

## Clippy Results

`cargo clippy` (default lints): **0 warnings, 0 errors** -- clean.

`cargo clippy -- -W clippy::pedantic -W clippy::nursery`: 105 warnings. The majority are stylistic (`use_self`, `uninlined_format_args`, `doc_markdown`) which are not blocking. The notable non-stylistic pedantic warnings are listed in the findings below where they overlap with real quality concerns.

## Build Results

`cargo build`: **0 warnings, 0 errors** -- clean.

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`as_array` returns `Option<&Vec<Value>>` instead of `Option<&[Value]>`** - `src/value.rs:102`
**Confidence**: 90%
- Problem: The `as_array` method returns `Option<&Vec<Value>>` instead of the idiomatic `Option<&[Value]>`. Rust API guidelines (C-BORROW) recommend accepting and returning slices rather than `&Vec<T>` to avoid leaking the implementation detail. This forces callers to work with `Vec`-specific APIs unnecessarily.
- Fix:
  ```rust
  #[must_use]
  pub fn as_array(&self) -> Option<&[Value]> {
      match self {
          Value::Array(a) => Some(a),
          _ => None,
      }
  }
  ```
  Note: The only caller (`evaluator.rs:315`) calls `.clone()` on the result and then iterates, so the change is backwards compatible.

**`tokenize` function is 178 lines long** - `src/lexer.rs:25`
**Confidence**: 82%
- Problem: The `tokenize` function spans 178 lines (clippy::too_many_lines detects this at the 100-line threshold). The function handles frontmatter, code fences, directives, escaped braces, interpolation, and regular text all in one monolithic function. While each branch is individually clear, the length makes it harder to reason about control flow.
- Fix: Extract the frontmatter parsing (lines 46-84), the code-fence handling (lines 90-139), and the body-token loop into separate helper functions. The closure `byte_pos` can be passed as a parameter or the byte offsets array can be shared via a small Lexer struct.

**`process_module` is too large (104 lines) with 7 parameters** - `src/resolver.rs:189`
**Confidence**: 80%
- Problem: `process_module` takes 7 parameters and handles tokenization, parsing, scope building, import resolution, export validation, and evaluation all in one method. This exceeds the 100-line pedantic threshold and bundles too many responsibilities.
- Fix: Extract the import/export processing loop (lines 226-303) and the export validation (lines 306-312) into separate methods on `ModuleCache`. This would bring `process_module` under 60 lines and make each responsibility independently testable.

### LOW

**`is_truthy` missing `#[must_use]`** - `src/value.rs:22`
**Confidence**: 85%
- Problem: `is_truthy()` is a pure query method whose return value should never be silently discarded. All other query methods on `Value` (`as_array`, `type_name`) already have `#[must_use]`.
- Fix: Add `#[must_use]` attribute:
  ```rust
  #[must_use]
  pub fn is_truthy(&self) -> bool {
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`.expect()` in production code assumes frames vector is non-empty (3 occurrences)** - `src/scope.rs:89`, `src/scope.rs:103`, `src/scope.rs:117`
**Confidence**: 85%
- Problem: Three calls to `.expect("scope always has at least one frame")` in `set_var`, `set_function`, and `set_namespace`. While the invariant is maintained by the constructor (which initializes with one frame) and `pop()` guards against removing the last frame, `.expect()` will panic at runtime if the invariant is ever violated. The `pop()` method already returns `Result` for the error case, showing the codebase otherwise avoids panics.
- Fix: Since `Scope::new()` guarantees at least one frame and `pop()` prevents removing the last one, the invariant holds. However, for consistency with the `pop()` approach, consider either: (a) adding a `debug_assert!(!self.frames.is_empty())` comment explaining the invariant, or (b) using `unwrap_or_else(|| unreachable!("scope always has at least one frame"))` to signal the impossibility to readers. The current `.expect()` is acceptable given the strong invariant -- this is LOW priority.

**`scope.clone()` in validator for `@for` and `@define` blocks** - `src/validator.rs:59`, `src/validator.rs:64`
**Confidence**: 80%
- Problem: The validator clones the entire `Scope` (including all frames, all HashMaps, all `FunctionDef` values with their captured closures) just to add a single variable for validation of `@for` and `@define` bodies. For deeply nested templates with many imports, this could clone significant data.
- Fix: Instead of cloning the full scope, push a new frame, validate, then pop:
  ```rust
  Node::For(block) => {
      // ... existing validation of iterable ...
      scope.push();
      scope.set_var(&block.var, Value::Null);
      let result = validate(&block.body, scope, file, source);
      scope.pop()?;
      result
  }
  ```
  This requires changing `scope` parameter to `&mut Scope` instead of `&Scope`, which is a small refactor.

### LOW

**`collect_all` clones every key and value across all frames** - `src/scope.rs:153-161`
**Confidence**: 82%
- Problem: `collect_all` (used by `get_all_namespaces`, `get_all_functions`, `get_all_vars`) clones every key and value from every scope frame into a new HashMap. This is called during function definition (resolver.rs:243-245) to capture closures, meaning every `@define` triggers three full scope flattens. For templates with many imports and functions, this could be expensive.
- Fix: Consider using `Rc` or `Arc` for `FunctionDef` and `NamespaceScope` to make cloning cheap (just a reference count bump). Alternatively, if closure capture is rare, a lazy approach that only captures on demand would reduce allocation.

## Pre-existing Issues (Not Blocking)

No pre-existing issues apply -- this is a new codebase (all files added in this branch).

## Suggestions (Lower Confidence)

- **Token enum variants carry positional data inline** - `src/lexer.rs:4-22` (Confidence: 70%) -- Each `Token` variant redundantly includes a `usize` offset. A struct-of-arrays approach (separate `Vec<TokenKind>` and `Vec<usize>`) or a wrapper `struct Token { kind: TokenKind, offset: usize }` would be more idiomatic and avoid the positional tuple pattern.

- **`chars: Vec<char>` allocation in tokenize** - `src/lexer.rs:27` (Confidence: 65%) -- The entire source is collected into a `Vec<char>` for character-level indexing. For ASCII-heavy content (which MDS templates typically are), iterating bytes with UTF-8 awareness could avoid this O(n) allocation. However, the current approach is correct and the allocation is bounded by `MAX_FILE_SIZE`.

- **`HashSet<String>` for call_stack recursion detection** - `src/evaluator.rs:29` (Confidence: 60%) -- Using a `HashSet<String>` allocates a new `String` on every function call insertion (line 216). A `HashSet<&str>` borrowing from the scope's function names would avoid allocation, though it would require lifetime annotations.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 3 | 1 |
| Should Fix | 0 | 0 | 2 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10

The codebase demonstrates strong Rust fundamentals: proper error handling with `thiserror` and `?` propagation, no `unsafe` blocks, no `.unwrap()` in production code (only `.expect()` with invariant-justified messages), exhaustive pattern matching, good use of enums for AST representation, bounded loops and recursion depths, and clean clippy output on default lints. The main areas for improvement are API idiom refinement (`&[T]` over `&Vec<T>`), reducing unnecessary clones in hot paths (validator scope, closure capture), and breaking up the two functions that exceed 100 lines.

**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions: The `as_array` return type should be changed to `Option<&[Value]>` before merge -- it is a public API that will be harder to change later. The other findings are desirable improvements but not merge-blocking.

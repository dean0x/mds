# Rust Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**`collect_all` clones every entry from every scope frame on every closure capture** - `src/scope.rs:146-154`
**Confidence**: 85%
- Problem: `collect_all` iterates all frames and clones every key-value pair into a new `HashMap`. This is called three times per `@define` (for namespaces, functions, and vars) in `resolver.rs:238-240`. Each clone copies the entire accumulated scope including all `FunctionDef` bodies (which contain `Vec<Node>` AST trees and their own captured maps). For modules with many definitions, this creates quadratic allocation: each successive `@define` captures all prior definitions including their captures. This is the Rust anti-pattern of cloning to satisfy the borrow checker rather than restructuring ownership.
- Fix: Consider a reference-counted (`Rc`/`Arc`) approach for `FunctionDef` so that closure captures share the underlying data rather than deep-cloning AST bodies. Alternatively, capture lazily or use indices into a shared function table:
  ```rust
  // Instead of HashMap<String, FunctionDef>, use shared ownership:
  pub struct FunctionDef {
      // ...fields...
      pub captured_functions: HashMap<String, Rc<FunctionDef>>,
  }
  ```

**`HashSet<String>` call stack prevents valid mutual recursion detection granularity** - `src/evaluator.rs:29,188`
**Confidence**: 82%
- Problem: The `call_stack` uses `HashSet<String>` which treats any re-entry of the same function name as recursion. This is correct for direct recursion but the `HashSet` approach means the stack depth is lost -- you can only detect "is this name currently on the stack" but not "how deep are we in this particular chain." More critically, `call_stack` is passed as `&mut HashSet<String>` through `evaluate_nodes`, which means the call stack state is shared across sequential function calls in the same body. If function A calls B and then A calls C, B's call-stack entries are properly cleaned up (line 219). This works correctly but `HashSet` means two calls to the same function in sequence (e.g., `{f("a")} {f("b")}`) will incorrectly flag the second call as recursion if `call_stack.remove` fails for any reason. The current code does handle this correctly with insert/remove, but the data structure choice obscures the intent -- a `Vec<String>` stack would be more idiomatic for call-depth tracking.
- Fix: Consider using a `Vec<String>` to model the actual call stack, which enables both cycle detection and depth tracking in a single structure, and makes the push/pop semantics explicit:
  ```rust
  fn invoke_function(..., call_stack: &mut Vec<String>, ...) {
      if call_stack.contains(call_key) { return Err(recursion); }
      if call_stack.len() >= MAX_CALL_DEPTH { return Err(depth_exceeded); }
      call_stack.push(call_key.to_string());
      let result = evaluate_nodes(...);
      call_stack.pop();
      result
  }
  ```

### MEDIUM

**`source.to_string()` in every `at()` call clones the entire source file** - `src/error.rs:16`
**Confidence**: 88%
- Problem: Every error constructed with an `_at` variant calls `at()`, which calls `source.to_string()` to create a `miette::NamedSource`. This copies the entire file content (up to 10 MB) into every error. Since errors are typically propagated and only one is displayed, this means a single compilation failure allocates a full copy of the source file. The `Arc` wrapper amortizes sharing if the error is cloned, but the initial allocation is unavoidable per error construction.
- Fix: Accept `Arc<String>` or `&Arc<String>` for the source in `at()` so the source can be shared without copying. The resolver already owns the source string and could wrap it in `Arc` once:
  ```rust
  fn at(
      file: &str,
      source: Arc<String>,  // shared, no copy
      offset: usize,
      len: usize,
  ) -> (Option<SourceSpan>, Option<Arc<miette::NamedSource<String>>>) {
      (
          Some(SourceSpan::new(offset.into(), len)),
          Some(Arc::new(miette::NamedSource::new(file.to_string(), Arc::try_unwrap(source).unwrap_or_else(|s| (*s).clone())))),
      )
  }
  ```
  Or restructure to share the source `Arc` at the resolver level and pass it through.

**Missing `#[must_use]` on public functions returning `Result`** - `src/lib.rs:338`, `src/error.rs:177-440`
**Confidence**: 90%
- Problem: `load_vars_file` at `lib.rs:338` has `#[must_use]` which is good, but the many constructor methods on `MdsError` (e.g., `MdsError::syntax()`, `MdsError::undefined_var()`, etc.) return `Self` without `#[must_use]`. While these are typically used inline, the Rust API Guidelines (C-MUST-USE) recommend `#[must_use]` on any function that returns a value where ignoring it is almost certainly a bug. The feature knowledge confirms `#[must_use]` is a project pattern.
- Fix: Add `#[must_use]` to the `MdsError` constructors, or add `#[must_use]` to the `MdsError` enum itself:
  ```rust
  #[derive(Error, Debug, Diagnostic)]
  #[must_use]
  pub enum MdsError { ... }
  ```

**`evaluate_for` clones the entire iterable array via `.to_vec()`** - `src/evaluator.rs:319`
**Confidence**: 85%
- Problem: `iterable.as_array()` returns `Option<&[Value]>`, but then `.to_vec()` is called to clone the entire array. This is done because `scope` is mutably borrowed later (`scope.push()`, `scope.set_var()`), and you cannot hold an immutable reference to scope data while mutating scope. However, for large arrays (up to 100,000 elements per `MAX_LOOP_ITERATIONS`), this creates a full clone of all values before iteration begins.
- Fix: This is a borrow-checker-driven clone. One approach is to extract the items from scope before the loop by temporarily removing the iterable, or to restructure scope so that iteration doesn't require cloning. At minimum, add a comment explaining why the clone is necessary:
  ```rust
  // Clone required: scope is mutably borrowed during iteration (push/set_var/pop),
  // so we cannot hold a reference to the array inside scope.
  let items = iterable.as_array()
      .ok_or_else(|| MdsError::type_error(iterable.type_name()))?
      .to_vec();
  ```

**`MdsError` is not `Clone` but contains `Arc` fields suggesting it should be** - `src/error.rs:20-174`
**Confidence**: 80%
- Problem: `MdsError` wraps source code in `Arc<miette::NamedSource<String>>`, which suggests the intent is for errors to be cheaply shareable. However, `MdsError` itself does not derive `Clone`. Since all its fields are either `String`, `Option<SourceSpan>`, `Option<Arc<...>>`, or `usize` -- all of which are `Clone` -- deriving `Clone` would be trivially correct and would enable patterns like caching errors or returning them from multiple code paths without `Box<dyn Error>`.
- Fix: Add `Clone` to the derive:
  ```rust
  #[derive(Error, Debug, Diagnostic, Clone)]
  pub enum MdsError { ... }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`Scope::set_var` / `set_function` / `set_namespace` use `unwrap()` on `last_mut()`** - `src/scope.rs:90,101,112`
**Confidence**: 85%
- Problem: While these are guarded by `debug_assert!` (which is correct), the `unwrap()` will panic in release builds if the invariant is ever violated. The `debug_assert!` only fires in debug builds. The `pop()` method already returns `Result` to prevent underflow, but a defensive approach would use `unwrap_or_else` with a message or propagate the error.
- Fix: The `debug_assert!` + `unwrap()` pattern is acceptable here given that `pop()` prevents underflow. However, for maximum safety, consider returning `Result` or using `expect()` with a descriptive message instead of bare `unwrap()`:
  ```rust
  self.frames.last_mut()
      .expect("BUG: scope has no frames — Scope::new() guarantees at least one")
      .vars.insert(name.to_string(), value);
  ```

**`resolving_stack` and `resolving` HashSet are redundant data structures** - `src/resolver.rs:52-53`
**Confidence**: 82%
- Problem: `ModuleCache` maintains both a `HashSet<PathBuf>` for O(1) cycle detection and a `Vec<PathBuf>` for ordered cycle path reconstruction. This dual structure must be kept in sync (insert into both, remove from both). An `IndexSet` from the `indexmap` crate would provide O(1) lookup AND insertion order in a single structure, eliminating the sync requirement.
- Fix: Consider using `IndexSet<PathBuf>` from the `indexmap` crate (already transitively pulled in by `serde_yaml`), or keep the current approach with a comment explaining the trade-off:
  ```rust
  // IndexSet gives O(1) contains + insertion-order iteration in one structure
  use indexmap::IndexSet;
  resolving: IndexSet<PathBuf>,
  ```

## Pre-existing Issues (Not Blocking)

(No pre-existing issues -- all code is new in this branch.)

## Suggestions (Lower Confidence)

- **Consider `Cow<'a, str>` for `TextNode::text` and `Frontmatter::raw`** - `src/ast.rs:39,11` (Confidence: 65%) -- Many text nodes are slices of the original source that are cloned into owned `String`s. Using `Cow<'a, str>` could avoid allocations for pass-through text, though it would require lifetime annotations throughout the AST.

- **`parse_quoted_path` silently accepts paths with embedded quotes** - `src/parser.rs:448-457` (Confidence: 70%) -- The function finds the first `"` after the opening quote. A path like `"foo"bar"` would parse as `foo` and leave `bar"` as unparsed trailing content. The caller does check for trailing content in some paths (`parse_import_directive`), but `parse_export_directive` at line 419 does not validate trailing content after the path.

- **`serde_yaml` 0.9 is unmaintained; consider migrating to `serde_yml`** - `Cargo.toml:9` (Confidence: 72%) -- The `serde_yaml` crate (0.9.x) has been deprecated by its author. The community fork `serde_yml` is actively maintained and API-compatible. This is not urgent but worth tracking.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 3 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates strong Rust fundamentals: idiomatic error handling with `thiserror`/`miette`, proper use of `Result` throughout, good use of `#[must_use]`, bounded recursion/iteration limits, and clean clippy output. The main concerns are performance-related: excessive cloning in scope capture and error construction. These are not correctness issues but will impact compilation performance on larger template projects. The `HashSet`-based call stack works correctly but a `Vec` stack would be more idiomatic. Address the HIGH items before merge or document them as known trade-offs for v0.1.

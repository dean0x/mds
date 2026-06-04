# Rust Review Report

**Branch**: main (ba816b5)
**Base**: d0624a2
**Date**: 2026-05-15

## Summary

The MDS compiler is a well-structured Rust codebase with zero clippy warnings, zero build warnings, 286 passing tests, and no `unsafe` code. The code demonstrates strong Rust proficiency: proper use of `thiserror` + `miette` for error handling, `Arc` for shared ownership, enum-based AST with exhaustive pattern matching, and resource limits on all unbounded operations. The primary concerns are a pre-release dependency (`serde_yml 0.0.12`), a few unnecessary full-scope clones in the validator, and some opportunities to improve API ergonomics with Cow and borrowing.

## Issues in Your Changes (BLOCKING)

### HIGH

**Pre-release dependency: `serde_yml 0.0.12`** - `Cargo.toml:12`
**Confidence**: 95%
- Problem: `serde_yml` is at version `0.0.12` (pre-release, pre-semver-stable). The `0.0.x` range means any minor bump could introduce breaking changes. The comment in Cargo.toml acknowledges this but shipping with it for a public release is risky.
- Impact: Users may encounter breakage when running `cargo update`. Downstream consumers of the library inherit this unstable transitive dependency.
- Fix: Evaluate alternatives:
  ```toml
  # Option 1: Pin more tightly
  serde_yml = "=0.0.12"
  
  # Option 2: Use the established serde_yaml (deprecated but stable)
  # or wait for serde_yml 0.1.x
  ```

**`collect_all` shadowing relies on HashMap insertion-order semantics** - `src/scope.rs:172-180`
**Confidence**: 85%
- Problem: The `collect_all` method uses `.flat_map(...).collect()` with outer-to-inner frame iteration, relying on the fact that `HashMap::from_iter` keeps the last value for duplicate keys. While this is the documented behavior of `HashMap::from_iter` (per the standard library docs), it is a subtle invariant that the comment says "overwritten by inner frames" but which depends on the `FromIterator` impl rather than an explicit overwrite. A future refactor could break this silently.
- Impact: If the semantics were wrong, closure captures would use outer-scope values instead of inner-scope values, producing silent incorrect behavior.
- Fix: Make the invariant explicit and self-documenting:
  ```rust
  fn collect_all<T: Clone>(
      &self,
      get: impl Fn(&Frame) -> &HashMap<String, T>,
  ) -> HashMap<String, T> {
      let mut result = HashMap::new();
      // Iterate outer-to-inner: later inserts overwrite earlier ones,
      // so inner frames shadow outer frames correctly.
      for frame in &self.frames {
          for (k, v) in get(frame) {
              result.insert(k.clone(), v.clone());
          }
      }
      result
  }
  ```

### MEDIUM

**Full scope clone in validator for `@for` and `@define` blocks** - `src/validator.rs:59,64`
**Confidence**: 88%
- Problem: `scope.clone()` performs a deep clone of the entire scope chain (all frames, all `HashMap` values, all `Value` and `FunctionDef` data) just to add a single variable for static validation. In deeply nested templates with many variables/functions, this could be expensive.
- Impact: Validation performance degrades with scope size. For typical use this is acceptable, but for templates with large frontmatter variable sets or many imports, it becomes a quadratic cost.
- Fix: Use `push`/`pop` instead of cloning:
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
  This requires changing `scope` from `&Scope` to `&mut Scope` in the validator signature, which is a small API change but avoids the deep clone entirely.

**`chars` and `byte_offsets` Vec allocation in Lexer** - `src/lexer.rs:46-51`
**Confidence**: 82%
- Problem: The lexer collects all chars into a `Vec<char>` and builds a parallel `Vec<usize>` of byte offsets. For a 10 MB file (the maximum), this is ~40 MB of char data (4 bytes per char) plus ~80 MB of usize data (8 bytes per usize on 64-bit), totaling ~120 MB just for the lexer's index structures.
- Impact: Memory amplification factor of ~12x relative to input size at the maximum file size. For typical small templates this is negligible, but the MAX_FILE_SIZE of 10 MB allows inputs that would use significant memory.
- Fix: Consider working directly with byte offsets and `str` slicing, or at minimum document the memory characteristics. Alternatively, lower MAX_FILE_SIZE to a more reasonable limit for template files (e.g., 1 MB).

**`as f64` lossy conversion for i64 values** - `src/value.rs:49,139,145`
**Confidence**: 80%
- Problem: `i64 as f64` is a lossy cast for integers larger than 2^53. An i64 value like `9007199254740993` would silently round to `9007199254740992.0` when stored as `Value::Number(f64)`.
- Impact: Users with large integer values in frontmatter would get silently wrong numbers. For a template language this is unlikely but technically incorrect.
- Fix: Document the precision limitation or add a check:
  ```rust
  impl From<i64> for Value {
      fn from(n: i64) -> Self {
          // i64 values beyond f64 precision (2^53) are represented approximately
          Value::Number(n as f64)
      }
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing `#[non_exhaustive]` on public enums** - `src/error.rs:22`, `src/value.rs:10`
**Confidence**: 85%
- Problem: `MdsError` and `Value` are public enums without `#[non_exhaustive]`. Adding a new variant in a future version is a semver-breaking change because downstream code with exhaustive `match` statements would break.
- Impact: Limits the ability to evolve the public API without breaking changes after 1.0.
- Fix:
  ```rust
  #[non_exhaustive]
  #[derive(Error, Debug, Diagnostic, Clone)]
  pub enum MdsError { ... }
  
  #[non_exhaustive]
  #[derive(Debug, Clone, PartialEq)]
  pub enum Value { ... }
  ```

**`assert_eq!` in production code for LIFO check** - `src/resolver.rs:207`
**Confidence**: 82%
- Problem: The LIFO invariant check in `resolve()` uses `assert_eq!` which panics in both debug and release builds. While the comment explains it is "safety-critical", panicking in a library is generally undesirable -- callers cannot catch or recover from it. The evaluator handles the same pattern with a `Result`-based check (see `src/evaluator.rs:208-215`), creating an inconsistency.
- Impact: A compiler bug that triggers this assertion would crash the host process rather than returning a recoverable error. The evaluator already demonstrates the correct pattern.
- Fix: Match the evaluator's approach:
  ```rust
  let popped = self.resolving.pop();
  if popped.as_ref() != Some(&canonical) {
      return Err(MdsError::syntax(format!(
          "internal error: resolving stack LIFO violated: expected '{}', got {:?}",
          canonical.display(), popped
      )));
  }
  ```

### LOW

**Error constructor boilerplate** - `src/error.rs:177-465`
**Confidence**: 80%
- Problem: The `MdsError` impl block contains ~290 lines of repetitive constructor methods (each variant has a plain constructor and an `_at` variant with source spans). This is functional but verbose.
- Impact: Maintenance burden when adding new error variants -- each requires two new methods with similar structure.
- Fix: Consider a macro to generate the constructor pairs:
  ```rust
  macro_rules! error_constructors {
      ($variant:ident, $field:ident) => {
          pub fn $variant(value: impl Into<String>) -> Self { ... }
          pub fn ${concat($variant, _at)}(...) -> Self { ... }
      }
  }
  ```
  This is optional -- the current approach is clear and explicit, which has value.

## Pre-existing Issues (Not Blocking)

None. This is the initial implementation -- all code is new.

## Suggestions (Lower Confidence)

- **Consider `Cow<'_, str>` for TextNode and Frontmatter** - `src/ast.rs:38-39` (Confidence: 65%) -- The `TextNode.text` and `Frontmatter.raw` fields are always cloned from token data. Using `Cow<'a, str>` could avoid allocations when the AST doesn't outlive the source, but would require lifetime parameters on the AST types.

- **Consider using `indexmap::IndexMap` for scope frames** - `src/scope.rs:49-55` (Confidence: 60%) -- The scope frames use `HashMap` which has non-deterministic iteration order. While correctness doesn't depend on order, deterministic output across runs would be nice for reproducibility in testing and CI.

- **`Scope::expect` vs structured error propagation** - `src/scope.rs:106,120,134` (Confidence: 70%) -- Three `expect("BUG: scope has no frames")` calls exist in production code. The comment explains why this is structurally sound (frames are never empty due to private fields), and converting to `Result` would add noise. This is a reasonable trade-off but worth noting for completeness.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

## Positive Observations

These deserve recognition as strong Rust practices:

1. **Zero clippy warnings** with `-D warnings` -- clean build across 8,760 lines of Rust
2. **No `unsafe` code** anywhere in the codebase
3. **No `.unwrap()` in production code** -- all unwraps confined to tests; `expect` used only for structurally-guaranteed invariants with clear BUG messages
4. **`thiserror` + `miette`** combination provides both typed errors (for programmatic handling) and rich diagnostic output (source spans, help text, error codes)
5. **`#[must_use]` on all public API functions** in `lib.rs` with descriptive messages
6. **Bounded everything**: MAX_CALL_DEPTH, MAX_LOOP_ITERATIONS, MAX_TOTAL_ITERATIONS, MAX_OUTPUT_SIZE, MAX_FILE_SIZE, MAX_NESTING_DEPTH, MAX_IMPORT_DEPTH, MAX_VALUE_DEPTH, MAX_WARNINGS, MAX_CONFIG_SIZE -- no unbounded operations
7. **`Arc<FunctionDef>`** for cheap scope sharing without deep clones during function calls
8. **`IndexSet` for cycle detection** -- preserves insertion order for readable cycle path messages while providing O(1) membership test
9. **CapturedScope with owned FunctionDef** to break reference cycles -- well-documented design decision
10. **286 tests** with strong integration coverage via fixture files

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions:
1. Address the `serde_yml` pre-release dependency strategy (pin exact version or document the risk)
2. Fix the `assert_eq!` panic in `resolver.rs:207` to return a `Result` (matching the evaluator's pattern)

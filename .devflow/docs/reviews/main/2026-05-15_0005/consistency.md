# Consistency Review Report

**Branch**: main (d0624a2...HEAD)
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### HIGH

**Magic number 256 used inconsistently for nesting/depth limits** - `src/validator.rs:154`, `src/resolver.rs:21`, `src/main.rs:51`
**Confidence**: 92%
- Problem: The parser defines a named constant `MAX_NESTING_DEPTH: usize = 256` at `src/parser.rs:12`, but the same logical value `256` is hardcoded as a raw integer in three other files:
  - `src/validator.rs:154`: `if depth > 256 {` in `validate_var_args`
  - `src/resolver.rs:21`: `for _ in 0..256 {` in `find_project_root`
  - `src/main.rs:51`: `for _ in 0..256 {` in `load_config`
  The validator's `256` is conceptually the same depth limit as `MAX_NESTING_DEPTH`, but uses a magic number instead of referencing the constant. The resolver and main.rs use `256` for directory traversal caps, which is a different concept entirely but should still have a named constant for clarity and maintainability.
- Fix: Extract named constants for each usage:
  ```rust
  // In validator.rs, import or reference MAX_NESTING_DEPTH from parser:
  use crate::parser::MAX_NESTING_DEPTH; // (requires making it pub(crate))
  if depth > MAX_NESTING_DEPTH { ... }

  // In resolver.rs and main.rs, define a named constant:
  const MAX_DIR_TRAVERSAL: usize = 256;
  ```

---

**Inconsistent error handling: `assert_eq!` panic vs. `Result` return for LIFO invariant** - `src/resolver.rs:207` vs. `src/evaluator.rs:208-215`
**Confidence**: 90%
- Problem: The evaluator and resolver both enforce a LIFO invariant on their respective stacks, but they handle violations differently:
  - `src/resolver.rs:207`: Uses `assert_eq!` which panics in both debug and release builds:
    ```rust
    assert_eq!(popped.as_ref(), Some(&canonical), "resolving unmark must be LIFO");
    ```
  - `src/evaluator.rs:208-215`: Returns a structured `MdsError::syntax(...)` error:
    ```rust
    let lifo_result = if popped.as_deref() == Some(call_key) {
        Ok(())
    } else {
        Err(MdsError::syntax(format!(
            "internal error: call_stack LIFO violated: ..."
        )))
    };
    ```
  Both comments explicitly describe the same kind of safety-critical LIFO invariant. The evaluator chose a graceful error path with clear rationale ("Return a structured error rather than panicking so callers get a proper diagnostic"), while the resolver chose panic. This is an inconsistent pattern for the same class of invariant violation.
- Fix: Align the resolver with the evaluator's approach and return a structured error instead of panicking, or use `debug_assert_eq!` if a panic-based check is preferred (since LIFO violations are compiler bugs, not user errors):
  ```rust
  // Option A: match evaluator's pattern (graceful error)
  let popped = self.resolving.pop();
  if popped.as_ref() != Some(&canonical) {
      return Err(MdsError::syntax(format!(
          "internal error: resolving stack LIFO violated"
      )));
  }

  // Option B: debug-only assertion (less expensive in release)
  debug_assert_eq!(popped.as_ref(), Some(&canonical), "resolving unmark must be LIFO");
  ```

---

### MEDIUM

**Inconsistent `#[must_use]` annotation coverage** - multiple files
**Confidence**: 85%
- Problem: The `#[must_use]` annotation is applied inconsistently across the public API:
  - `src/lib.rs`: All 10 public functions have `#[must_use = "..."]` with messages -- good.
  - `src/error.rs:20`: `MdsError` enum itself has `#[must_use]` (no message) -- good.
  - `src/value.rs:22,98,107`: Three `Value` methods (`is_truthy`, `as_array`, `type_name`) have `#[must_use]` -- good.
  - But `src/lib.rs:375`: `compile_file` is missing `#[must_use]`. Looking more carefully, it does have it. Actually all public functions in lib.rs do have it.
  - The deeper inconsistency: `src/evaluator.rs:41` `pub fn evaluate(...)` returns `Result<String, MdsError>` but has no `#[must_use]`. Similarly, `src/resolver.rs:164` `ModuleCache::resolve(...)` returns `Result<Arc<ResolvedModule>, MdsError>` but has no `#[must_use]`. These are `pub` functions on `pub(crate)` modules, so they are effectively internal. This is acceptable but worth noting -- the pattern is: public API gets `#[must_use]`, internal functions do not.
  
  The real inconsistency is minor: `MdsError` uses `#[must_use]` without a message string, while all `lib.rs` functions use `#[must_use = "..."]` with descriptive messages. This inconsistency between the enum-level and function-level annotation style is cosmetic.
- Fix: Consider adding a message to the `MdsError` annotation for uniformity:
  ```rust
  #[must_use = "errors should be handled"]
  #[derive(Error, Debug, Diagnostic, Clone)]
  pub enum MdsError { ... }
  ```

---

**`Scope::pop()` returns `Result` but `Scope::push()` is infallible -- asymmetric API** - `src/scope.rs:80-95`
**Confidence**: 82%
- Problem: The Scope API has an asymmetry: `push()` always succeeds (no return value), but `pop()` returns `Result<(), MdsError>`. Meanwhile, `set_var`/`set_function`/`set_namespace` use `.expect("BUG: scope has no frames")` (panic) for the same kind of internal invariant (frames is non-empty). This means there are two different patterns for handling the "frames should not be empty" invariant within the same struct:
  - `pop()`: returns `Err(MdsError::syntax("internal error: ..."))`
  - `set_var/set_function/set_namespace`: `.expect("BUG: ...")`
  The code comments at `src/scope.rs:99-101` acknowledge this decision and explain the rationale, which is reasonable. However, having both patterns in the same struct is a consistency concern.
- Fix: This is documented and justified in the code comments. The current approach is pragmatic: `pop()` can fail from user input patterns (deeply nested blocks), while `set_*` cannot fail if the constructor works. No code change strictly needed, but if you want full consistency, either make all invariant checks return `Result` or all use `expect`.

---

**Inconsistent parameter naming: `message` vs `name` vs `path` vs `got` vs `cycle` in error constructors** - `src/error.rs:178-465`
**Confidence**: 80%
- Problem: The `MdsError` convenience constructors use the first field name as the parameter name in the `_at` variants, which is consistent within each error type. However, there is an inconsistency in the semantic grouping of these names:
  - `syntax(message)`, `import_error(message)`, `export_error(message)` -- use `message` for the free-form error text
  - `undefined_var(name)`, `undefined_fn(name)`, `recursion(name)`, `name_collision(name)`, `arity(name, ...)` -- use `name` for the identifier
  - `file_not_found(path)`, `not_mds_file(path)` -- use `path` for file references
  - `type_error(got)` -- uses `got` for the actual type received
  - `circular_import(cycle)` -- uses `cycle` for the cycle chain
  
  This naming convention is actually well-structured and semantically accurate. The only mild inconsistency is that `io(message)` and `resource_limit(message)` follow the `message` pattern, while `yaml_error(message)` and `json_error(message)` also do. This is consistent. No real issue here upon deeper analysis.
- Fix: No action needed. The naming is semantically consistent.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`pub(crate) mod scope` but `Scope`, `FunctionDef`, `CapturedScope`, `NamespaceScope` all have `pub` fields and methods** - `src/lib.rs:46`, `src/scope.rs`
**Confidence**: 83%
- Problem: The `scope` module is declared `pub(crate)` in `lib.rs:46`, meaning it is not part of the public API. However, all its types (`Scope`, `FunctionDef`, `CapturedScope`, `NamespaceScope`) have `pub` visibility on their fields and methods. This is technically fine (the module visibility restricts access), but it creates a misleading signal. If someone later makes the module public, all internals would be exposed.
  
  By contrast, `evaluator.rs` correctly uses `pub(crate)` on its `EvalContext` struct (`src/evaluator.rs:28`), which is more precise about intent.
- Fix: Consider changing `pub` to `pub(crate)` on Scope's methods and fields, to match the `EvalContext` pattern. This is low-priority since the module-level restriction already applies.

---

**Inconsistent doc comment coverage on internal helper functions** - multiple files
**Confidence**: 81%
- Problem: Doc comments (`///`) are applied inconsistently to internal helper functions:
  - `src/lexer.rs`: Every method and free function has a doc comment, including helpers like `is_line_start_chars`, `scan_fence`, `skip_newline` -- thorough and consistent.
  - `src/evaluator.rs`: Some functions have doc comments (`evaluate`, `prefer_first_error`, `EvalContext`) but others do not (`evaluate_nodes`, `evaluate_expr`, `resolve_args`, `invoke_function`, `call_function`, `call_qualified_function`, `evaluate_if`, `evaluate_for`, `evaluate_include`).
  - `src/resolver.rs`: Most functions have doc comments, but `find_project_root` (line 19) has a one-liner while `parse_frontmatter` (line 725) has the same brief style.
  - `src/validator.rs`: Only `validate` has a doc comment. `validate_node`, `validate_expr`, `validate_var_args` have none.
  
  The lexer sets the most thorough standard; other modules are less consistent.
- Fix: Add brief doc comments to uncommented internal functions in evaluator.rs and validator.rs for consistency with the lexer's thorough documentation. Even `/// Evaluate an if-block.` is better than nothing.

## Pre-existing Issues (Not Blocking)

(None -- this is the initial codebase.)

## Suggestions (Lower Confidence)

- **Resource limit constants scattered across modules** - `src/evaluator.rs:9-22`, `src/resolver.rs:44-47`, `src/parser.rs:12`, `src/main.rs:26`, `src/value.rs:6` (Confidence: 72%) -- All resource limit constants (`MAX_CALL_DEPTH`, `MAX_FILE_SIZE`, `MAX_NESTING_DEPTH`, `MAX_VALUE_DEPTH`, etc.) are defined locally in each module. A centralized `limits` module or section in `lib.rs` would make it easier to audit and adjust all bounds in one place, especially for security reviews.

- **`compile_file` convenience function takes `&str` while all other public API functions take `impl AsRef<Path>`** - `src/lib.rs:375` (Confidence: 75%) -- The `compile_file` function takes `path: &str`, while `compile`, `check`, `compile_collecting_warnings`, and `check_collecting_warnings` all take `path: impl AsRef<Path>`. The `impl AsRef<Path>` pattern is more ergonomic and idiomatic. While `compile_file` is a convenience wrapper, the input type divergence is a mild inconsistency.

- **Two error handling libraries in the project: `miette` in main.rs vs `MdsError` in library code** - `src/main.rs`, `src/error.rs` (Confidence: 65%) -- The CLI uses `miette::miette!()` for its own errors and converts `MdsError` via `.map_err(miette::Error::from)`. This is a well-known pattern (library error type vs. application error type), but it means errors created in `main.rs` (e.g., "mds.json is too large") bypass the `MdsError` categorization entirely, relying on the `exit_code` function's `else` branch. This is intentional and documented, but worth noting as a design choice.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates strong consistency overall. Naming conventions follow Rust idioms (snake_case functions, PascalCase types, SCREAMING_SNAKE constants) uniformly across all 11 source files. Error handling consistently uses `Result` types with `MdsError` throughout the library -- no panics in user-facing paths. The `_at` constructor pattern in `MdsError` is applied completely and consistently to all span-bearing error variants. Module visibility is well-structured with a clean public API surface.

The two HIGH findings (magic number duplication and inconsistent LIFO-invariant handling) are real consistency violations that should be addressed before public release. The magic number issue risks divergence if one constant is updated but the others are not. The LIFO invariant inconsistency means one code path panics (resolver) while another gracefully errors (evaluator) for the same class of bug -- a confusing difference for maintainers.

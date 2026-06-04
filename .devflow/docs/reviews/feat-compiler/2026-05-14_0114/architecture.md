# Architecture Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**Resolver `process_module` has too many responsibilities (SRP violation)** - `src/resolver.rs:184-323`
**Confidence**: 85%
- Problem: `process_module` is a 140-line method that performs tokenization, parsing, scope construction from frontmatter, runtime var application, function registration with lexical capture, import resolution, export validation, semantic validation, evaluation, and result assembly. This is 10+ distinct responsibilities in a single method. Adding any new directive type or phase (e.g., optimization, type checking) requires modifying this one method.
- Fix: Extract sequential phases into named helper methods on `ModuleCache`. The pipeline is already conceptually clean (tokenize -> parse -> build_scope -> validate -> evaluate), but the "build_scope" phase conflates function registration, import resolution, and export handling into one loop. Consider extracting:
  ```rust
  fn build_scope_from_ast(
      &mut self,
      module: &Module,
      base_dir: &Path,
      is_md: bool,
      runtime_vars: &HashMap<String, Value>,
      warnings: &mut Vec<String>,
      source_ctx: (&str, &str),
  ) -> Result<(Scope, HashMap<String, FunctionDef>, bool, HashSet<String>), MdsError>
  ```
  This would keep the orchestration in `process_module` but move the scope-building logic (the 80-line for-loop over `module.body` nodes) into a focused method.

**Closure capture clones entire scope eagerly** - `src/resolver.rs:238-240`
**Confidence**: 82%
- Problem: Every `@define` triggers `scope.get_all_namespaces()`, `scope.get_all_functions()`, and `scope.get_all_vars()`, which each clone the entire scope chain into new `HashMap`s. These cloned maps are then stored inside each `FunctionDef` and cloned again every time the function is invoked (`src/evaluator.rs:203-213`). In a module with N functions and M variables, this is O(N * M) cloning at definition time, plus O(M) cloning per invocation. For modules with many functions or large scopes, this creates quadratic allocation.
- Fix: Use `Rc<HashMap<...>>` or an arena-based approach for captured scope data so that multiple functions sharing the same definition-site scope share one allocation. Alternatively, capture lazily (only capture names actually referenced in the function body).

### MEDIUM

**`ModuleCache` mixes caching, security validation, and orchestration** - `src/resolver.rs:47-161`
**Confidence**: 80%
- Problem: The `resolve` method on `ModuleCache` handles symlink rejection, path canonicalization, root directory setup, cache lookup, cycle detection, import depth enforcement, path traversal prevention, file reading, file size validation, file type validation, and delegation to `process_module`. While each check is individually correct, the method has 13 distinct concerns. This makes it hard to test individual security checks in isolation.
- Fix: Extract a `validate_and_read_file` method that handles file I/O, size checks, symlink rejection, and path-traversal prevention. The `resolve` method would then focus on cache/cycle management and delegation:
  ```rust
  fn validate_and_read_file(&self, path: &Path) -> Result<(PathBuf, String, bool), MdsError> {
      // symlink check, canonicalize, root_dir check, read, size check, type check
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`FunctionDef` struct has too many fields for a data type** - `src/scope.rs:8-17`
**Confidence**: 80%
- Problem: `FunctionDef` bundles both the function's definition (params, body) and its captured closure environment (three separate `HashMap` fields). This couples function identity with scope capture mechanics. Every clone of a `FunctionDef` must clone all five fields, including three potentially large maps.
- Fix: Separate the closure capture into its own type:
  ```rust
  #[derive(Debug, Clone)]
  pub struct CapturedScope {
      pub namespaces: HashMap<String, NamespaceScope>,
      pub functions: HashMap<String, FunctionDef>,
      pub vars: HashMap<String, Value>,
  }

  #[derive(Debug, Clone)]
  pub struct FunctionDef {
      pub params: Vec<String>,
      pub body: Vec<Node>,
      pub captured: CapturedScope,
  }
  ```
  This makes the closure relationship explicit and could enable sharing captured scopes via `Rc` in the future.

**Warnings use `Vec<String>` threaded through every function** - Multiple files
**Confidence**: 80%
- Problem: The `warnings: &mut Vec<String>` parameter is threaded through `evaluate`, `evaluate_nodes`, `evaluate_expr`, `resolve_args`, `invoke_function`, `call_function`, `call_qualified_function`, `evaluate_if`, `evaluate_for`, and `evaluate_include` -- 10+ function signatures. This is classic "parameter drilling" (a form of tight coupling to the warning collection mechanism). Adding a new phase or changing warning behavior requires touching every signature in the chain.
- Fix: Move warnings into the `Scope` struct or into a dedicated `CompilationContext` that is already threaded through the system. Alternatively, since warnings are only generated in one place (`evaluate_include`), the current cost may be acceptable as a pragmatic trade-off for v0.1. If more warning sites emerge, refactor then.

## Pre-existing Issues (Not Blocking)

(No pre-existing issues -- all code is new in this branch.)

## Suggestions (Lower Confidence)

- **Consider a trait-based pipeline abstraction** - `src/resolver.rs:194-314` (Confidence: 65%) -- The sequential pipeline (tokenize -> parse -> validate -> evaluate) could be expressed as a chain of trait-implementors, making it easier to add optional phases (optimization, linting). Currently acceptable for v0.1 complexity level.

- **`ResolvedModule` leaks internal representation** - `src/resolver.rs:32-38` (Confidence: 70%) -- `ResolvedModule.functions` is a public `HashMap<String, FunctionDef>` but consumers should only use `get_export` / `get_all_exports`. Making the field private and only exposing the accessor methods would enforce the export-visibility invariant structurally.

- **`collect_all` in Scope re-collects on every call** - `src/scope.rs:146-154` (Confidence: 65%) -- Each closure capture calls `collect_all` three times, creating three new HashMaps by iterating all frames. Could be optimized with a snapshot/generation approach if performance becomes a concern.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The architecture is fundamentally sound. The codebase follows a strictly sequential pipeline (lexer -> parser -> validator -> resolver -> evaluator), with clear module boundaries and a well-designed separation between pure functional core (AST, Value, Scope) and the imperative shell (CLI in main.rs, file I/O in resolver). Key architectural strengths:

1. **Clean dependency graph**: Dependencies flow strictly inward. The AST module has zero dependencies. Value depends only on error. Scope depends on AST and Value. Evaluator depends on AST, error, scope, and value. Resolver orchestrates everything. No circular module dependencies exist.

2. **Well-bounded resource usage**: Every loop, recursion path, file read, and output buffer has explicit limits (MAX_CALL_DEPTH, MAX_LOOP_ITERATIONS, MAX_TOTAL_ITERATIONS, MAX_OUTPUT_SIZE, MAX_FILE_SIZE, MAX_IMPORT_DEPTH, MAX_NESTING_DEPTH, MAX_VALUE_DEPTH). This is exemplary.

3. **Consistent error handling**: All fallible operations return `Result<T, MdsError>`. The error type is a single enum with rich diagnostic context (source spans, help text). No panics in business logic.

4. **Minimal dependencies**: Only clap, serde_json, serde_yaml, miette, and thiserror. tempfile is dev-only. No unnecessary dependencies.

The conditions for approval are:
- The two HIGH findings (SRP in `process_module` and quadratic closure capture cloning) should be tracked for resolution. They are acceptable for v0.1 but will become maintenance burdens as the language grows.
- The remaining MEDIUM findings are improvements worth making but not blocking.

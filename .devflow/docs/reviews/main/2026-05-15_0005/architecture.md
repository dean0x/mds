# Architecture Review Report

**Branch**: main (ba816b5 vs d0624a2)
**Date**: 2026-05-15
**Scope**: Full codebase review — all 5,586 lines of Rust source

## Summary

The MDS compiler demonstrates strong architectural fundamentals: clean separation of concerns across eight well-defined modules (lexer, parser, AST, evaluator, resolver, scope, validator, value), unidirectional dependency flow, a well-contained public API, and consistent Result-based error handling with rich diagnostics via miette. The main architectural concern is the resolver module's dual responsibility as both the module linker and the full compilation pipeline orchestrator, which conflates two conceptually distinct roles and will impede extensibility as the language grows.

## Issues in Your Changes (BLOCKING)

### HIGH

**Resolver is the God Orchestrator — SRP Violation** — `src/resolver.rs:245-282`
**Confidence**: 90%
- Problem: `process_module` (resolver.rs:245) orchestrates the entire compilation pipeline: tokenize, parse, scope-build, definition collection, export validation, semantic validation, and evaluation. The resolver module handles both its core responsibility (module resolution, import/export linking, caching, cycle detection) AND full pipeline orchestration. This is a Single Responsibility Principle violation — the module has multiple reasons to change (adding a new pipeline phase vs. changing how imports resolve).
- Impact: Adding new compiler phases (optimization, code generation, formatting) will bloat this module further. Testing the pipeline independently from module resolution is difficult. The module is already the second-largest source file at 740 lines.
- Fix: Extract a `Pipeline` or `Compiler` struct that owns the orchestration sequence. The resolver would become one phase called by the pipeline rather than the pipeline driver:
  ```rust
  // src/pipeline.rs
  pub(crate) struct Pipeline { cache: ModuleCache }
  
  impl Pipeline {
      pub fn compile(&mut self, source: &str, ...) -> Result<ResolvedModule, MdsError> {
          let tokens = tokenize(source, file_str)?;
          let module = parse_with_ctx(&tokens, file_str, source)?;
          let scope = build_scope(...)?;
          self.cache.resolve_definitions(&module, &mut scope, ...)?;
          validator::validate(&module.body, &scope, ...)?;
          let output = evaluate(&module.body, &mut scope, ...)?;
          // ...
      }
  }
  ```

**Resolver directly calls evaluator — Layering Violation** — `src/resolver.rs:9`
**Confidence**: 88%
- Problem: The resolver imports and calls `crate::evaluator::evaluate` directly (resolver.rs:273). In a clean compiler architecture, the resolver (which handles name resolution, import linking, and cycle detection) should not know about evaluation. This creates a bidirectional concern: the resolver depends on the evaluator, and the evaluator depends on types defined in scope (which the resolver populates). While there is no actual Rust circular dependency (the `use` graph is a DAG), the conceptual coupling is tight.
- Impact: Cannot evaluate without resolving, and cannot resolve without evaluating — the two phases are fused. This blocks use cases like "resolve all imports but defer evaluation" (useful for IDE tooling, language server support, or dry-run validation).
- Fix: Have the orchestration layer (see SRP fix above) call evaluator after resolver completes, rather than having the resolver call the evaluator internally. The resolver's `process_module` return type would become an intermediate representation (resolved AST + scope) rather than the final evaluated output.

### MEDIUM

**Validator uses scope cloning for nested scopes** — `src/validator.rs:59, 64`
**Confidence**: 82%
- Problem: The validator clones the entire `Scope` to check `@for` and `@define` blocks. Each `Scope` contains `HashMap<String, Arc<FunctionDef>>` and `HashMap<String, NamespaceScope>` across all frames. For deeply nested templates with many imports, this creates O(n * depth) allocation where n is the number of scope entries.
- Impact: Unnecessary allocation during validation. The evaluator already uses push/pop for the same purpose, demonstrating the correct pattern.
- Fix: Use `scope.push()` / `scope.pop()` in the validator (matching the evaluator's pattern), setting the loop variable and params in the pushed frame:
  ```rust
  Node::For(block) => {
      // ... check iterable ...
      let mut inner = scope.clone(); // replace with push/pop
      scope.push();
      scope.set_var(&block.var, Value::Null);
      let result = validate(&block.body, scope, file, source);
      scope.pop()?;
      result
  }
  ```
  Note: this requires taking `scope: &mut Scope` instead of `scope: &Scope` in the validator.

**Warning system uses `Vec<String>` threaded through all layers** — `src/evaluator.rs:44`, `src/resolver.rs:166`, `src/lib.rs:82`
**Confidence**: 80%
- Problem: Warnings are accumulated via `&mut Vec<String>` passed as a parameter through compile -> resolver -> evaluator -> evaluate_nodes -> evaluate_include. This raw threading adds a parameter to every function in the call chain and uses unstructured strings rather than typed warning variants.
- Impact: Adding new warning types (e.g., deprecation warnings, performance hints) requires grepping for string patterns. Filtering or categorizing warnings programmatically is impossible. The `MAX_WARNINGS` guard (evaluator.rs:22) operates on a different constant than could be enforced structurally.
- Fix: Define a `Warning` enum alongside `MdsError`, and use a shared collector (e.g., `WarningCollector` struct wrapping `Vec<Warning>`) to centralize warning management:
  ```rust
  pub enum Warning {
      EmptyInclude { alias: String },
      // future: Deprecated { feature: String, alternative: String },
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`MdsError` constructor explosion — 27 public methods** — `src/error.rs:177-465`
**Confidence**: 85%
- Problem: `MdsError` has 14 variants, each with a plain constructor and an `_at` variant (with source span), yielding 27 public constructors. Every variant that carries a span repeats the same `(Option<SourceSpan>, Option<Arc<NamedSource>>)` pattern. The constructors are all `pub` even though they are only called from within the crate (the error module itself is `pub`).
- Impact: Large API surface that is tedious to maintain. Adding a new error variant requires adding two constructors with the same boilerplate. External consumers can construct arbitrary `MdsError` values, which may not be desirable.
- Fix: (1) Make constructors `pub(crate)` since they are implementation detail. (2) Consider a builder or a shared `SpanInfo` struct to reduce the `_at` boilerplate:
  ```rust
  pub(crate) struct SpanInfo { file: String, source: String, offset: usize, len: usize }
  
  impl MdsError {
      pub(crate) fn syntax(msg: impl Into<String>) -> Self { ... }
      pub(crate) fn syntax_spanned(msg: impl Into<String>, span: SpanInfo) -> Self { ... }
  }
  ```

**`ResolvedModule` fields are `pub` but module is `pub(crate)`** — `src/resolver.rs:36-41`
**Confidence**: 82%
- Problem: `ResolvedModule` has all fields as `pub` (`functions`, `prompt_body`, `has_explicit_exports`, `explicit_exports`), but the resolver module is `pub(crate)`. While Rust prevents external access today, the `pub` visibility on fields is misleading — it suggests these are part of the public API. If the module ever becomes `pub`, internal representation details would leak.
- Impact: If the resolver module visibility is ever relaxed (e.g., for advanced library consumers), the entire internal state is exposed. The `has_explicit_exports` + `explicit_exports` pair is a code smell — this could be a single `Option<HashSet<String>>` using `None` for "no explicit exports".
- Fix: Make fields `pub(crate)` and consolidate the export tracking:
  ```rust
  pub(crate) struct ResolvedModule {
      pub(crate) functions: HashMap<String, Arc<FunctionDef>>,
      pub(crate) prompt_body: Option<String>,
      /// None = all functions visible; Some(set) = only listed names visible
      pub(crate) explicit_exports: Option<HashSet<String>>,
  }
  ```

**Scope types are `pub` but module is `pub(crate)`** — `src/scope.rs:13-63`
**Confidence**: 80%
- Problem: `CapturedScope`, `FunctionDef`, `Scope`, and `NamespaceScope` are all `pub struct` with `pub` fields, but the `scope` module is `pub(crate)`. Same concern as `ResolvedModule` — these are internal implementation types with unnecessarily broad visibility markers.
- Fix: Change all to `pub(crate)`.

### LOW

**`lib.rs` public API has 12 functions — combinatorial expansion pattern** — `src/lib.rs:77-417`
**Confidence**: 80%
- Problem: The public API has 12 functions across 4 groups: `compile`/`compile_str`/`compile_str_with`/`compile_collecting_warnings`/`compile_str_collecting_warnings`, and the parallel `check` family. This is a combinatorial expansion of 3 dimensions: (file vs string) x (with options vs without) x (emit warnings vs collect warnings).
- Impact: Each new option dimension doubles the function count. Adding a "dry run" flag would create another set of variants.
- Fix: Consider a builder pattern for compile options:
  ```rust
  mds::compile(path).with_vars(vars).collecting_warnings().run()?;
  ```
  This is a v0.2+ concern; the current surface is manageable but shows early signs of expansion.

## Pre-existing Issues (Not Blocking)

None identified. All code was introduced in the reviewed commit.

## Suggestions (Lower Confidence)

- **AST nodes lack span information** — `src/ast.rs` (Confidence: 72%) — Only `Interpolation`, `ForBlock`, `IfBlock`, `DefineBlock`, and `ImportDirective` carry `offset`. `TextNode`, `ExportDirective::Named`, and `EscapedBrace` have no position info, which will limit error reporting as the language grows. Consider a uniform `Span` field on all nodes.

- **`collect_all` in Scope allocates full HashMap on every closure capture** — `src/scope.rs:172-180` (Confidence: 65%) — Every `@define` triggers `get_all_namespaces()`, `get_all_functions()`, and `get_all_vars()`, each creating a new HashMap by iterating all frames. For modules with many definitions, this is O(definitions * scope_size). Could be optimized with a generation counter or lazy capture.

- **`serde_yml` is pre-release (0.0.12)** — `Cargo.toml:12` (Confidence: 75%) — The YAML parser dependency is at 0.0.x, which may introduce breaking changes. The comment acknowledges this, but for a public release, consider pinning more precisely or having a plan for migration.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 3 | 1 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10

The module decomposition is strong (lexer -> parser -> AST is textbook), dependency directions are clean (no circular imports), the scope chain implementation is well-designed, and the error type hierarchy provides rich diagnostics. The main concern is the resolver's dual role as both module linker and pipeline orchestrator, which should be separated before the architecture calcifies. The public API is well-curated with good `pub(crate)` boundaries at the module level, though internal struct visibility is looser than needed.

**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions: The HIGH-severity resolver SRP issue (resolver-as-orchestrator) should be addressed before v1.0 to prevent the module from becoming a bottleneck for all future compiler changes. For v0.1, the architecture is sound and functional.

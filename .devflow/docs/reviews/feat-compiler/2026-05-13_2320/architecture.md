# Architecture Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13
**Scope**: Full branch diff (8,916 lines added across 105 files -- new compiler codebase)

## Issues in Your Changes (BLOCKING)

### HIGH

**Resolver accumulates multiple responsibilities (SRP tension)** - `src/resolver.rs:188-328`
**Confidence**: 85%
- Problem: `ModuleCache::process_module` is a 140-line method that orchestrates tokenization, parsing, scope construction, frontmatter parsing, import resolution, export validation, semantic validation, and evaluation -- all in a single method. While the resolver is documented as "the orchestrator," the method mixes orchestration with inline business logic: it manually iterates the AST body to collect defines, handle imports, process exports, and validate export names, all interleaved in one pass. This makes the method a nexus of change reasons -- adding a new directive type, changing export semantics, or modifying scope construction all require modifying this single method.
- Impact: As the language grows, `process_module` will grow proportionally. Adding new node types or modifying scope initialization will require careful reasoning about ordering within this monolith.
- Fix: Extract the body-iteration phase into a dedicated function (e.g., `build_module_scope`) that takes the AST and returns a structured intermediate result (scope, functions, exports). This separates "what scope does this module produce" from "how do we drive the pipeline." The pipeline orchestration (tokenize, parse, validate, evaluate) can remain in `process_module` as a clean sequence of calls.

```rust
// Sketch: extract scope-building from process_module
struct ModuleScope {
    scope: Scope,
    functions: HashMap<String, FunctionDef>,
    has_explicit_exports: bool,
    explicit_exports: HashSet<String>,
}

fn build_module_scope(
    body: &[Node],
    base_scope: Scope,
    // ... other params
) -> Result<ModuleScope, MdsError> {
    // Move the body-iteration logic here
}
```

---

**`process_module` re-registers defines during evaluation (duplicate scope mutation)** - `src/evaluator.rs:80-82` and `src/resolver.rs:228-248`
**Confidence**: 82%
- Problem: `process_module` in the resolver iterates the AST body and registers all `@define` blocks into the scope (lines 228-248), capturing lexical closures. Then the evaluator *also* registers `@define` blocks into the scope during its own walk (evaluator.rs line 80-82), but without the lexical capture logic (no `captured_namespaces`, `captured_functions`, or `captured_vars`). This means there are two code paths that register functions -- one with full closure capture (resolver) and one without (evaluator). The evaluator's registration overwrites the resolver's richer definition, potentially losing captured scope.
- Impact: Functions that rely on lexical closure capture from their definition site could silently lose access to captured namespaces and sibling functions when the evaluator re-processes the same `@define` node. This creates a subtle ordering dependency between resolver and evaluator scope setup.
- Fix: Either (a) have the evaluator skip `@define` nodes entirely (they are already handled by the resolver, similar to how `Import` and `Export` are skipped at line 83-85), or (b) move all scope registration to one place. Option (a) is the minimal fix:

```rust
// In evaluator.rs evaluate_nodes, change:
Node::Define(block) => {
    scope.set_function(&block.name, FunctionDef::from(block));
}
// To:
Node::Define(_) => {
    // Handled by resolver (with lexical capture), skip during evaluation
}
```

### MEDIUM

**Shared constant defined in two places** - `src/main.rs:203` and `src/resolver.rs:43`
**Confidence**: 90%
- Problem: `MAX_STDIN_SIZE` in main.rs (10 MB) and `MAX_FILE_SIZE` in resolver.rs (10 MB) represent the same logical limit but are defined independently. The comment in main.rs even says "mirrors the per-file limit in resolver.rs." If one changes, the other must be updated manually.
- Impact: Divergence risk. A future change to the file size limit could miss the stdin mirror, leading to inconsistent behavior between file and stdin input.
- Fix: Export the constant from the resolver (or a shared `limits` module) and reference it from main.rs:

```rust
// In resolver.rs (already pub(crate)):
pub(crate) const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

// In main.rs:
const MAX_STDIN_SIZE: u64 = mds::resolver::MAX_FILE_SIZE;  // or re-export from lib.rs
```

---

**`clean_output` lives in `lib.rs` but is a rendering concern** - `src/lib.rs:271-301`
**Confidence**: 80%
- Problem: `clean_output` (whitespace normalization: collapse 3+ newlines to 2, trim edges) is a post-processing step on the rendered output, but it lives in `lib.rs` alongside the public API functions. The evaluator produces the raw output, and this function transforms it. It belongs closer to the rendering/evaluation layer or in its own utility.
- Impact: Minor. As more post-processing steps are added (e.g., trailing whitespace normalization, output format conversion), they will accumulate in `lib.rs`, diluting the public API facade.
- Fix: Move `clean_output` into a `render.rs` utility module or into `evaluator.rs` as a post-processing step. Alternatively, keep it in `lib.rs` but accept this as a pragmatic choice for a small codebase.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`MdsError::Io` used as a catch-all for non-I/O errors** - `src/evaluator.rs:91-93`, `src/evaluator.rs:321-327`, `src/evaluator.rs:342-347`
**Confidence**: 85%
- Problem: Resource exhaustion errors (output size exceeded, loop iteration limits) are reported through `MdsError::Io`, which has the diagnostic code `mds::io`. These are not I/O errors -- they are resource limit violations. This conflates two distinct error categories, making it harder for programmatic consumers to distinguish "file system error" from "your template is too complex."
- Impact: Any caller matching on `MdsError::Io` to handle file system errors will also catch resource exhaustion, and vice versa.
- Fix: Add a dedicated `MdsError::ResourceLimit` variant:

```rust
#[error("resource limit exceeded: {message}")]
#[diagnostic(code(mds::resource_limit))]
ResourceLimit { message: String },
```

---

**Validator duplicates evaluation logic for `@for` type checking** - `src/validator.rs:40-62` and `src/evaluator.rs:304-358`
**Confidence**: 80%
- Problem: The validator checks that `@for` iterables are arrays (validator.rs:50-58), and the evaluator performs the same check again (evaluator.rs:315-318). Both check `Value::Array`, both produce `MdsError::type_error`. The validator also clones the entire scope to simulate the loop variable binding (line 59: `let mut inner = scope.clone()`), which is an expensive operation for validation-only purposes.
- Impact: Low runtime cost since validation happens once per module, but it represents a design tension: either the validator is authoritative and the evaluator trusts it, or the evaluator is self-sufficient and the validator is advisory. Currently both are independently authoritative, leading to duplicated logic.
- Fix: This is acceptable as defense-in-depth, but document the intentional duplication. If the validator is intended to catch all type errors, the evaluator checks become assertions (debug-only). If the evaluator is self-sufficient, the validator type checks become optional early reporting.

## Pre-existing Issues (Not Blocking)

*No pre-existing issues -- this is a new codebase.*

## Suggestions (Lower Confidence)

- **`FunctionDef` captures are cloned eagerly** - `src/resolver.rs:243-245` (Confidence: 70%) -- Every `@define` captures the entire visible scope (all namespaces, functions, and variables) via `get_all_*` which clone everything. For modules with many imports, this could be expensive. Consider lazy capture or capture-by-reference with a generation counter.

- **Parser uses string-based directive dispatch** - `src/parser.rs:159-204` (Confidence: 65%) -- `parse_directive` matches on string prefixes (`"if "`, `"for "`, `"define "`, etc.) rather than a tokenized directive type. This is pragmatic for the current language size but could benefit from a dedicated `DirectiveKind` enum as the directive set grows.

- **No trait abstraction for the compilation pipeline** - (Confidence: 62%) -- The pipeline stages (tokenize, parse, validate, evaluate) are concrete function calls with no trait-based abstraction. For a template compiler of this scope this is appropriate, but if plugin/extension support is ever needed, trait-based pipeline stages would enable it.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10

The overall architecture is well-structured with clear layer separation. The dependency graph is clean and acyclic:

```
main.rs -> lib.rs -> resolver.rs -> {lexer, parser, evaluator, validator, scope}
                                     lexer -> error
                                     parser -> {ast, error, lexer}
                                     evaluator -> {ast, error, scope, value}
                                     validator -> {ast, error, scope, value}
                                     scope -> {ast, value}
                                     value -> error
                                     ast -> (no internal deps)
                                     error -> (no internal deps)
```

No circular dependencies. No upward dependencies. Each module has a clear primary responsibility. The public API in `lib.rs` provides a clean facade. The main issues are: (1) `process_module` doing too much in one method, (2) duplicate `@define` registration between resolver and evaluator which could cause subtle scope capture bugs, and (3) minor organizational cleanups.

**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions:
1. Investigate the duplicate `@define` registration in the evaluator (HIGH) -- this may cause a functional bug where closure captures are silently lost
2. Consider extracting `process_module` scope-building into a dedicated function (HIGH) -- will pay off as the language grows

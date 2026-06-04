# Complexity Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13

## Issues in Your Changes (BLOCKING)

### HIGH

**`process_module` does too many things (7 responsibilities)** - `resolver.rs:189-328`
**Confidence**: 90%
- Problem: `process_module` at 140 lines handles tokenization, parsing, scope building, frontmatter parsing, function definition collection, import resolution, export validation, semantic validation, and evaluation. This is 7+ distinct responsibilities in a single function with cyclomatic complexity estimated at 15+ (multiple match arms, nested conditionals, early returns). The function is hard to reason about as a unit because changes to any one phase can silently affect others.
- Fix: Extract into named pipeline stages. Each stage is already logically distinct:
```rust
fn process_module(...) -> Result<ResolvedModule, MdsError> {
    let module = self.parse_source(source, file_str)?;
    let mut scope = self.build_scope_from_frontmatter(&module, is_md, runtime_vars)?;
    let (functions, exports) = self.collect_definitions(&module.body, base_dir, runtime_vars, &mut scope, warnings, source, file_str)?;
    self.validate_exports(&exports, &functions)?;
    validator::validate(&module.body, &scope, file_str, source)?;
    let prompt_body = self.evaluate_body(&module.body, &mut scope, warnings)?;
    Ok(ResolvedModule { functions, prompt_body, has_explicit_exports: !exports.is_empty(), explicit_exports: exports })
}
```

**`tokenize` function is 155 lines with 5-level nesting** - `lexer.rs:25-241`
**Confidence**: 92%
- Problem: The main `tokenize` function spans 155 lines of active logic (excluding the helper functions below it). It uses a manual character-by-character state machine with multiple nested `if/while` blocks reaching 4-5 levels of nesting (e.g., line 53-73: while > if > if > if). The frontmatter parsing, code fence detection, interpolation parsing, directive parsing, and regular text scanning are all interleaved in a single function. This makes it difficult to add new token types or modify existing ones without risk of breaking adjacent logic.
- Fix: Extract the main loop body into handler methods on a Lexer struct:
```rust
struct Lexer<'a> { chars: &'a [char], byte_offsets: &'a [usize], pos: usize, source: &'a str, file: &'a str, code_fence_backticks: usize }

impl Lexer<'_> {
    fn tokenize_frontmatter(&mut self, tokens: &mut Vec<Token>) -> Result<(), MdsError> { ... }
    fn tokenize_code_content(&mut self, tokens: &mut Vec<Token>) -> Result<(), MdsError> { ... }
    fn tokenize_directive(&mut self, tokens: &mut Vec<Token>) -> Result<(), MdsError> { ... }
    fn tokenize_interpolation(&mut self, tokens: &mut Vec<Token>) -> Result<(), MdsError> { ... }
    fn tokenize_text(&mut self, tokens: &mut Vec<Token>) -> Result<(), MdsError> { ... }
}
```

**`parse_interpolation_expr` has high cyclomatic complexity and deep nesting** - `parser.rs:460-549`
**Confidence**: 85%
- Problem: `parse_interpolation_expr` at 89 lines has 3 major branches (qualified call, simple call, variable), but the qualified-call branch alone contains nested `if let` / `if let` / `return if/else` reaching 4 levels of nesting (lines 470-521). The dot-notation error path (lines 491-521) is particularly convoluted with a conditional span calculation, then a conditional error constructor choice based on whether file context is available.
- Fix: Extract the dot-notation error and the qualified-call parsing into their own functions:
```rust
fn parse_qualified_call_or_dot_error(content: &str, dot_pos: usize, offset: usize, len: usize, file: &str, source: &str) -> Result<Interpolation, MdsError> { ... }
```

**`parse_args_inner` character-level state machine at 57 lines** - `parser.rs:557-615`
**Confidence**: 82%
- Problem: `parse_args_inner` implements a hand-rolled character-by-character parser with 4 boolean state variables (`in_string`, `escaped`, `paren_depth`, `string_char`) and nested `if/else if/else` branches reaching 4 levels deep. The cyclomatic complexity is approximately 12 from the character dispatch alone. Adding any new argument syntax (e.g., numeric literals, named params) would require modifying this tangled state machine.
- Fix: Consider factoring the character-classification into a small enum-based state machine, or at minimum extract the in-string and paren-depth tracking into helper methods. At the current complexity level this is manageable but sits right at the threshold.

### MEDIUM

**`ModuleCache::resolve` at 100 lines with 8 sequential validation checks** - `resolver.rs:63-166`
**Confidence**: 85%
- Problem: `resolve` performs symlink rejection, canonicalization, root-dir initialization, cache lookup, cycle detection, depth guard, path traversal prevention, file reading, size check, UTF-8 validation, file type validation, and recursive module processing -- all in a single method. While each check is individually simple, the function has 8 early-return error paths that create cyclomatic complexity around 10. The resolving-stack push/pop with manual cleanup (lines 150-158) adds to cognitive load.
- Fix: Group the validation steps into a helper:
```rust
fn validate_and_read(&self, canonical: &Path) -> Result<String, MdsError> { ... }
```
This would reduce `resolve` to: validate_and_read + mark_resolving + process_module + unmark + cache.

**`run` function in main.rs has 3 deeply nested command branches** - `main.rs:223-336`
**Confidence**: 80%
- Problem: The `run` function is 113 lines containing a top-level `match` with 3 arms (`Build`, `Check`, `Init`), where `Build` alone is 47 lines with 3 levels of nesting (match > if/else > if let). The stdin vs file distinction is duplicated across `Build` and `Check`.
- Fix: Extract each command handler into its own function (`run_build`, `run_check`, `run_init`). Extract the shared stdin-vs-file resolution pattern into a helper.

**Parameter threading: 5-6 parameters passed through every evaluator function** - `evaluator.rs:40-46, 100-106, 144-150`
**Confidence**: 83%
- Problem: Nearly every function in `evaluator.rs` takes the same 5 parameters: `scope`, `call_stack`, `total_iterations`, `warnings`, plus the node/expression being evaluated. This "parameter convoy" pattern makes signatures hard to read and easy to get wrong. The same pattern appears in `evaluate_nodes`, `evaluate_expr`, `resolve_args`, `invoke_function`, `call_function`, `call_qualified_function`, `evaluate_if`, `evaluate_for` -- 8 functions with near-identical signatures.
- Fix: Bundle the mutable evaluation state into a context struct:
```rust
struct EvalCtx<'a> {
    scope: &'a mut Scope,
    call_stack: HashSet<String>,
    total_iterations: usize,
    warnings: &'a mut Vec<String>,
}
```
Then each function takes `ctx: &mut EvalCtx` instead of 4 separate parameters.

**Repetitive `_at` constructor pairs in error.rs** - `error.rs:172-393`
**Confidence**: 80%
- Problem: `error.rs` defines 11 constructor methods, almost all appearing as paired `foo()` / `foo_at()` variants that differ only by the presence of span/source fields. This is 220 lines of highly repetitive code (lines 172-393). While not complex per-function, the file is 393 lines of almost entirely boilerplate, making it easy to forget to add the `_at` variant for a new error type.
- Fix: Consider a macro to generate the paired constructors:
```rust
macro_rules! error_constructors {
    ($name:ident, $variant:ident, { $($field:ident: $ty:ty),* }) => {
        pub fn $name($($field: impl Into<$ty>),*) -> Self { ... }
        pub fn ${name}_at($($field: impl Into<$ty>),*, file: &str, source: &str, offset: usize, len: usize) -> Self { ... }
    };
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`find_project_root` has a potentially unbounded loop** - `resolver.rs:16-28`
**Confidence**: 82%
- Problem: The `loop` in `find_project_root` walks up the directory tree until it finds a `.git` or `.mdsroot` marker, or reaches the filesystem root (when `dir.pop()` returns false). While this terminates in practice because filesystem trees are finite, it has no explicit upper bound. On a deeply nested path (e.g., 1000+ directories), this would perform 1000+ `exists()` syscalls per marker. The reliability principle from the project guidelines states "every loop and retry must have a fixed upper bound."
- Fix: Add a depth counter with a reasonable maximum:
```rust
fn find_project_root(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    for _ in 0..256 {
        for marker in [".git", ".mdsroot"] {
            if dir.join(marker).exists() { return dir; }
        }
        if !dir.pop() { return start.to_path_buf(); }
    }
    start.to_path_buf()
}
```

## Pre-existing Issues (Not Blocking)

*No pre-existing code -- this is a new codebase on the branch.*

## Suggestions (Lower Confidence)

- **`parse_import_directive` combines 3 import forms in 60 lines** - `parser.rs:348-408` (Confidence: 70%) -- Could be split into `parse_selective_import`, `parse_alias_import`, `parse_merge_import` for clarity, though the current structure is readable enough.

- **Magic number 256 for `MAX_NESTING_DEPTH`** - `parser.rs:12` (Confidence: 65%) -- The choice of 256 is undocumented beyond "prevents stack overflow." A comment explaining why 256 (vs 64 or 128) would help future maintainers. The `MAX_IMPORT_DEPTH` (64) and `MAX_CALL_DEPTH` (128) use different limits without explaining the rationale for the differences.

- **Integration test file at 2313 lines** - `tests/integration.rs` (Confidence: 62%) -- The single test file is quite large. Consider splitting into `tests/parser_integration.rs`, `tests/resolver_integration.rs`, `tests/evaluator_integration.rs` for navigability.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 4 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 6/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase is a new compiler written in well-structured Rust. Individual functions are mostly well-scoped, and the module decomposition (lexer/parser/validator/evaluator/resolver/scope) is sound. However, two functions (`process_module` at 140 lines and `tokenize` at 155 lines) significantly exceed the 50-line guideline and carry high cyclomatic complexity. The evaluator's parameter convoy pattern adds readability friction across 8 functions. None of these block correctness, but they will make the next round of feature additions (new directives, new expression types) harder than it needs to be.

Conditions for approval:
1. Extract `process_module` into named pipeline stages (HIGH -- highest impact refactor)
2. Bundle evaluator parameters into an `EvalCtx` struct (MEDIUM -- improves 8 function signatures)
3. The remaining items (lexer extraction, parse_interpolation_expr cleanup) can be addressed in follow-up PRs

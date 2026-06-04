# Complexity Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**`tokenize()` function is 216 lines with deep nesting and high cyclomatic complexity** - `src/lexer.rs:25-241`
**Confidence**: 95%
- Problem: The `tokenize()` function spans lines 25-241 (216 lines of logic). It contains a main `while` loop with 8+ top-level conditional branches, several of which contain their own inner `while` loops (e.g., frontmatter scanning at line 53, code content scanning at line 125, interpolation scanning at line 180, text scanning at line 210). Nesting reaches 4 levels in multiple places (e.g., the frontmatter block at lines 53-74 with `while > if > if > if`). The cyclomatic complexity is approximately 20+, well above the threshold of 10. This is the single most complex function in the codebase and violates the "explainable in 5 minutes" rule.
- Fix: Extract the distinct scanning phases into separate methods on a `Lexer` struct, similar to how the parser uses a `Parser` struct. Each token type already has a clear boundary:
```rust
struct Lexer<'a> {
    chars: Vec<char>,
    byte_offsets: Vec<usize>,
    source: &'a str,
    file: &'a str,
    pos: usize,
    code_fence_backticks: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn scan_frontmatter(&mut self) -> Result<(), MdsError> { /* lines 46-84 */ }
    fn scan_code_fence(&mut self) -> Result<bool, MdsError> { /* lines 91-118 */ }
    fn scan_code_content(&mut self) -> Result<(), MdsError> { /* lines 121-140 */ }
    fn scan_directive(&mut self) { /* lines 143-158 */ }
    fn scan_interpolation(&mut self) -> Result<(), MdsError> { /* lines 175-205 */ }
    fn scan_text(&mut self) { /* lines 208-233 */ }
}
```

**`process_module()` is 141 lines with 4 responsibilities and nesting depth 4** - `src/resolver.rs:184-323`
**Confidence**: 92%
- Problem: This function handles four distinct responsibilities in a single method: (1) tokenize and parse, (2) build scope from frontmatter and runtime vars, (3) process imports/exports/defines with collision checking, and (4) validate and evaluate. The `for node in &module.body` loop at line 221 contains a `match` with `Node::Export` arm (line 254) that itself has a nested `match` over 3 export variants, each with its own logic. Nesting reaches 4 levels (`fn > for > match > match`). Total cyclomatic complexity is approximately 15.
- Fix: Extract the per-node processing into a dedicated method:
```rust
fn process_node(
    &mut self,
    node: &Node,
    functions: &mut HashMap<String, FunctionDef>,
    has_explicit_exports: &mut bool,
    explicit_exports: &mut HashSet<String>,
    scope: &mut Scope,
    base_dir: &Path,
    runtime_vars: &HashMap<String, Value>,
    warnings: &mut Vec<String>,
    source_ctx: (&str, &str),
) -> Result<(), MdsError> { ... }
```
Or better yet, split the export handling into `process_export()` since the three export variants each contain significant logic.

**`parse_interpolation_expr()` is 90 lines with 3 control flow paths and deep nesting** - `src/parser.rs:460-549`
**Confidence**: 85%
- Problem: This function handles three distinct expression types (qualified call, simple call, variable reference) each with their own validation and error formatting logic. The qualified-call branch (lines 470-521) is particularly dense: it has a nested `if let` for the dot position, another for the paren position, and then a separate fallback path for the "dot but no parens" error case that itself branches on whether source context is available. This path alone has nesting depth 4 and the error formatting on lines 504-520 is duplicated between the `if !file.is_empty()` and `else` branches, differing only in which constructor is called.
- Fix: Extract the error branch into a helper and use early returns more aggressively:
```rust
fn dot_notation_error(content: &str, namespace: &str, field: &str,
    file: &str, source: &str, offset: usize, len: usize) -> MdsError {
    let msg = format!(
        "dot notation for variables is not supported in v0.1: '{content}'. \
         To call a function from an imported module use: {{{namespace}.{field}()}}",
    );
    if !file.is_empty() && !source.is_empty() {
        let interp_len = source[offset..].find('}')
            .map(|end| end + 1).unwrap_or(len + 2);
        MdsError::syntax_at(msg, file, source, offset, interp_len)
    } else {
        MdsError::syntax(msg)
    }
}
```

### MEDIUM

**`resolve()` is 98 lines with 7 early-return guard clauses** - `src/resolver.rs:64-161`
**Confidence**: 82%
- Problem: The function has 7 sequential validation guards before reaching core logic (symlink check, canonicalize, cache check, cycle check, depth check, path traversal check, size check). While each guard is individually clear, the sequential flow of 7 validation steps in one function makes it harder to see the happy path. The function mixes validation (lines 70-118) with I/O (lines 122-135) with state management (lines 145-158). At 98 lines it crosses the "warning" threshold of 50 lines.
- Fix: Extract validation guards into a `validate_resolve_path()` helper that returns the canonical path, leaving `resolve()` focused on cache-check, mark-resolving, process, unmark, cache-store:
```rust
fn validate_and_read(&self, path: &Path) -> Result<(PathBuf, String), MdsError> {
    // All 7 guards + file read in one focused function
}
```

**`parse_args_inner()` has 6 state-tracking variables for manual character-level parsing** - `src/parser.rs:557-615`
**Confidence**: 80%
- Problem: The function manually tracks 6 pieces of state (`args`, `current`, `in_string`, `string_char`, `escaped`, `paren_depth`) while iterating character-by-character. The main loop body has 8 conditional branches. While this is a common pattern for tokenizers, the combination of state variables and branching makes it moderately complex (cyclomatic complexity ~10). The logic is correct but requires careful mental simulation to verify.
- Fix: This is acceptable for a hand-rolled parser, but could be improved by extracting the string-scanning logic into a helper that returns the end position and content, reducing the number of active state variables in the main loop.

**Evaluator functions pass 5 parameters through the entire call chain** - `src/evaluator.rs` (multiple functions)
**Confidence**: 82%
- Problem: Every function in the evaluator (`evaluate_nodes`, `evaluate_expr`, `resolve_args`, `invoke_function`, `call_function`, `call_qualified_function`, `evaluate_if`, `evaluate_for`) passes the same 5 parameters: `scope`, `call_stack`, `total_iterations`, `warnings`, plus the node/expression being evaluated. This is a parameter list smell (5 parameters at the "warning" threshold). The repetition makes each call site verbose and error-prone if a parameter is forgotten.
- Fix: Bundle the mutable evaluation state into an `EvalContext` struct:
```rust
struct EvalContext<'a> {
    scope: &'a mut Scope,
    call_stack: HashSet<String>,
    total_iterations: usize,
    warnings: Vec<String>,
}
```
This reduces each function signature to `(node: &Node, ctx: &mut EvalContext)` -- 2 parameters instead of 5.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`error.rs` has 28 constructor methods with a repeating pattern, inflating the file to 441 lines** - `src/error.rs:176-441`
**Confidence**: 85%
- Problem: Every error variant has two constructors: `error_name()` (without span) and `error_name_at()` (with span). The `_at()` variants all follow the same pattern: call `at()` to build span+source, then construct the variant. This creates 14 pairs of nearly identical constructors. While each is individually simple, the sheer volume pushes the file well past 300 lines and makes it harder to find specific error variants.
- Fix: Consider a macro to generate the constructor pairs:
```rust
macro_rules! error_constructors {
    ($name:ident, $variant:ident { $($field:ident: $ty:ty),* }) => {
        pub fn $name($($field: impl Into<$ty>),*) -> Self { ... }
        pub fn paste::paste!{[<$name _at>]}(...) -> Self { ... }
    }
}
```
Or alternatively, make each constructor accept `Option<(&str, &str, usize, usize)>` so one function serves both use cases.

## Pre-existing Issues (Not Blocking)

No pre-existing issues -- all code is newly added in this branch.

## Suggestions (Lower Confidence)

- **`run()` in `main.rs` handles 3 subcommand arms in a single function (121 lines)** - `src/main.rs:223-344` (Confidence: 70%) -- The `Commands::Build` arm alone is 40 lines. Consider extracting each subcommand into its own function (`run_build`, `run_check`, `run_init`) for clarity.

- **`integration.rs` at 2320 lines is large but acceptable for a test file** - `tests/integration.rs` (Confidence: 65%) -- Each test is self-contained and short. Splitting into multiple test files by feature area (imports, loops, CLI, errors) would improve navigation but is not strictly a complexity issue.

- **`validate_expr()` has 3 match arms with repeated arity-checking logic** - `src/validator.rs:86-143` (Confidence: 65%) -- The `Expr::Call` and `Expr::QualifiedCall` arms both perform function lookup followed by arity check with nearly identical code. A shared helper could reduce duplication.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 3 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 6/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase is well-structured overall with good separation between lexer, parser, resolver, evaluator, and validator. Functions generally have clear responsibilities, nesting depth is usually controlled, and resource limits are consistently enforced with named constants. The main complexity concerns center on three long functions (`tokenize`, `process_module`, `parse_interpolation_expr`) and the evaluator's parameter threading pattern. The `tokenize()` function at 216 lines with ~20 cyclomatic complexity is the most pressing issue. The evaluator's 5-parameter threading is a moderate maintainability concern that will compound as the codebase evolves. None of these are blocking for a v0.1 implementation, but should be addressed before further feature additions.

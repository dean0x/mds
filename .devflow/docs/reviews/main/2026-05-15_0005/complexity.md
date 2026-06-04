# Complexity Review Report

**Branch**: main (d0624a2...HEAD)
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### HIGH

**`run()` in main.rs is 151 lines with deeply nested match arms** - `src/main.rs:435`
**Confidence**: 95%
- Problem: The `run()` function handles all three CLI subcommands (`Build`, `Check`, `Init`) in a single function spanning lines 435-585. The `Build` arm alone is ~70 lines with 4 levels of nesting (match -> match -> if/let -> if/let). This violates SRP and exceeds the 50-line function threshold by 3x.
- Fix: Extract each subcommand into its own handler function:
```rust
fn run(cli: Cli) -> Result<(), miette::Error> {
    let quiet = cli.quiet;
    match cli.command {
        Commands::Build { input, output, out_dir, vars, set_vars } =>
            run_build(input, output, out_dir, vars, set_vars, quiet),
        Commands::Check { input, vars, set_vars } =>
            run_check(input, vars, set_vars, quiet),
        Commands::Init { filename, force } =>
            run_init(filename, force, quiet),
    }
}
```

**`resolve_import()` is 81 lines with 3 import variants inlined** - `src/resolver.rs:365`
**Confidence**: 92%
- Problem: This function handles all three import forms (Alias, Merge, Selective) in a single match block spanning lines 365-445. The `Selective` arm alone is ~40 lines with 4 levels of nesting. This makes the function hard to understand within 5 minutes and difficult to test each import form in isolation.
- Fix: Extract each match arm into a helper:
```rust
fn resolve_import(&mut self, import: &ImportDirective, ...) -> Result<(), MdsError> {
    match import {
        ImportDirective::Alias { path, alias, offset } =>
            self.resolve_alias_import(path, alias, *offset, scope, ctx, warnings),
        ImportDirective::Merge { path, offset } =>
            self.resolve_merge_import(path, *offset, scope, ctx, warnings),
        ImportDirective::Selective { names, path, offset } =>
            self.resolve_selective_import(names, path, *offset, scope, ctx, warnings),
    }
}
```

**`canonicalize_and_check()` is 67 lines doing too many things** - `src/resolver.rs:73`
**Confidence**: 85%
- Problem: This function performs four distinct operations: symlink detection (lines 91-112), root_dir initialization (lines 115-118), import depth guard (lines 121-125), and path traversal prevention (lines 128-135). While each is individually simple, combining them into one function violates SRP and makes the security checks harder to audit independently.
- Fix: Keep `canonicalize_and_check` as the orchestrator but extract each security check into a named predicate:
```rust
fn canonicalize_and_check(&mut self, path: &Path) -> Result<(PathBuf, bool), MdsError> {
    let canonical = self.canonicalize_detecting_symlinks(path)?;
    self.init_root_dir_if_needed(&canonical);
    self.check_import_depth()?;
    self.check_path_traversal(&canonical)?;
    let is_md = canonical.extension().and_then(|e| e.to_str()) == Some("md");
    Ok((canonical, is_md))
}
```

### MEDIUM

**`parse_interpolation_expr()` is 64 lines with 3 levels of nested if-let** - `src/parser.rs:495`
**Confidence**: 88%
- Problem: This function spans lines 495-558 and handles three expression forms (qualified call, simple call, variable reference) with nested `if let` checks that reach 3 levels deep. The dot-notation branch (lines 505-530) interleaves parsing with error construction.
- Fix: Use early returns more aggressively by extracting the qualified-call parsing:
```rust
fn parse_interpolation_expr(...) -> Result<Interpolation, MdsError> {
    let content = content.trim();
    let len = content.len();
    if let Some(interp) = try_parse_qualified_call(content, offset, len, file, source)? {
        return Ok(interp);
    }
    if let Some(interp) = try_parse_call(content, offset, len)? {
        return Ok(interp);
    }
    parse_var_reference(content, offset, len)
}
```

**`parse_import_directive()` is 61 lines handling 3 import forms** - `src/parser.rs:348`
**Confidence**: 85%
- Problem: Function spans lines 348-408 handling selective, alias, and merge imports. While structured with early returns, the selective import parsing (lines 352-382) has 3 levels of nesting and mixes path parsing with name validation.
- Fix: Extract the selective import parsing into its own function.

**`parse_define_block()` is 60 lines with mixed concerns** - `src/parser.rs:285`
**Confidence**: 82%
- Problem: Function spans lines 285-344 and handles: directive syntax parsing, parameter name extraction, duplicate parameter detection, body parsing, and body whitespace trimming. Five distinct concerns in one function.
- Fix: Extract parameter parsing into `parse_param_list()`:
```rust
fn parse_param_list(params_str: &str, fn_name: &str) -> Result<Vec<String>, MdsError> {
    let params: Vec<String> = params_str.split(',').map(str::trim)
        .filter(|s| !s.is_empty()).map(str::to_owned).collect();
    let mut seen = HashSet::new();
    for param in &params {
        if !is_valid_identifier(param) { ... }
        if !seen.insert(param.as_str()) { ... }
    }
    Ok(params)
}
```

**`parse_args_inner()` is 59 lines with 4-level nesting in the character loop** - `src/parser.rs:566`
**Confidence**: 84%
- Problem: The character-by-character argument parsing loop (lines 584-616) has 4 levels of nesting: `for ch` -> `if escaped` / `else if in_string` -> `if ch == '\\'` / `else if ch == string_char` -> push. The interleaved state tracking (escaped, in_string, string_char, paren_depth) makes the control flow hard to follow.
- Fix: Consider a state-machine approach with an explicit `enum ScanState { Normal, InString { quote: char }, Escaped }` that collapses the nested conditionals into a single match.

**`invoke_function()` is 57 lines with multi-fault error handling** - `src/evaluator.rs:165`
**Confidence**: 80%
- Problem: This function (lines 165-221) combines recursion detection, scope push/pop, closure restoration, parameter binding, evaluation, and a double-fault error path. The LIFO invariant check (lines 208-215) adds complexity that could be extracted.
- Fix: Extract the closure-restore logic into a `restore_captured_scope()` helper, reducing `invoke_function` to the structural flow only.

**`resolve_output_path()` is 60 lines with 6-step precedence chain** - `src/main.rs:128`
**Confidence**: 82%
- Problem: This function implements a 6-level precedence chain for output path resolution. While each step is documented, the function is long enough that understanding the full precedence requires reading all 60 lines. The comments help, but the length still exceeds the threshold.
- Fix: Acceptable as-is given the thorough comments and linear flow (each step is an early return). Consider extracting steps 4-5 into a `resolve_from_config_or_out_dir()` helper if the function grows further.

**`error.rs` has 639 lines of repetitive constructor boilerplate** - `src/error.rs:1`
**Confidence**: 90%
- Problem: The file defines 14 error variants, each with a plain constructor and an `_at` constructor that adds source spans. This produces ~290 lines of near-identical constructor methods (lines 177-466). While each method is short, the aggregate duplication hurts maintainability -- adding a new error variant requires writing ~20 lines of boilerplate.
- Fix: Use a macro to generate the paired constructors:
```rust
macro_rules! error_constructors {
    ($variant:ident, $plain:ident, $at:ident, { $($field:ident: $ty:ty),* }) => {
        pub fn $plain($($field: impl Into<$ty>),*) -> Self {
            MdsError::$variant { $($field: $field.into(),)* span: None, src: None }
        }
        pub fn $at($($field: impl Into<$ty>),*, file: &str, source: &str, offset: usize, len: usize) -> Self {
            let (span, src) = at(file, source, offset, len);
            MdsError::$variant { $($field: $field.into(),)* span, src }
        }
    };
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`parse_body()` is at the 50-line boundary** - `src/parser.rs:108`
**Confidence**: 80%
- Problem: At exactly 50 lines (108-157), this function is at the warning threshold. The match has 7 arms, each straightforward but collectively verbose. If any new token types are added, it will immediately exceed the threshold.
- Fix: No immediate action needed, but the Token -> Node mapping for simple cases (CodeFence, CodeContent, EscapedBrace) could be extracted into a `simple_token_to_node()` helper.

**`validate_expr()` in validator.rs is 58 lines with structural duplication** - `src/validator.rs:86`
**Confidence**: 83%
- Problem: The `Call` and `QualifiedCall` arms (lines 99-142) are structurally near-identical: look up function, check arity, validate args. The only difference is the namespace lookup. This is duplication that inflates the function length.
- Fix: Extract a `validate_call_arity()` helper that takes the function ref and args, eliminating the duplicated arity check.

## Pre-existing Issues (Not Blocking)

_N/A -- this is a full initial review; all code is new._

## Suggestions (Lower Confidence)

- **`scan_frontmatter()` has 4-level nesting** - `src/lexer.rs:81` (Confidence: 70%) -- The while/if/if/if structure (lines 88-112) for matching the closing `---` fence reaches 4 levels. The inner logic could be extracted into a `try_match_closing_fence()` predicate.
- **`collect_export()` is 47 lines with 3 export forms** - `src/resolver.rs:317` (Confidence: 65%) -- Approaching the 50-line threshold. The Wildcard arm (lines 347-359) has collision checking that could be a separate `merge_wildcard_exports()` helper.
- **Named constants for resource limits are scattered** - `src/evaluator.rs:9-19`, `src/resolver.rs:44-47`, `src/main.rs:26-27` (Confidence: 72%) -- Resource limit constants (MAX_CALL_DEPTH, MAX_LOOP_ITERATIONS, MAX_FILE_SIZE, MAX_CONFIG_SIZE, etc.) are defined in 3 different files. A single `limits.rs` module would make the security posture auditable at a glance.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 3 | 5 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates good structural decomposition overall. Functions are generally well-focused, nesting is managed through early returns, and named constants replace most magic numbers. The compiler pipeline (lexer -> parser -> resolver -> evaluator) provides clean separation of concerns.

The primary complexity issues are concentrated in three areas: (1) the `run()` function in main.rs which monolithically handles all CLI subcommands, (2) the `resolve_import()` function which inlines all three import variant handlers, and (3) the repetitive error constructor boilerplate in error.rs. None of these are architectural problems -- they are extraction opportunities that would improve readability and testability.

Conditions for approval:
1. Extract `run()` into per-subcommand handlers (HIGH -- 151 lines is 3x the threshold)
2. Extract `resolve_import()` match arms into helper methods (HIGH -- 81 lines with deep nesting)
3. Consider extracting `canonicalize_and_check()` security checks for auditability (HIGH -- multiple security concerns in one function)

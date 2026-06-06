---
feature: mds-compiler
name: MDS Compiler
description: "Use when working on the MDS compilation pipeline, adding directives, modifying scope/variable handling, extending the module system, debugging output rendering, modifying CLI output behavior, or using the virtual filesystem / dependency tracking API. Keywords: lexer, parser, evaluator, resolver, validator, scope, frontmatter, interpolation, directive, import, export, include, define, for, if, elseif, negation, equality, Condition, CondValue, And, Or, logical operators, Param, default arguments, builtins, built-in functions, upper, lower, trim, replace, split, join, closure, lexical scope, prompt export, nested function calls, arg parsing, warnings, quiet mode, stdin, auto-detect, compile_file, compile_virtual, compile_with_deps, compile_str_with_deps, CompileOutput, dependency graph, FileSystem, NativeFs, VirtualFs, ModuleCache, resolve_path, resolve_key, resolve_source, dependencies, virtual filesystem, WASM, reexport, EvalContext, CapturedScope, IndexSet, Arc, exit_code, mds.json, output_dir, out_dir, default output, file output, MdsConfig, BuildConfig, load_config, resolve_output_path, derive_output_filename, non_exhaustive, pub(crate), run_build, run_check, run_init, MAX_TRAVERSAL_DEPTH, MAX_NESTING_DEPTH, MAX_DOT_SEGMENTS, MAX_ELSEIF_BRANCHES, MAX_LOGICAL_OPERANDS, MAX_FRONTMATTER_IMPORTS, object, map, Value::Object, dot notation, member access, MemberAccess, key-value iteration, resolve_dot_path, dot path, config.field, raw_frontmatter, strip_type_mds, prepend_frontmatter, frontmatter preservation, limits, dot segments, run_loop_body, evaluate_for_array, evaluate_for_key_value, validate_dot_path_parts, SerializedError, SerializedSpan, serialize, error serialization, path_to_str, resolve_base_dir, UTF-8 boundary, values_equal_runtime, evaluate_condition, ArityMismatch, BuiltinError, required_param_count, condvalue_to_value, Expr, parse_expr_inner, strip_trailing_directive_colon, FrontmatterImport, frontmatter imports, parse_frontmatter_imports_from_yaml."
category: architecture
directories: [crates/mds-core/src/, crates/mds-cli/src/, crates/mds-cli/tests/]
referencedFiles:
  - crates/mds-core/src/lib.rs
  - crates/mds-core/src/fs.rs
  - crates/mds-core/src/ast.rs
  - crates/mds-core/src/lexer.rs
  - crates/mds-core/src/parser.rs
  - crates/mds-core/src/parser_helpers.rs
  - crates/mds-core/src/validator.rs
  - crates/mds-core/src/resolver.rs
  - crates/mds-core/src/evaluator.rs
  - crates/mds-core/src/scope.rs
  - crates/mds-core/src/value.rs
  - crates/mds-core/src/error.rs
  - crates/mds-core/src/limits.rs
  - crates/mds-core/src/builtins.rs
  - crates/mds-cli/src/main.rs
  - crates/mds-core/tests/api_surface.rs
created: 2026-05-12
updated: 2026-06-06
---

# MDS Compiler

## Overview

MDS (Markdown Script) is a Rust compiler that transforms `.mds` files — Markdown with `@directives` and `{var}` interpolation — into plain Markdown. The primary use case is composable LLM prompt templates: authors write templates with variables, conditionals, loops, and reusable function fragments, then compile them to a final prompt string.

The compilation pipeline is strictly sequential: **lexer → parser → validator → resolver → evaluator → render**. Each layer has a single responsibility and communicates through typed interfaces rather than shared mutable state. The `resolver` is the orchestrator — it drives all other stages and manages the module cache used for imports.

## System Context

**Cargo workspace**: `mds-core` (library crate, publishes as `mds`) at `crates/mds-core/`; `mds-cli` (binary crate) at `crates/mds-cli/`. The workspace root `Cargo.toml` and `Cargo.lock` are at the repo root.

The library exposes public `compile*` / `check*` functions (see the existing API table — unchanged in v0.2.0). All carry `#[must_use]`. The public types include: `FileSystem`, `NativeFs`, `VirtualFs`, `ModuleCache`, `Value`, `MdsError`, `SerializedError`, `SerializedSpan`, `CompileOutput`, and constants `MAX_FILE_SIZE` / `MAX_TRAVERSAL_DEPTH`.

All compile/check functions funnel through `ModuleCache::resolve` / `ModuleCache::resolve_source`, the single entry point to the full pipeline.

**Warning collection pattern**: Warnings pass as `&mut Vec<String>` through the full pipeline. Nothing in the evaluator or resolver calls `eprintln!` directly.

The library module tree includes `pub(crate) mod builtins` (declared in `lib.rs`) which holds the 18 built-in functions added in v0.2.0.

## Component Architecture

### Limits Module (`crates/mds-core/src/limits.rs`)

All cross-pipeline defense-in-depth constants. Current set:

- `pub(crate) const MAX_DOT_SEGMENTS: usize = 32`
- `pub(crate) const MAX_NESTING_DEPTH: usize = 64`
- `pub(crate) const MAX_ELSEIF_BRANCHES: usize = 256`
- `pub(crate) const MAX_LOGICAL_OPERANDS: usize = 16` — caps leaf operands in a single `&&`/`||` expression
- `pub(crate) const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024`
- `pub(crate) const MAX_TRAVERSAL_DEPTH: usize = 256`
- `pub(crate) const MAX_FRONTMATTER_IMPORTS: usize = 256` — **new in PR #85**: caps `imports` entries in YAML frontmatter

When adding a limit used by more than one pipeline stage, add it here.

### Built-in Functions (`crates/mds-core/src/builtins.rs`) — New in v0.2.0

18 built-in functions organized into three groups. User-defined functions shadow built-ins with the same name (shadowing is checked first in `call_function`).

**String:** `upper`, `lower`, `trim`, `replace(str, from, to)`, `starts_with(str, prefix)`, `ends_with(str, suffix)`, `contains(str_or_array, needle)`, `slice(str_or_array, start[, end])`, `string(val)`

**Array:** `split(str, sep)`, `join(array, sep)`, `length(str_or_array)`, `first(array)`, `last(array)`, `reverse(str_or_array)`, `sort(array)`, `unique(array)`

**Type conversion:** `string(val)`, `number(val)`

Two public(crate) functions are the entire interface:
- `get_builtin(name: &str) -> Option<&'static BuiltinMeta>` — used by validator and evaluator for existence checks and arity bounds
- `call_builtin(name: &str, args: &[Value]) -> Result<Value, MdsError>` — dispatches to the private per-function implementations

`BuiltinMeta` carries `name`, `min_args`, `max_args`, and `handler: fn(&[Value]) -> Result<Value, MdsError>`. The `BUILTINS` static array is the single source of truth.

### AST (`crates/mds-core/src/ast.rs`)

**`Condition` enum** — six variants. **Breaking change from pre-PR #76**: leaves now hold `Expr` instead of `Vec<String>` / `CondValue`:

| Variant | Syntax | Notes |
|---|---|---|
| `Condition::Truthy(Expr)` | `@if flag:` or `@if func(x):` | truthy check on any expr |
| `Condition::Not(Expr)` | `@if !flag:` or `@if !func(x):` | negated truthy |
| `Condition::Eq(Expr, Expr)` | `@if func(a) == func(b):` | both sides are expressions |
| `Condition::NotEq(Expr, Expr)` | `@if role != "admin":` | both sides are expressions |
| `Condition::And(Vec<Condition>)` | `@if a && b:` | short-circuit AND |
| `Condition::Or(Vec<Condition>)` | `@if a \|\| b:` | short-circuit OR |

**`Condition` no longer has `path()` or `root()` methods.** These were removed in PR #76. Code that previously called `condition.root()` must now match on the variant directly. `Condition` intentionally does not derive `PartialEq` because `Expr::NumberLiteral(f64)` uses IEEE 754 semantics where `NaN != NaN`.

**`Expr` enum** — the unified expression type shared between interpolation `{ }` and directive conditions/iterables:

| Variant | Example |
|---|---|
| `Expr::Var(String)` | `{name}`, `@if flag:` |
| `Expr::Call { name, args }` | `{greet("Alice")}`, `@if func(x):` |
| `Expr::QualifiedCall { namespace, name, args }` | `{utils.greet("Alice")}` |
| `Expr::MemberAccess { object, fields }` | `{config.key}`, `@if config.debug:` |
| `Expr::StringLiteral(String)` | `@if x == "admin":` (RHS literal) |
| `Expr::NumberLiteral(f64)` | `@if count == 42:` (RHS literal) |
| `Expr::BooleanLiteral(bool)` | `@if x == true:` (RHS literal) |
| `Expr::NullLiteral` | `@if x == null:` (RHS literal) |

Bare literals as `Truthy`/`Not` operands (`@if true:`, `@if "admin":`) are rejected with a clear parse error — literals only make sense in comparisons, not truthy checks.

**`ForBlock.iterable: Expr`** — previously `Vec<String>`. Any expression accepted by `parse_expr_inner` is valid as a `@for` iterable, including function calls and qualified calls.

**`Arg` enum** — seven variants (unchanged from v0.2.0): `StringLiteral`, `NumberLiteral`, `BooleanLiteral`, `NullLiteral`, `Var`, `Call { name, args }`, `MemberAccess { object, fields }`.

**`Param` struct** — unchanged: `name: String`, `default: Option<CondValue>`. Note: `CondValue` and `Expr` literal variants are structurally identical (tracked as tech debt #78 — unification deferred).

**`required_param_count(params: &[Param]) -> usize`** — **moved to `ast.rs` in PR #76** (was in `evaluator.rs`). Both the validator and evaluator now import it from `crate::ast`. Do not look for it in `evaluator.rs`.

### Scope (`crates/mds-core/src/scope.rs`)

**`FunctionDef.params: Vec<Param>`** — unchanged. The `CapturedScope` struct, `Arc<FunctionDef>` in frames, and all `get_all_*` methods are unchanged.

### Parser (`crates/mds-core/src/parser.rs`, `parser_helpers.rs`)

**`parse_expr_inner(s: &str) -> Result<Expr, MdsError>`** (in `parser_helpers.rs`) — the unified expression parser. Used by both `parse_interpolation_expr` (for `{...}`) and directive parsers (`parse_simple_condition`, `parse_for_directive`). Handles: variable paths, dot-paths/member access, function calls, qualified calls, and literal values (string, number, boolean, null). **This is the key shared grammar point introduced in PR #76.**

**`strip_trailing_directive_colon(s: &str) -> Option<&str>`** (in `parser_helpers.rs`) — strips the trailing `:` from a directive line. Quote-and-paren-aware — does not strip a `:` inside a quoted string or parenthesized expression. Returns `None` if no valid trailing colon. Replaces four independent colon-stripping sites that preceded PR #76.

**`find_unquoted_operator`** and **`split_on_unquoted_op`** — both have paren-depth tracking (added in PR #76) so operators inside `func(a || b)` are not treated as condition-level operators.

**Condition precedence parser** (`parse_condition(s)` in `parser_helpers.rs`):
1. Splits on `||` first (lower precedence) → `Condition::Or` if multiple segments
2. Each segment through `parse_and_level` → splits on `&&` → `Condition::And`
3. Leaves through `parse_simple_condition` (truthy/not/eq/neq)

`count_leaf_operands(condition)` recursively counts leaf operands. Exceeding `MAX_LOGICAL_OPERANDS = 16` → syntax error.

**Default parameter parsing**: `parse_define_block` parses `name(param1, param2 = "default"):` syntax. Parameters with defaults must come after required parameters.

### Validator (`crates/mds-core/src/validator.rs`)

**`validate_condition`** — handles `And`/`Or` recursively. For leaves: uses `parse_expr_inner` implicitly through the AST — validates `Expr::Var` and `Expr::MemberAccess` roots against scope; validates `Expr::Call` / `Expr::QualifiedCall` against known functions and builtins.

**`validate_expr` for `Expr::Call`** — checks builtins before rejecting as undefined:
1. Try `scope.get_function(name)` (user-defined) — check arity with `required_param_count`/`total`
2. Try `crate::builtins::get_builtin(name)` — check arity with `meta.min_args`/`meta.max_args`
3. Otherwise: `MdsError::undefined_fn_at`

Imports `required_param_count` from `crate::ast` (not from evaluator).

### Evaluator (`crates/mds-core/src/evaluator.rs`)

**`evaluate_expr(expr: &Expr, scope, ctx) -> Result<Value, MdsError>`** — evaluates any `Expr` to a `Value`. Shared entry point for interpolation and directive evaluation. Handles all eight `Expr` variants.

**`call_function` returns `Result<Value, MdsError>`** — changed from `Result<String>` in v0.2.0. The call sites convert to `String` via `.to_string()` when needed for interpolation output.

**`values_equal_runtime(lhs: &Value, rhs: &Value) -> bool`** — **new in PR #76**, replaces the old `values_equal(Value, CondValue)`. Used by `Eq`/`NotEq` condition evaluation where both sides are now runtime `Value` after expression evaluation.

**`evaluate_condition`** — dispatches all six `Condition` variants. `And`/`Or` short-circuit. Leaf variants call `evaluate_expr` on each `Expr` arm.

**`condvalue_to_value(cv: &CondValue) -> Value`** — converts compile-time `CondValue` literals to runtime `Value`. Used in `invoke_function` to supply default argument values.

`required_param_count` is now imported from `crate::ast` (the function moved from evaluator to ast in PR #76).

### Resolver (`crates/mds-core/src/resolver.rs`)

**Frontmatter imports** (new in PR #85):

`FrontmatterImport` enum with three variants:
- `Alias { path: String, alias: String }` — `imports: [{path: "x.mds", as: alias}]`
- `Merge { path: String }` — `imports: [{path: "x.mds"}]`
- `Selective { path: String, names: Vec<String> }` — `imports: [{path: "x.mds", names: [greet]}]`

Key functions:
- `parse_frontmatter_imports_from_yaml(val: &serde_yaml_ng::Value) -> Result<Vec<FrontmatterImport>, MdsError>` — parses the `imports` YAML value
- `parse_frontmatter_imports(raw: &str) -> Result<Vec<FrontmatterImport>, MdsError>` — parses from raw YAML frontmatter string
- `build_scope_from_frontmatter(fm, is_md, runtime_vars) -> Result<(Scope, Vec<FrontmatterImport>), MdsError>` — now returns both the scope and the frontmatter import list (previously returned `Scope` only)

**Resolution order**: frontmatter imports are resolved BEFORE body `@import` directives. A namespace collision between frontmatter and body is a hard compile error (uses `name_collision()` same as body imports).

**`.md` file handling**: The `imports` key is treated as a regular variable in plain `.md` files. Only `.mds` files and `.md` files with `type: mds` in frontmatter trigger import processing. An empty `names: []` selective import is a compile error.

**Output stripping**: `imports` is stripped from the compiled output (like `type: mds`).

**Limit**: `MAX_FRONTMATTER_IMPORTS = 256` in `limits.rs` — enforced in `parse_frontmatter_imports_from_yaml`.

### Error System (`crates/mds-core/src/error.rs`)

**`ArityMismatch` variant** — fields: `expected_min: usize`, `expected_max: usize`. Display uses `format_arity(min, max)`: `"expected 1 argument"` (min==max==1), `"expected N arguments"` (min==max), `"expected M-N arguments"` (min!=max). Always pass both min and max to `MdsError::arity` / `MdsError::arity_at`.

**`BuiltinError` variant** — `{ message, span, src }`. Constructor: `MdsError::builtin_error(msg)`. No `_at` variant yet.

## Component Interactions

The data flow is unchanged: lexer → parser → resolver → validator → evaluator → lib::build_output. Key cross-component dependencies:

- **`ast.rs`**: defines `required_param_count` — imported by both `validator.rs` and `evaluator.rs`
- **`parser_helpers.rs`**: `parse_expr_inner` is the shared grammar entry point for both interpolation (`parser.rs`) and directive parsing (`parser_helpers.rs`)
- **`resolver.rs`**: `build_scope_from_frontmatter` now returns `(Scope, Vec<FrontmatterImport>)` — callers must destructure and resolve frontmatter imports before processing body imports
- **`builtins.rs`**: `get_builtin` is called from both `validator.rs` and `evaluator.rs`

## Integration Patterns

### Adding a Built-in Function

1. Add a `BuiltinMeta { name, min_args, max_args, handler }` entry to the `BUILTINS` static slice in `builtins.rs`
2. Add a `"name" => builtin_name(args)` arm in `call_builtin`'s match
3. Write the private `fn builtin_name(args: &[Value]) -> Result<Value, MdsError>` using `require_string` / `require_string_at` helpers
4. Validator and evaluator automatically recognize the new function through `get_builtin` — no changes needed there

### Adding a New Arg Variant

If you add an eighth `Arg` variant, update all three sites:
1. `parse_single_arg_inner` in `parser_helpers.rs` — construct the new variant
2. `resolve_args` in `evaluator.rs` — evaluate to a `Value`
3. `validate_var_args` in `validator.rs` — pre-evaluation validity check

### Adding a New Directive

1. Add a new variant to `Node` in `ast.rs`
2. Parse: add a branch in `Parser::parse_directive()` matching the `@name` prefix
3. Validate: add a match arm in `validate_node()`
4. Resolve: handle in `collect_definitions_and_imports` (file I/O) or `build_scope_from_frontmatter` (scope-only)
5. Evaluate: add a match arm in `evaluate_nodes()` — pass `ctx` through

### Adding a New Expression Form

If you need a new `Expr` variant:
1. Add to `Expr` enum in `ast.rs`
2. Add parsing in `parse_expr_inner` in `parser_helpers.rs`
3. Add evaluation in `evaluate_expr` in `evaluator.rs`
4. Add validation in `validate_expr` in `validator.rs`

All four sites have exhaustive matches — missing arms produce compile errors.

### Adding a Frontmatter-Processed Key

Follow the pattern used by `type: mds` and `imports`:
1. Check for the key in `build_scope_from_frontmatter` in `resolver.rs`
2. Remove it from the scope or handle it before passing remaining keys to the scope builder
3. Return the extracted value alongside the `Scope` in the function return type
4. Strip from output by adding to the exclusion list in `strip_type_mds`

## Anti-Patterns

- **Calling `eprintln!` from evaluator or resolver code** — use `ctx.warnings` or `warnings: &mut Vec<String>`.
- **Calling `evaluate` before `validate`** — the evaluator trusts all references exist.
- **Creating `ModuleCache` per-module instead of per-compile** — destroys caching.
- **Using bare `MdsError::syntax(msg)` when source context is available** — prefer `syntax_at`.
- **Directly interpolating `Value::Object`** — `{obj}` is a runtime error; use `{obj.key}`.
- **Adding a new `Arg` variant without updating all three match sites** — parser, evaluator, validator all match exhaustively.
- **Adding a new `Condition` variant without updating `validate_condition`** — compound conditions require recursive traversal.
- **Adding a new `Expr` variant without updating all four match sites** — parser, evaluator, validator, and any direct Expr matches in tests.
- **Calling `condition.root()` or `condition.path()`** — these methods were removed in PR #76. Match on the variant directly.
- **Looking for `required_param_count` in `evaluator.rs`** — it moved to `ast.rs` in PR #76.
- **Using `values_equal(Value, CondValue)` for condition equality** — that function was replaced by `values_equal_runtime(Value, Value)` in PR #76.
- **Calling `arity` / `arity_at` with a single `expected` value** — both now require `expected_min` and `expected_max`.
- **Placing a required param after a param with a default** — the parser rejects this at parse time.
- **Matching exhaustively on `MdsError` or `Value` in external code** — both are `#[non_exhaustive]`.
- **Processing body `@import` before frontmatter `imports`** — frontmatter imports must resolve first; the resolver's main path already does this correctly, but any custom resolution code must respect the order.
- **Treating `imports` as a user variable in `.mds` files** — it is a reserved frontmatter key in `.mds` and in `.md` files with `type: mds`. Plain `.md` files without `type: mds` keep `imports` as a regular variable.

## Gotchas

- **`Condition` does not derive `PartialEq`** — `Expr::NumberLiteral(f64)` uses IEEE 754 where `NaN != NaN`. Implement `PartialEq` manually if needed.
- **`Condition` leaves now hold `Expr`, not `Vec<String>`** — code written against the pre-PR #76 AST will not compile. The path is through `evaluate_expr`, not a field lookup.
- **`parse_expr_inner` is the unified grammar** — both `{interpolation}` and `@directive` expressions go through the same function. A bug in `parse_expr_inner` affects both contexts.
- **`strip_trailing_directive_colon` is paren-aware** — `@if func(a:b):` strips only the final colon. Earlier naive colon stripping would have broken on such inputs.
- **`required_param_count` is in `ast.rs`, not `evaluator.rs`** — importing from the wrong module is a compile error.
- **`MAX_LOGICAL_OPERANDS = 16` is a leaf count, not a per-level count** — `a && b || c && d` has 4 leaf operands.
- **`And`/`Or` conditions are validated conservatively** — the validator checks all operands even though evaluation short-circuits at runtime.
- **Frontmatter `imports` is stripped from output** — it does not appear in the rendered Markdown.
- **Empty `names: []` in frontmatter selective import is a compile error** — not a no-op.
- **`build_scope_from_frontmatter` now returns a tuple** — `(Scope, Vec<FrontmatterImport>)`. Any code calling it must be updated to destructure.
- **`CondValue` and `Expr` literal types are near-duplicates** — tracked as tech debt issue #78. Do not unify them without a dedicated PR.
- **`call_function` returns `Value`, not `String`** — code that previously expected `call_function` to return `Result<String>` must be updated.
- **Key-value iteration sorts keys alphabetically** — YAML insertion order is not preserved.
- **`call_stack` is `Vec`, not `HashSet`** — recursion detection uses O(n) scan at MAX_CALL_DEPTH=128.

## Key Files

- `crates/mds-core/src/limits.rs` — all cross-pipeline resource limits; `MAX_FRONTMATTER_IMPORTS = 256` added in PR #85
- `crates/mds-core/src/ast.rs` — all AST types; `Condition` variants now hold `Expr`; `ForBlock.iterable: Expr`; `Param` struct; `required_param_count` function (moved here from evaluator in PR #76)
- `crates/mds-core/src/builtins.rs` — 18 built-in functions; `BuiltinMeta` struct; `get_builtin` / `call_builtin` entry points
- `crates/mds-core/src/parser_helpers.rs` — `parse_expr_inner` (shared expression grammar); `strip_trailing_directive_colon`; condition precedence parser (`parse_condition`, `parse_and_level`, `count_leaf_operands`); default param parsing; `find_unquoted_operator` and `split_on_unquoted_op` with paren-depth tracking
- `crates/mds-core/src/evaluator.rs` — `evaluate_expr` (Expr → Value); `call_function` returns `Value`; `values_equal_runtime`; `condvalue_to_value`; `And`/`Or` short-circuit in `evaluate_condition`
- `crates/mds-core/src/validator.rs` — builtin-aware `validate_expr`; range arity checks; recursive `validate_condition` for `And`/`Or`; imports `required_param_count` from `crate::ast`
- `crates/mds-core/src/resolver.rs` — orchestrator; `ModuleCache`; `FrontmatterImport` enum and parse functions; `build_scope_from_frontmatter` returns `(Scope, Vec<FrontmatterImport>)`; import semantics; security enforcement
- `crates/mds-core/src/error.rs` — `ArityMismatch` with `expected_min`/`expected_max`; `BuiltinError` variant; `format_arity` helper
- `crates/mds-core/src/lib.rs` — public API; declares `pub(crate) mod builtins`; `strip_type_mds` and `prepend_frontmatter` for frontmatter preservation
- `crates/mds-cli/src/main.rs` — CLI: `run_build`/`run_check`/`run_init`; `exit_code`; `resolve_output_path`; `load_config`
- `crates/mds-core/tests/api_surface.rs` — public API regression tests; update when adding public symbols

## Related

- ADR-008: bundles related language features in single PR (applied to v0.2.0 — built-ins, default args, and logical operators shipped together; applied to expression directives #74 which touched parser/validator/evaluator in one PR)
- ADR-010: reuse `parse_expr_inner` across interpolation and directive parsing to avoid grammar duplication
- `crates/mds-core/src/resolver.rs` — canonical reference for module system, import semantics, `FrontmatterImport`, security guards, `Arc<ResolvedModule>` cache
- `crates/mds-core/src/evaluator.rs` — canonical reference for `EvalContext`, `evaluate_expr`, directive execution, closure restore, call-depth guards
- `crates/mds-core/src/scope.rs` — canonical reference for `CapturedScope`, `Arc<FunctionDef>`, closure capture API
- `crates/mds-core/src/ast.rs` — canonical reference for all AST types; start here for new argument or expression forms
- `crates/mds-cli/tests/` — end-to-end tests across 10 categorized files (`language.rs`, `objects.rs`, `imports.rs`, `errors.rs`, `cli_build.rs`, `cli_commands.rs`, `security.rs`, `frontmatter.rs`, `warnings.rs`) plus `common/mod.rs`
- Tech debt: issue #77 (ScanState extraction), #78 (CondValue/Expr unification), #79 (parse_interpolation_expr delegation), #80 (parse_simple_condition complexity)

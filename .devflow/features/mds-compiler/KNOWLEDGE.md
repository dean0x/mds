---
feature: mds-compiler
name: MDS Compiler
description: "Use when working on the MDS compilation pipeline, adding directives, modifying scope/variable handling, extending the module system, debugging output rendering, modifying CLI output behavior, or using the virtual filesystem / dependency tracking API. Keywords: lexer, parser, evaluator, resolver, validator, scope, frontmatter, interpolation, directive, import, export, include, define, for, if, elseif, negation, equality, Condition, CondValue, And, Or, logical operators, Param, default arguments, builtins, built-in functions, upper, lower, trim, replace, split, join, closure, lexical scope, prompt export, nested function calls, arg parsing, warnings, quiet mode, stdin, auto-detect, compile_file, compile_virtual, compile_with_deps, compile_str_with_deps, CompileOutput, dependency graph, FileSystem, NativeFs, VirtualFs, ModuleCache, resolve_path, resolve_key, resolve_source, dependencies, virtual filesystem, WASM, reexport, EvalContext, CapturedScope, IndexSet, Arc, exit_code, mds.json, output_dir, out_dir, default output, file output, MdsConfig, BuildConfig, load_config, resolve_output_path, derive_output_filename, non_exhaustive, pub(crate), run_build, run_check, run_init, MAX_TRAVERSAL_DEPTH, MAX_NESTING_DEPTH, MAX_DOT_SEGMENTS, MAX_ELSEIF_BRANCHES, MAX_LOGICAL_OPERANDS, object, map, Value::Object, dot notation, member access, MemberAccess, key-value iteration, resolve_dot_path, dot path, config.field, raw_frontmatter, strip_type_mds, prepend_frontmatter, frontmatter preservation, limits, dot segments, run_loop_body, evaluate_for_array, evaluate_for_key_value, validate_dot_path_parts, SerializedError, SerializedSpan, serialize, error serialization, path_to_str, resolve_base_dir, UTF-8 boundary, values_equal, evaluate_condition, resolve_condition_path, ArityMismatch, BuiltinError, required_param_count, condvalue_to_value."
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
updated: 2026-06-02
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

The library module tree now includes `pub(crate) mod builtins` (declared in `lib.rs` at line 41), which holds the 18 built-in functions added in v0.2.0.

## Component Architecture

### Limits Module (`crates/mds-core/src/limits.rs`)

All cross-pipeline defense-in-depth constants. As of v0.2.0:

- `pub(crate) const MAX_DOT_SEGMENTS: usize = 32`
- `pub(crate) const MAX_NESTING_DEPTH: usize = 64`
- `pub(crate) const MAX_ELSEIF_BRANCHES: usize = 256`
- `pub(crate) const MAX_LOGICAL_OPERANDS: usize = 16` — **new in v0.2.0**; caps the total number of leaf operands in a single `&&` or `||` expression (counted recursively). Prevents adversarial condition trees.
- `pub(crate) const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024`
- `pub(crate) const MAX_TRAVERSAL_DEPTH: usize = 256`

When adding a limit used by more than one pipeline stage, add it here.

### Built-in Functions (`crates/mds-core/src/builtins.rs`) — New in v0.2.0

18 built-in functions organized into three groups. User-defined functions shadow built-ins with the same name (shadowing is checked first in `call_function`).

**String:** `upper`, `lower`, `trim`, `replace(str, from, to)`, `starts_with(str, prefix)`, `ends_with(str, suffix)`, `contains(str_or_array, needle)`, `slice(str_or_array, start[, end])`, `string(val)`

**Array:** `split(str, sep)`, `join(array, sep)`, `length(str_or_array)`, `first(array)`, `last(array)`, `reverse(str_or_array)`, `sort(array)`, `unique(array)`

**Type conversion:** `string(val)`, `number(val)`

Two public(crate) functions are the entire interface:
- `get_builtin(name: &str) -> Option<&'static BuiltinMeta>` — used by validator and evaluator for existence checks and arity bounds
- `call_builtin(name: &str, args: &[Value]) -> Result<Value, MdsError>` — dispatches to the private per-function implementations

`BuiltinMeta` carries `name: &'static str`, `min_args: usize`, `max_args: usize`, and `handler: fn(&[Value]) -> Result<Value, MdsError>`. The `BUILTINS` static array is the single source of truth — `call_builtin` dispatches via `get_builtin(name).handler` with no separate match arm. Some functions have `min_args != max_args` (e.g., `slice` is 2–3 args).

**`slice` semantics**: string slicing uses character (Unicode scalar value) indices, not byte offsets. `slice("café", 0, 4)` returns `"café"` (4 chars). Indices are clamped to the character count.

**`length` semantics**: returns Unicode scalar value count for strings (`length("café") == 4`), not bytes.

**`sort` homogeneity**: rejects mixed-type arrays at runtime with `MdsError::BuiltinError`.

**`first`/`last` on empty array**: returns `Value::Null` (not an error).

**`contains`**: works on both strings (substring check) and arrays (element equality).

### AST (`crates/mds-core/src/ast.rs`)

**`Condition` enum** — now has six variants (four leaf + two compound, added in v0.2.0):

| Variant | Syntax | Notes |
|---|---|---|
| `Condition::Truthy(Vec<String>)` | `@if flag:` | truthy check |
| `Condition::Not(Vec<String>)` | `@if !flag:` | negated truthy |
| `Condition::Eq(Vec<String>, CondValue)` | `@if role == "admin":` | strict equality |
| `Condition::NotEq(Vec<String>, CondValue)` | `@if role != "admin":` | strict inequality |
| `Condition::And(Vec<Condition>)` | `@if a && b:` | **new** — short-circuit AND |
| `Condition::Or(Vec<Condition>)` | `@if a \|\| b:` | **new** — short-circuit OR |

`Condition::path()` returns `&[]` for `And`/`Or` variants — callers that need to handle compound conditions must match on the variant directly. `Condition::root()` returns an error for `And`/`Or`; only call it on leaf variants.

**`Arg` enum** — now seven variants (three added in v0.2.0):

| Variant | Meaning |
|---|---|
| `Arg::StringLiteral(String)` | Quoted string: `"hello"` |
| `Arg::NumberLiteral(f64)` | Numeric literal: `func(42)` — **new** |
| `Arg::BooleanLiteral(bool)` | Boolean literal: `func(true)` — **new** |
| `Arg::NullLiteral` | Null literal: `func(null)` — **new** |
| `Arg::Var(String)` | Variable reference |
| `Arg::Call { name, args }` | Nested function call |
| `Arg::MemberAccess { object, fields }` | Object field access as argument |

**`Param` struct** — new in v0.2.0, replaces `Vec<String>` for function parameters:

```rust
pub struct Param {
    pub name: String,
    pub default: Option<CondValue>,  // None = required, Some = optional with default
}
```

`Param::required(name)` is a convenience constructor for parameters with no default. Parameters without defaults must appear before parameters with defaults — the parser enforces this.

**`DefineBlock.params: Vec<Param>`** — previously `Vec<String>`. Any code building `DefineBlock` must now use `Vec<Param>`.

**`CondValue`** is reused as the type for default parameter values (same four variants: `String`, `Number`, `Bool`, `Null`).

### Scope (`crates/mds-core/src/scope.rs`)

**`FunctionDef.params: Vec<Param>`** — changed from `Vec<String>` in v0.2.0. The resolver fills `captured` after `FunctionDef::from(&DefineBlock)` as before (no change to capture flow).

The `CapturedScope` struct, `Arc<FunctionDef>` in frames, and all `get_all_*` methods are unchanged.

### Parser (`crates/mds-core/src/parser.rs`, `parser_helpers.rs`)

**Condition precedence parser** (new in v0.2.0 in `parser_helpers.rs`):

`parse_condition(s)` now handles `&&` and `||`:
1. Splits on `||` first (lower precedence) — produces `Condition::Or` if multiple segments
2. Each segment is parsed by `parse_and_level` — splits on `&&`, produces `Condition::And` if multiple segments
3. Each leaf is handled by `parse_simple_condition` (existing truthy/not/eq/neq logic)

Precedence: `||` binds less tightly than `&&`. Example: `a && b || c` → `Or([And([a, b]), c])`.

`count_leaf_operands(condition)` recursively counts leaf operands. After parsing, if the total count exceeds `MAX_LOGICAL_OPERANDS = 16`, a syntax error is returned.

**Default parameter parsing**: `parse_define_block` now parses `name(param1, param2 = "default"):` syntax. Each parameter is parsed to a `Param` struct. Parameters with defaults must come after required parameters; the parser returns an error otherwise. Duplicate parameter names are still rejected.

**`@else` without colon**, dot-path helpers (`validate_dot_path_parts`), argument nesting, interpolation disambiguation — all unchanged from the prior KB entry.

### Validator (`crates/mds-core/src/validator.rs`)

**`validate_condition`** — updated to handle `And`/`Or` recursively:

For `Condition::And(operands)` or `Condition::Or(operands)`: validates all operands recursively (conservative — does not short-circuit validation even though evaluation short-circuits at runtime).

For leaf variants: extracts `root` via `condition.root()?` and checks it exists in scope.

**`validate_expr` for `Expr::Call`** — now checks builtins before rejecting as undefined:
1. Try `scope.get_function(name)` (user-defined) — check arity with `required_param_count`/`total`
2. Try `crate::builtins::get_builtin(name)` — check arity with `meta.min_args`/`meta.max_args`
3. Otherwise: `MdsError::undefined_fn_at`

**Arity checks now use range**: `args.len() < required || args.len() > total` instead of `args.len() != expected`. Required = count of params without defaults; total = all params including optional.

**`validate_var_args`** — updated to match all seven `Arg` variants. `NumberLiteral`, `BooleanLiteral`, and `NullLiteral` arms require no validation (grouped with `StringLiteral` as no-ops).

**`Node::Define` body validation**: still uses `scope.set_var(&param.name, Value::Array(vec![]))` for each param — accesses `param.name` instead of the old plain `String`.

### Evaluator (`crates/mds-core/src/evaluator.rs`)

**`call_function` returns `Result<Value, MdsError>`** — changed from `Result<String>` in v0.2.0. This allows built-ins to return non-String values (arrays, booleans, numbers). The call sites in `evaluate_expr` convert to String via `.to_string()` when needed for interpolation.

**Built-in dispatch in `call_function`**:

```rust
fn call_function(name: &str, args: &[Value], scope: &mut Scope, ctx: &mut EvalContext) -> Result<Value, MdsError> {
    // User-defined functions take priority (shadowing built-ins).
    if let Some(func) = scope.get_function(name).cloned() {
        return invoke_function(&func, name, args, scope, ctx).map(Value::String);
    }
    // Fall back to built-ins.
    if let Some(meta) = crate::builtins::get_builtin(name) {
        if args.len() < meta.min_args || args.len() > meta.max_args {
            return Err(MdsError::arity(name, meta.min_args, meta.max_args, args.len()));
        }
        return crate::builtins::call_builtin(name, args);
    }
    Err(MdsError::undefined_fn(name))
}
```

Key consequence: `invoke_function` still returns `Result<String, MdsError>` (user-defined function bodies render to string), wrapped via `.map(Value::String)`. Built-ins return `Value` directly.

**`resolve_args`** — handles all seven `Arg` variants. `NumberLiteral`, `BooleanLiteral`, `NullLiteral` map directly to `Value::Number`, `Value::Boolean`, `Value::Null`. The `Arg::Call` arm calls `call_function` and returns its `Value` directly (not `.to_string()` — preserving the actual type for nested calls passed to built-ins).

**`condvalue_to_value(cv: &CondValue) -> Value`** — new `pub(crate)` function converting compile-time `CondValue` literals to runtime `Value`. Used in `invoke_function` to supply default argument values when optional params are omitted.

**`required_param_count(params: &[Param]) -> usize`** — new `pub(crate)` function counting params with `default.is_none()`. Used by both evaluator and validator (validator imports it via `use crate::evaluator::required_param_count`).

**`invoke_function` arity check** — now uses range: `args.len() < required || args.len() > total`. Missing optional params are filled from their `CondValue` defaults via `condvalue_to_value`.

**`evaluate_condition`** — now dispatches `And`/`Or` variants with short-circuit evaluation:
- `And`: returns `false` on the first operand that evaluates to false
- `Or`: returns `true` on the first operand that evaluates to true

**`evaluate_if`** — unchanged structure, but now calls `evaluate_condition` which handles compound conditions. `debug_assert!` still checks `block.elseif_branches.len() <= MAX_ELSEIF_BRANCHES`.

### Error System (`crates/mds-core/src/error.rs`)

**`ArityMismatch` variant** — changed fields in v0.2.0:
- `expected: usize` → `expected_min: usize` + `expected_max: usize`
- Display: `format_arity(min, max)` helper: `"expected 1 argument"` (if min==max==1), `"expected N arguments"` (if min==max), `"expected M-N arguments"` (if min!=max)

**`BuiltinError` variant** — new in v0.2.0:
```rust
#[error("{message}")]
#[diagnostic(code(mds::builtin_type_error))]
BuiltinError {
    message: String,
    span: Option<SourceSpan>,
    src: Option<Arc<miette::NamedSource<String>>>,
}
```
Constructor: `MdsError::builtin_error(msg: impl Into<String>) -> Self`. No `_at` variant currently — built-in errors do not carry source spans.

Both `ArityMismatch` and `BuiltinError` are included in the `serialize()` method's span-bearing variant match arm.

The `arity` and `arity_at` constructors now take `(name, expected_min, expected_max, got, ...)` — callers that pass a single `expected` value must be updated to pass the same value for both `min` and `max`.

## Component Interactions

The data flow is unchanged (lexer → parser → resolver → validator → evaluator → lib::build_output). The scope pipeline now includes:

- **Resolver**: calls `required_param_count` from `evaluator.rs` when verifying arity during closure capture (not directly — arity checking is in validate/evaluate)
- **Validator**: imports `required_param_count` from `evaluator.rs`; imports `get_builtin` from `builtins` to recognize built-in names
- **Evaluator**: imports `get_builtin` and `call_builtin` from `builtins`; `call_function` returns `Value` (not `String`)

## Integration Patterns

### Adding a Built-in Function

1. Add a `BuiltinMeta { name, min_args, max_args }` entry to the `BUILTINS` static slice in `crates/mds-core/src/builtins.rs`
2. Add a `"name" => builtin_name(args)` arm in `call_builtin`'s match
3. Write the private `fn builtin_name(args: &[Value]) -> Result<Value, MdsError>` using `require_string` / `require_string_at` helpers for type errors
4. The validator and evaluator automatically recognize the new function through `get_builtin` — no changes needed there

User-defined functions with the same name shadow the built-in — this is by design.

### Adding a New Arg Variant

If you add an eighth `Arg` variant, update all three sites that match on `Arg`:
1. `parse_single_arg_inner` in `crates/mds-core/src/parser_helpers.rs` — construct the new variant
2. `resolve_args` in `crates/mds-core/src/evaluator.rs` — evaluate to a `Value`
3. `validate_var_args` in `crates/mds-core/src/validator.rs` — pre-evaluation validity check

All three have exhaustive matches — a missing arm produces a compile error.

### Adding a New Directive

1. Add a new variant to `Node` in `ast.rs`
2. Lex: directives are already captured as `Token::Directive` — no change unless new syntax
3. Parse: add a branch in `Parser::parse_directive()` matching the `@name` prefix
4. Validate: add a match arm in `validate_node()`
5. Resolve: handle in `collect_definitions_and_imports` (file I/O) or `build_scope_from_frontmatter` (scope-only)
6. Evaluate: add a match arm in `evaluate_nodes()` — pass `ctx` through for warnings/iteration tracking

### Adding a New Value Type

Add the new variant to `Value` and update all internal match sites: `from_yaml`, `from_json`, `Display`, `is_truthy`, `type_name`, `as_array`, and consider `resolve_dot_path` field traversal.

### Warning-Emitting Code

Accept `warnings: &mut Vec<String>` and push to it. Inside the evaluator, access via `ctx.warnings`. Never call `eprintln!`. The `MAX_WARNINGS = 1,000` limit in the evaluator silently drops further pushes once reached.

### Error Reporting Pattern

Use `_at` constructors when source context is available. For arity errors, always supply both `expected_min` and `expected_max` to `MdsError::arity` / `MdsError::arity_at`. For built-in type errors, use `MdsError::builtin_error(msg)` (no `_at` variant exists yet).

## Anti-Patterns

- **Calling `eprintln!` from evaluator or resolver code** — use `ctx.warnings` or `warnings: &mut Vec<String>`.
- **Calling `evaluate` before `validate`** — the evaluator trusts all references exist.
- **Creating `ModuleCache` per-module instead of per-compile** — destroys caching.
- **Using bare `MdsError::syntax(msg)` when source context is available** — prefer `syntax_at`.
- **Directly interpolating `Value::Object`** — `{obj}` is a runtime error; use `{obj.key}`.
- **Using single-var `@for item in obj:` on an object** — fails at validate time with a hint.
- **Bypassing `build_output` for new output paths** — must call `build_output(&resolved)`.
- **Adding a new `Arg` variant without updating all three match sites** — parser, evaluator, validator all match exhaustively.
- **Passing separate `call_stack`/`warnings` instead of `ctx` to evaluator helpers**.
- **Using `compile` instead of `compile_collecting_warnings` in CLI code**.
- **Calling `get_all_exports()` and expecting a `HashMap`** — it returns `Vec<(String, Arc<FunctionDef>)>`.
- **Injecting `Value::Null` as a placeholder for `@define` params in validation** — use `Value::Array(vec![])` so `@for item in param:` inside a body passes the array type check.
- **Ignoring the `Result` from `scope.pop()`** — always `scope.pop()?`.
- **Accessing `func.captured_namespaces` directly** — use `func.captured.namespaces`.
- **Calling `arity` / `arity_at` with a single `expected` value** — both now require `expected_min` and `expected_max`.
- **Adding a new `Condition` variant without updating `validate_condition` in the validator** — compound conditions require recursive traversal, leaf conditions use `condition.root()`.
- **Matching exhaustively on `MdsError` or `Value` in external code** — both are `#[non_exhaustive]`.
- **Forgetting to capture closure scope in new definition-like directives** — `FunctionDef::from` always produces `captured: CapturedScope::default()`.
- **Placing a required param after a param with a default** — the parser rejects this, but any programmatic `DefineBlock` construction must also respect this invariant.

## Gotchas

- **`And`/`Or` conditions are validated conservatively** — the validator checks all operands even though evaluation short-circuits. A variable undefined in any operand fails validation even if that operand would never be reached at runtime.
- **`call_function` returns `Value`, not `String`** — code that previously expected `call_function` to return `Result<String>` must be updated. `invoke_function` (user-defined functions) still returns `String` internally.
- **`Arg::Call` in `resolve_args` returns the raw `Value`** — in v0.2.0 the `Arg::Call` arm calls `call_function` and returns its `Value` directly, preserving the type for built-in consumers. This differs from pre-v0.2.0 where the result was always `Value::String`.
- **`MAX_LOGICAL_OPERANDS = 16` is a leaf count, not an operand-per-level count** — `count_leaf_operands` sums recursively. `a && b || c && d` has 4 leaf operands.
- **`slice` clamping behavior** — negative start indices clamp to 0. End indices beyond the collection length clamp to the collection length. No panic, no error.
- **`sort` requires homogeneous arrays** — mixing strings and numbers is a `BuiltinError` at runtime.
- **`first`/`last` on empty array returns `Value::Null`** — not an error.
- **`join` requires an array of strings** — non-string elements produce `BuiltinError`.
- **User-defined functions shadow built-ins** — if a template defines `@define upper(x):`, it replaces the built-in `upper`. This is intentional.
- **`Param.default` uses `CondValue`, not `Value`** — defaults are parsed at definition time, stored as `CondValue`, and converted via `condvalue_to_value` at call time.
- **`required_param_count` is defined in `evaluator.rs` and imported by `validator.rs`** — not in `scope.rs` where `FunctionDef` lives.
- **`@elseif` short-circuits** — once truthy, remaining branches are not evaluated but all are validated.
- **Equality conditions require exact type match** — no coercion. `"0" != 0`.
- **Key-value iteration sorts keys alphabetically** — YAML insertion order is not preserved.
- **`raw_frontmatter` is captured for all resolved modules** — only the entry module's frontmatter is prepended to output.
- **`call_stack` is `Vec`, not `HashSet`** — recursion detection uses O(n) scan at MAX_CALL_DEPTH=128.
- **`TextNode` has no offset** — raw text nodes do not carry byte offsets for error reporting.

## Key Files

- `crates/mds-core/src/limits.rs` — all cross-pipeline resource limits; `MAX_LOGICAL_OPERANDS = 16` added in v0.2.0
- `crates/mds-core/src/builtins.rs` — 18 built-in functions; `BuiltinMeta` struct; `get_builtin` / `call_builtin` entry points; **new file in v0.2.0**
- `crates/mds-core/src/ast.rs` — all AST types; `Condition::And`/`Or` variants; `Arg::NumberLiteral`, `BooleanLiteral`, `NullLiteral`; `Param` struct replacing `Vec<String>` for params
- `crates/mds-core/src/lib.rs` — public API; declares `pub(crate) mod builtins`; `strip_type_mds` and `prepend_frontmatter` for frontmatter preservation
- `crates/mds-core/src/parser_helpers.rs` — condition precedence parser (`parse_condition`, `parse_and_level`, `count_leaf_operands`); default param parsing; all free parser functions
- `crates/mds-core/src/evaluator.rs` — `call_function` returns `Value`; built-in dispatch; `condvalue_to_value`; `required_param_count`; `And`/`Or` short-circuit in `evaluate_condition`
- `crates/mds-core/src/validator.rs` — builtin-aware `validate_expr`; range arity checks; recursive `validate_condition` for `And`/`Or`; imports `required_param_count` from evaluator
- `crates/mds-core/src/error.rs` — `ArityMismatch` with `expected_min`/`expected_max`; new `BuiltinError` variant; `format_arity` helper
- `crates/mds-core/src/scope.rs` — `FunctionDef.params: Vec<Param>`; `CapturedScope` struct; `Arc<FunctionDef>` in frames
- `crates/mds-core/src/resolver.rs` — orchestrator; `ModuleCache`; import semantics; security enforcement; `raw_frontmatter`
- `crates/mds-cli/src/main.rs` — CLI: `run_build`/`run_check`/`run_init`; `exit_code`; `resolve_output_path`; `load_config`
- `crates/mds-core/tests/api_surface.rs` — public API regression tests; update when adding public symbols

## Related

- ADR-008: bundles related language features in single PR (applies to v0.2.0 — built-ins, default args, and logical operators shipped together)
- `crates/mds-core/src/resolver.rs` — canonical reference for module system, import semantics, security guards, `Arc<ResolvedModule>` cache
- `crates/mds-core/src/evaluator.rs` — canonical reference for `EvalContext`, `resolve_dot_path`, directive execution, closure restore, call-depth guards
- `crates/mds-core/src/scope.rs` — canonical reference for `CapturedScope`, `Arc<FunctionDef>`, closure capture API
- `crates/mds-core/src/ast.rs` — canonical reference for all AST types; start here for new argument or expression forms
- `crates/mds-core/src/lib.rs` — canonical reference for two-tier warning API, `resolve_base_dir`, frontmatter preservation helpers
- `crates/mds-cli/tests/` — end-to-end tests across 10 categorized files (`language.rs`, `objects.rs`, `imports.rs`, `errors.rs`, `cli_build.rs`, `cli_commands.rs`, `security.rs`, `frontmatter.rs`, `warnings.rs`) plus `common/mod.rs`
- `crates/mds-core/tests/api_surface.rs` — update here when adding public functions, `Value` variants, or `MdsError` variants

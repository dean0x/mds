---
type: design-artifact
version: 1
status: APPROVED
issue: 74
title: "Expression support in @for and @if directives"
slug: expression-directives
created: 2026-06-04T13:07:00+0300
updated: 2026-06-04T13:45:00+0300
execution-strategy: SINGLE_CODER
context-risk: MEDIUM
---

## Problem Statement

Directives (`@for`, `@if`, `@elseif`) only accept bare variable names and dot-paths, even though interpolation (`{...}`) already supports the full expression grammar — function calls, chaining, qualified calls. This forces users into awkward workarounds (pre-computing in frontmatter, helper functions) and creates a jarring semantic gap in the language.

Target users: LLM prompt engineers and AI application developers composing multi-turn prompt templates.

## Acceptance Criteria

### Functionality (F)
- [ ] F1: `@for x in func(args):` compiles and iterates the resulting array
- [ ] F2: `@for x in func(func(args)):` — nested/chained calls work
- [ ] F3: `@if func(args):` evaluates and branches on truthiness
- [ ] F4: `@if !func(args):` negates the truthiness of the expression result
- [ ] F5: `@if func(x) == "value":` — expression on LHS of comparison
- [ ] F6: `@if func(a) == func(b):` — expression on BOTH sides of comparison
- [ ] F7: `@if func(a) && func(b):` — logical operators compose with function calls
- [ ] F8: `@elseif func(args):`, `@elseif func(x) == "val":` — same as @if
- [ ] F9: `@if ns.func(x):` and `@for x in ns.func(x):` — QualifiedCall works
- [ ] F10: User-defined `@define` functions work in conditions and iterables
- [ ] F11: `@for key, value in func():` works when expression returns object
- [ ] F12: `@for item in func():` returning empty array → zero iterations, no output

### Backward Compatibility (BC)
- [ ] BC1: `@for x in variable:` behavior unchanged
- [ ] BC2: `@if variable:` and `@if !variable:` behavior unchanged
- [ ] BC3: `@if var == "literal":` and `@if var != "literal":` behavior unchanged
- [ ] BC4: `@for x in config.items:` (dot-path) behavior unchanged
- [ ] BC5: `@if config.debug:` (dot-path) behavior unchanged
- [ ] BC6: All 590+ existing tests pass without modification (test assertions may update for new AST shapes)

### Error Handling (E)
- [ ] E1: Non-array result in `@for` produces clear diagnostic including expression text
- [ ] E2: Null result in `@for` produces clear diagnostic
- [ ] E3: Undefined function in directive → "undefined function 'name'" error
- [ ] E4: Arity mismatch in directive → "func requires N args, got M" error
- [ ] E5: Undefined variable in expression arg → "undefined variable 'name'" error
- [ ] E6: `@if true:` / `@if "literal":` → reject with "use a variable or function call, not a bare literal"
- [ ] E7: `@if !func(x) == "val":` → reject with "cannot combine negation with comparison"
- [ ] E8: `@for key, value in func():` where func returns array → clear "requires object" error

### Security (S)
- [ ] S1: `split()` rejects results exceeding MAX_ARRAY_ELEMENTS (100,000)
- [ ] S2: `join()` rejects output exceeding MAX_OUTPUT_SIZE (50 MB)
- [ ] S3: Colon inside string literal args does not corrupt directive parsing
- [ ] S4: Nested function calls respect MAX_CALL_DEPTH (128) and MAX_NESTING_DEPTH (64)

### Distribution (D)
- [ ] D1: Expression directive templates compile via CLI (`mds compile`)
- [ ] D2: Expression directive templates compile via WASM binding
- [ ] D3: Expression directive templates compile via napi binding
- [ ] D4: Node-API integration tests pass on all 3 OS (ubuntu, macos, windows)

### Performance (P)
- [ ] P1: Simple `@for x in var:` has no measurable regression (same code path minus one .to_string())
- [ ] P2: Simple `@if var:` has no measurable regression
- [ ] P3: Expression evaluation in `@for` happens once before iteration, not per-iteration
- [ ] P4: `@if` short-circuit evaluation preserved for `&&`/`||` with expressions

## Scope

### v1 Included
- All 4 Expr variants (Var, Call, QualifiedCall, MemberAccess) in @for iterables and @if/@elseif conditions
- Literal variants added to Expr (StringLiteral, NumberLiteral, BooleanLiteral, NullLiteral) for comparison operands
- Expression on BOTH sides of `==`/`!=` comparisons (Eq/NotEq become Eq(Expr, Expr))
- Negated expressions (`!func(x)`)
- Logical composition (`&&`, `||`) with expression operands
- Quote+paren-aware directive colon detection
- Security hardening: split()/join() resource limits
- CondValue removed from Condition enum (replaced by Expr literal variants)

### Deferred
- `@for item in func(x).field:` — post-call member access chaining
- Parenthesized sub-expressions: `@if (a || b) && c:` — explicit grouping
- Pipeline/chaining syntax: `func(x) | other()`
- Arithmetic operators (`>`, `<`, `>=`, `<=`)
- Bare literals in truthy conditions (`@if true:`, `@if "string":`)

### Excluded
- New comparison operators beyond `==`/`!=`
- Full scripting runtime or expression language

## Gap Analysis Results

### Blocking (resolved in plan)
1. **ForBlock.iterable: Vec<String> → Expr** — AST can't represent expressions
2. **Condition enum lacks Expr variants** — can't represent @if func(args):
3. **evaluate_expr returns String, not Value** — @for can't consume expression results
4. **strip_suffix(':') is position-blind** — corrupts colons inside string literals
5. **Eq/NotEq hold CondValue on RHS** — can't support expression-on-both-sides comparisons

### Should-Address (resolved in plan)
- split() lacks element count limit → add MAX_ARRAY_ELEMENTS
- join() lacks output size limit → add MAX_OUTPUT_SIZE check
- evaluate_condition takes &Scope but evaluate_expr needs &mut Scope → signature change
- Condition::path()/root() meaningless with Expr → remove, use direct pattern matching
- CondValue removed from Condition enum → replaced by Expr literal variants

### Informational
- Validator static type checking limited to Expr::Var (accepted limitation for Call/QualifiedCall/MemberAccess)
- Common-case performance: extra enum match for simple @for/@if — negligible
- Intermediate array allocations for chained builtins — bounded by MAX_LOOP_ITERATIONS, evaluated once
- No public API affected — Condition/ForBlock/Expr are pub(crate) only

## Execution Strategy

**SINGLE_CODER** — ~300 LOC production + ~120 LOC tests across 6 files in mds-core.

Rationale: AST change breaks all consumers simultaneously (Rust's exhaustive match). No valid intermediate compilation state. Single crate with tightly coupled sequential dependency chain. 590+ existing tests provide safety net.

## Implementation Plan

Steps 1-4 are atomic — the crate will not compile between them. Step 5 is independent.

### Step 1: AST Type Changes
**File:** `crates/mds-core/src/ast.rs`

Add literal variants to Expr:
```rust
pub enum Expr {
    Var(String),
    Call { name: String, args: Vec<Arg> },
    QualifiedCall { namespace: String, name: String, args: Vec<Arg> },
    MemberAccess { object: String, fields: Vec<String> },
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    NullLiteral,
}
```

Change Condition leaf variants:
- `Truthy(Vec<String>)` → `Truthy(Expr)`
- `Not(Vec<String>)` → `Not(Expr)`
- `Eq(Vec<String>, CondValue)` → `Eq(Expr, Expr)` — expression on both sides
- `NotEq(Vec<String>, CondValue)` → `NotEq(Expr, Expr)` — expression on both sides
- `And`/`Or` unchanged

Change `ForBlock.iterable: Vec<String>` → `ForBlock.iterable: Expr`

Remove `Condition::path()` and `Condition::root()` helper methods.

CondValue enum can be kept for now (used in default param parsing) but is no longer referenced in Condition variants.

### Step 2: Parser Changes
**Files:** `crates/mds-core/src/parser.rs`, `crates/mds-core/src/parser_helpers.rs`

**2a.** Add `strip_trailing_directive_colon(s: &str) -> Option<&str>` — quote+paren-aware forward scan. Returns `Option<&str>` so callers produce directive-specific errors. Replace 4 `strip_suffix(':')` sites.

**2b.** Add `paren_depth` tracking to `find_unquoted_operator` and `split_on_unquoted_op`. Only report operators when `paren_depth == 0`.

**2c.** Extract `parse_expr_inner(content: &str) -> Result<Expr, MdsError>` from `parse_interpolation_expr`. Extend with literal detection:
- Quoted strings → `Expr::StringLiteral`
- Numeric values → `Expr::NumberLiteral`
- `true`/`false` → `Expr::BooleanLiteral`
- `null` → `Expr::NullLiteral`
- Existing: var, call, qualified call, member access

**2d.** Update `parse_simple_condition`:
- Truthy/Not: call `parse_expr_inner`, reject literal Expr variants with error "use a variable or function call, not a bare literal"
- Eq/NotEq: parse BOTH sides with `parse_expr_inner` → `Condition::Eq(Expr, Expr)`. Both literal and expression operands are valid on either side.
- Update `parse_negation_condition`: replace `parse_dot_path` with `parse_expr_inner`, keep double-negation and negation+comparison rejection.

**2e.** Update `parse_for_block`: replace inline dot-split with `parse_expr_inner`. Reject literal Expr variants with error "cannot iterate over a literal value".

### Step 3: Evaluator Changes
**File:** `crates/mds-core/src/evaluator.rs`

**3a.** Rename `evaluate_expr` to return `Result<Value, MdsError>` (remove `.to_string()` calls). Add `render_expr` as thin wrapper: calls `evaluate_expr`, checks for object interpolation, returns `.to_string()`. Update caller at line 68 to use `render_expr`. Add match arms for new literal Expr variants:
- `Expr::StringLiteral(s)` → `Ok(Value::String(s.clone()))`
- `Expr::NumberLiteral(n)` → `Ok(Value::Number(*n))`
- `Expr::BooleanLiteral(b)` → `Ok(Value::Boolean(*b))`
- `Expr::NullLiteral` → `Ok(Value::Null)`

**3b.** Change `evaluate_condition` signature to `(&Condition, &mut Scope, &mut EvalContext)`. Keep thin `evaluate_condition_value` helper. Rewrite match arms:
- `Truthy(expr)` → `evaluate_condition_value(expr, scope, ctx)?.is_truthy()`
- `Not(expr)` → `!evaluate_condition_value(expr, scope, ctx)?.is_truthy()`
- `Eq(lhs, rhs)` → evaluate both sides, compare with `values_equal_runtime`
- `NotEq(lhs, rhs)` → evaluate both sides, negate comparison
- `And`/`Or` → recursive with scope/ctx passthrough

**3c.** Replace `values_equal(value: &Value, expected: &CondValue)` with `values_equal_runtime(lhs: &Value, rhs: &Value)` that compares two runtime Values directly. Comparison semantics: same type → equal if same value; different types → not equal (no coercion). NaN != NaN (IEEE 754).

**3d.** Update evaluate_condition call sites (evaluate_if lines 464, 478) to pass `scope, ctx`.

**3e.** Rewrite `evaluate_for`: replace `resolve_dot_path` with `evaluate_expr(&block.iterable, scope, ctx)`.

### Step 4: Validator Changes
**File:** `crates/mds-core/src/validator.rs`

**4a.** Rewrite `validate_condition`: for Truthy/Not, extract Expr → `validate_expr`. For Eq/NotEq, validate both Expr operands (literal variants need no validation — they are always valid). For And/Or, recurse.

**4b.** Update `validate_for_node`:
- Call `validate_expr(&block.iterable, ...)` for all expression types
- Static type check only for `Expr::Var(name)` (existing behavior)
- Skip for MemberAccess/Call/QualifiedCall/literals (accepted limitation: runtime type check)
- Validation matrix:
  - `Expr::Var` → static type check (look up variable, verify array)
  - `Expr::MemberAccess` → validate root exists, skip type check (existing limitation)
  - `Expr::Call` / `QualifiedCall` → validate function exists + arity, skip type check
  - Literal variants → reject at parse time ("cannot iterate over a literal")

### Step 5: Security Hardening (Independent — can implement first)
**Files:** `crates/mds-core/src/limits.rs`, `crates/mds-core/src/builtins.rs`

- Add `MAX_ARRAY_ELEMENTS: usize = 100_000` to limits.rs
- `builtin_split`: add `parts.len() > MAX_ARRAY_ELEMENTS` check after collect()
- `builtin_join`: add `out.len() > MAX_OUTPUT_SIZE` check after loop
- Follow existing pattern from `builtin_replace`

### Step 6: Update Existing Tests
- Update parser_tests.rs: `Condition::Truthy(vec![...])` → `Condition::Truthy(Expr::Var(...))`
- Update evaluator tests: same Condition/ForBlock construction updates
- Update Eq/NotEq test assertions: `Condition::Eq(path, CondValue::String(...))` → `Condition::Eq(Expr::Var(...), Expr::StringLiteral(...))`
- Run `cargo test --workspace` — all 590+ tests pass

### Step 7: New Feature Tests

## Test Matrix

### Parser Tests (parser_tests.rs)

| ID | Input | Expected AST | Criteria |
|----|-------|-------------|----------|
| PT1 | `@if func(x):` | `Truthy(Expr::Call{name:"func",...})` | F3 |
| PT2 | `@if !func(x):` | `Not(Expr::Call{name:"func",...})` | F4 |
| PT3 | `@if func(x) == "val":` | `Eq(Expr::Call{...}, Expr::StringLiteral("val"))` | F5 |
| PT4 | `@if func(a) == func(b):` | `Eq(Expr::Call{...}, Expr::Call{...})` | F6 |
| PT5 | `@if func(a) && func(b):` | `And([Truthy(Call), Truthy(Call)])` | F7 |
| PT6 | `@if ns.func(x):` | `Truthy(Expr::QualifiedCall{...})` | F9 |
| PT7 | `@for x in func(args):` | `ForBlock{iterable: Expr::Call{...}}` | F1 |
| PT8 | `@for x in sort(unique(tags)):` | `ForBlock{iterable: Expr::Call{nested}}` | F2 |
| PT9 | `@for x in ns.func(x):` | `ForBlock{iterable: Expr::QualifiedCall{...}}` | F9 |
| PT10 | `@if contains(s, "a:b"):` | Parses correctly (colon in string) | S3 |
| PT11 | `@for x in split(s, ":"):` | Parses correctly (colon as separator) | S3 |
| PT12 | `@if true:` | Parse error: "use a variable or function call" | E6 |
| PT13 | `@if "literal":` | Parse error: "use a variable or function call" | E6 |
| PT14 | `@for x in "literal":` | Parse error: "cannot iterate over a literal" | E6 |
| PT15 | `@if !func(x) == "v":` | Parse error: "cannot combine negation" | E7 |
| PT16 | `@if func():` | `Truthy(Expr::Call{args:[]})` | F3 |
| PT17 | `@elseif func(x) == "v":` | Condition with Eq(Call, StringLiteral) | F8 |

### Backward Compatibility Tests (parser_tests.rs)

| ID | Input | Expected | Criteria |
|----|-------|----------|----------|
| BC-PT1 | `@if active:` | `Truthy(Expr::Var("active"))` | BC2 |
| BC-PT2 | `@if config.debug:` | `Truthy(Expr::MemberAccess{...})` | BC5 |
| BC-PT3 | `@if role == "admin":` | `Eq(Expr::Var("role"), Expr::StringLiteral("admin"))` | BC3 |
| BC-PT4 | `@for x in items:` | `ForBlock{iterable: Expr::Var("items")}` | BC1 |
| BC-PT5 | `@for x in data.list:` | `ForBlock{iterable: Expr::MemberAccess{...}}` | BC4 |

### Evaluator Tests (evaluator.rs + integration)

| ID | Template | Data | Expected Output | Criteria |
|----|----------|------|----------------|----------|
| ET1 | `@if contains(tags,"rust"):\nyes\n@end` | `tags:[rust,go]` | "yes" | F3 |
| ET2 | `@if !starts_with(n,"z"):\nyes\n@end` | `n:"abc"` | "yes" | F4 |
| ET3 | `@if lower(n) == "alice":\nyes\n@end` | `n:"Alice"` | "yes" | F5 |
| ET4 | `@if lower(a) == lower(b):\nyes\n@end` | `a:"Hi",b:"hi"` | "yes" | F6 |
| ET5 | `@if lower(a) == lower(b):\nyes\n@end` | `a:"Hi",b:"Bye"` | "" | F6 |
| ET6 | `@if contains(t,"r") && contains(t,"g"):\nyes\n@end` | `t:[r,g]` | "yes" | F7 |
| ET7 | `@for x in split(csv,","):\n- {x}\n@end` | `csv:"a,b,c"` | "- a\n- b\n- c\n" | F1 |
| ET8 | `@for t in sort(unique(tags)):\n- {t}\n@end` | `tags:[b,a,b]` | "- a\n- b\n" | F2 |
| ET9 | `@for x in upper("hi"):\n@end` | — | Type error (string, not array) | E1 |
| ET10 | `@for x in func():` where func returns null | — | Type error (null) | E2 |
| ET11 | `@for x in func():` returning `[]` | — | "" (empty, no error) | F12 |
| ET12 | `@if undefined_func(x):` | — | Undefined function error | E3 |
| ET13 | `@for x in split("a,b"):` | — | Arity error (missing arg) | E4 |
| ET14 | `@if contains(undef, "x"):` | — | Undefined variable error | E5 |
| ET15 | `@elseif contains(t,"go"):\nsecond` | `t:[go]` | "second" | F8 |
| ET16 | `@define check(x): ... @if check(role):` | — | User function works | F10 |
| ET17 | `@for k,v in func():` returning object | — | Key-value iteration works | F11 |
| ET18 | `@for k,v in func():` returning array | — | "requires object" error | E8 |

### Security Tests

| ID | Scenario | Expected | Criteria |
|----|----------|----------|----------|
| ST1 | split() producing >100K elements | Resource limit error | S1 |
| ST2 | join() producing >50MB output | Resource limit error | S2 |
| ST3 | `@if func("a:b"):` — colon in args | Parses correctly | S3 |
| ST4 | Deeply nested: `func(func(func(...)))` 64 deep | Depth limit error | S4 |
| ST5 | `@for x in split(big, "a"):` in loop | Bounded by MAX_LOOP_ITERATIONS | S4 |

### Distribution Tests (Node-API / edge-case .mds files)

| ID | Path | Verifies | Criteria |
|----|------|----------|----------|
| DT1 | `examples/edge-cases/23_expression_directives.mds` | All expression forms via CLI | D1 |
| DT2 | `examples/edge-cases/24_colon_in_string_args.mds` | Colon ambiguity via CLI | D1 |
| DT3 | node-api-test.mjs: expression @if | Native + WASM bindings | D2, D3 |
| DT4 | node-api-test.mjs: expression @for | Native + WASM bindings | D2, D3 |
| DT5 | node-api-test.mjs: expression comparison | Both sides expression | D2, D3 |
| DT6 | node-api-test.mjs: negation + logical ops | Composition works | D2, D3 |
| DT7 | node-api-test.mjs: error cases | Proper error propagation | D2, D3 |

### Performance Verification

| ID | Check | Method | Threshold | Criteria |
|----|-------|--------|-----------|----------|
| PV1 | Simple @for x in var: | Compare evaluate_for before/after | No measurable difference | P1 |
| PV2 | Simple @if var: | Compare evaluate_condition before/after | No measurable difference | P2 |
| PV3 | @for with expression iterable | Verify single evaluation before loop | Check evaluate_expr called once | P3 |
| PV4 | @if a && b short-circuit | Verify b not evaluated when a is false | Check call count | P4 |

## Patterns to Follow

| Pattern | File:Line | Reuse |
|---------|-----------|-------|
| `validate_expr` | validator.rs:224-287 | Directly reuse for Expr-based conditions and iterables |
| `find_unquoted_operator` | parser_helpers.rs:126-173 | Adapt quote-tracking pattern for colon scanner |
| `parse_interpolation_expr` | parser_helpers.rs:620-675 | Extract core into `parse_expr_inner` |
| `builtin_replace` MAX_OUTPUT_SIZE | builtins.rs:240-249 | Follow pattern for split/join limits |
| `evaluate_for_array`/`evaluate_for_key_value` | evaluator.rs:560-597 | Already Value-based, no change needed |
| `parse_cond_value` | parser_helpers.rs:54-103 | Reuse literal detection for Expr literal variants |

## Integration Points

| Entry Point | File | Connection |
|-------------|------|------------|
| ForBlock.iterable | parser.rs → evaluator.rs → validator.rs | Expr parsed in parser, evaluated in evaluator, validated in validator |
| Condition enum | parser_helpers.rs → evaluator.rs → validator.rs | Same flow as ForBlock |
| evaluate_expr | evaluator.rs:141 | Core evaluation; returns Value; called by conditions, iterables, and render_expr |
| validate_expr | validator.rs:224 | Core validation; handles all Expr variants; called by conditions and iterables |
| values_equal_runtime | evaluator.rs (new) | Compares two Values; replaces values_equal(Value, CondValue) |

## Design Review Results

| Finding | Severity | Mitigation |
|---------|----------|------------|
| evaluate_condition abstraction boundary | HIGH | Keep thin delegation helper (evaluate_condition_value) |
| strip_trailing_directive_colon error specificity | HIGH | Returns Option<&str>; callers produce directive-specific errors |
| evaluate_expr naming | MEDIUM | evaluate_expr (Value) + render_expr (String wrapper) |
| paren_depth scope expansion | MEDIUM | Document and test paren interaction in scanner functions |
| Validator accepted limitation | MEDIUM | Matrix: Var=static, MemberAccess/Call/QualifiedCall=runtime |
| values_equal_runtime comparison semantics | MEDIUM | Same type → value equality; different types → not equal; NaN != NaN |

## Risk Assessment

**Context Risk:** MEDIUM

| Risk | Severity | Mitigation |
|------|----------|------------|
| Atomic compilation (Steps 1-4) | HIGH | Complete all together; use Rust compiler as checklist |
| Expr literal variants add complexity | MEDIUM | Reuse parse_cond_value logic; reject literals in truthy/for positions |
| Colon ambiguity regression | MEDIUM | Quote+paren-aware scanner with explicit edge-case tests |
| evaluate_condition signature ripple | MEDIUM | Only 5 call sites total |
| values_equal_runtime cross-type comparison | LOW | Strict: different types are never equal |
| Backward compatibility | LOW | Exhaustive match enforces all consumer updates |
| Performance regression | LOW | Extra enum match; single .to_string() removed from hot path |

## PR Description Guidance

### Problem Being Solved
Directives (@for, @if, @elseif) only accept bare variables, even though interpolation already supports function calls and chaining. This creates a semantic gap that forces workarounds.

### Key Changes to Highlight
- Condition enum leaf variants unified to hold Expr (function calls, qualified calls, member access, literals)
- Eq/NotEq support expressions on both sides: `@if func(a) == func(b):`
- ForBlock.iterable accepts expressions, not just dot-paths
- evaluate_expr returns Value; new render_expr wrapper for interpolation
- Quote+paren-aware directive colon detection prevents misparsing
- split()/join() security hardening with resource limits

### Breaking Changes
None expected — existing templates produce the same output. Internal AST types change but are not part of the public API (confirmed: all types are pub(crate)).

### Reviewer Focus Areas
- strip_trailing_directive_colon correctness — colon ambiguity fix
- values_equal_runtime semantics — cross-type comparison behavior
- evaluate_condition mutability — scope side effects in And/Or chains
- Expr literal variant rejection in Truthy/Not/ForBlock positions
- Backward compatibility of all test assertion updates

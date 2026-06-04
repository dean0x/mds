# Tech Debt Refactoring Plan — MDS Compiler

## Context

Previous code reviews deferred 11 architectural issues (GitHub #2-#6) as tech debt: parameter threading, ownership model cloning, code decomposition, and CLI UX. The user wants to address all of them now. One item (Arc<String> for error source, RUST-3) is already done — `error.rs` line 12 uses `Arc<NamedSource<String>>`. **10 items remain.**

## Critical Design Decisions

1. **Scope stays OUTSIDE EvalContext** — `evaluate_include` takes `&Scope` (immutable) vs EvalContext holding `&mut warnings`. Bundling creates split-borrow conflicts.
2. **Vec for call_stack** (not IndexSet) — insert/remove in `invoke_function` is LIFO push/pop. O(n) contains is fine at MAX_CALL_DEPTH=128.
3. **Arc<FunctionDef> at storage layer only** — `CapturedScope.functions` stays owned to avoid reference cycles (A captures B captures A). Deref+clone at capture time.
4. **Lexer keeps Vec<Token> return** — iterator pattern would cascade to parser.

---

## Phase 1: EvalContext + Vec call_stack + CLI exit codes

**Files:** `src/evaluator.rs`, `src/main.rs`
**Issues:** #4 (EvalContext), #5 Items 1+3 (HashSet→Vec, exit codes)

### 1A. Define EvalContext struct (`evaluator.rs`, after constants)

```rust
pub(crate) struct EvalContext<'a> {
    call_stack: Vec<String>,
    total_iterations: usize,
    warnings: &'a mut Vec<String>,
}
```

### 1B. Refactor function signatures (leaf-first order)

| Order | Function | Line | Signature Change |
|-------|----------|------|-----------------|
| 1 | `evaluate_include` | 366 | `(inc, scope: &Scope, warnings)` → `(inc, scope: &Scope, ctx: &mut EvalContext)` |
| 2 | `evaluate_for` | 322 | `(block, scope, call_stack, total_iterations, warnings)` → `(block, scope: &mut Scope, ctx: &mut EvalContext)` |
| 3 | `evaluate_if` | 296 | same pattern |
| 4 | `invoke_function` | 196 | `(func, call_key, args, scope, call_stack, total_iterations, warnings)` → `(func, call_key, args, scope: &mut Scope, ctx: &mut EvalContext)` |
| 5 | `call_function` | 241 | same pattern |
| 6 | `call_qualified_function` | 264 | same pattern |
| 7 | `resolve_args` | 150 | `(args, scope, call_stack, total_iterations, warnings, depth)` → `(args, scope: &mut Scope, ctx: &mut EvalContext, depth: usize)` |
| 8 | `evaluate_expr` | 104 | same pattern |
| 9 | `evaluate_nodes` | 43 | same pattern |
| 10 | `evaluate` | 27 | Creates `EvalContext { call_stack: Vec::new(), total_iterations: 0, warnings }` |

### Vec call_stack operations (in `invoke_function`)

| HashSet op | Vec replacement | Line |
|-----------|----------------|------|
| `call_stack.contains(call_key)` | `ctx.call_stack.iter().any(\|s\| s == call_key)` | 205 |
| `call_stack.len() >= MAX_CALL_DEPTH` | `ctx.call_stack.len() >= MAX_CALL_DEPTH` | 208 |
| `call_stack.insert(call_key.to_string())` | `ctx.call_stack.push(call_key.to_string())` | 234 |
| `call_stack.remove(call_key)` | `ctx.call_stack.pop()` + `debug_assert!` | 236 |

Add `debug_assert!(ctx.call_stack.last().map_or(false, |s| s == call_key))` before pop.

### 1C. CLI exit codes (`main.rs`)

```rust
fn exit_code(err: &miette::Error) -> i32 {
    if let Some(mds_err) = err.downcast_ref::<MdsError>() {
        match mds_err {
            MdsError::Io { .. } | MdsError::FileNotFound { .. } | MdsError::NotMdsFile { .. } => 2,
            MdsError::ResourceLimit { .. } => 3,
            _ => 1,
        }
    } else {
        1
    }
}
```

Exit codes: **0**=success, **1**=compilation/validation error, **2**=I/O error, **3**=resource limit.

Change line 165: `process::exit(1)` → `process::exit(exit_code(&e))`. Add `use mds::MdsError;`.

**Edge case:** Errors created via `miette::miette!()` in main.rs (e.g., "cannot read stdin", "cannot write output") do NOT downcast to MdsError — they correctly fall through to exit code 1. Only `MdsError` values converted via `.map_err(miette::Error::from)` are categorized. This is intentional: CLI-level I/O errors (bad arguments) are distinct from compilation I/O errors (missing import file).

### 1D. Borrow analysis

- `evaluate_nodes(nodes, scope: &mut Scope, ctx: &mut EvalContext)` — two independent mutable references, no conflict
- `evaluate_include(inc, scope: &Scope, ctx: &mut EvalContext)` — called from evaluate_nodes where `scope` is `&mut Scope`. Rust allows implicit reborrow from `&mut` to `&`. During the reborrow, scope is immutably borrowed while `ctx` is mutably borrowed — no conflict since they're separate bindings.
- All other evaluator functions take both as `&mut` — no issue since they're independent values.

---

## Phase 2: Lexer + Resolver decomposition

**Files:** `src/lexer.rs`, `src/resolver.rs`
**Issues:** #3 (Lexer/Resolver decomposition)

### 2A. Lexer struct (`lexer.rs`)

```rust
struct Lexer<'a> {
    source: &'a str,
    file: &'a str,
    chars: Vec<char>,
    byte_offsets: Vec<usize>,
    pos: usize,
    tokens: Vec<Token>,
    code_fence_backticks: usize,
}
```

Public API unchanged: `tokenize(source, file)` becomes `Lexer::new(source, file).run()`.

| Method | Current lines | Responsibility |
|--------|--------------|----------------|
| `byte_pos(&self, pos) -> usize` | 37-42 | Closure → method |
| `is_line_start(&self) -> bool` | inline checks | `self.pos == 0 \|\| self.chars[self.pos - 1] == '\n'` |
| `scan_frontmatter(&mut self)` | 46-84 | `---` detection, frontmatter content |
| `scan_code_fence(&mut self) -> bool` | 91-119 | Code fence open/close |
| `scan_code_content(&mut self)` | 121-140 | Raw content in code blocks |
| `scan_directive(&mut self)` | 143-158 | `@` directives at line start |
| `scan_escape(&mut self) -> bool` | 161-172 | `\{` and `\}` escapes |
| `scan_interpolation(&mut self)` | 175-205 | `{...}` with brace depth |
| `scan_text(&mut self)` | 207-232 | Regular text accumulation |
| `run(mut self) -> Result<Vec<Token>>` | main loop | ~25-line dispatcher |

Free functions `is_line_start_chars`, `scan_fence`, `skip_newline` stay as-is (they don't need Lexer state).

**State invariant:** Each `scan_*` method advances `self.pos` past the consumed content and pushes tokens to `self.tokens`. The main `run()` loop checks `self.pos < self.chars.len()` between dispatches.

**Error points:** Three error paths exist (unclosed frontmatter, unclosed interpolation, unclosed code fence). Each `scan_*` method returns `Result<_, MdsError>` and propagates via `?` in `run()`.

### 2B. Resolver decomposition (`resolver.rs`)

**From `process_module()` (lines 207-346):**

| Extracted function | Lines | Signature | Owner |
|-------------------|-------|-----------|-------|
| `build_scope_from_frontmatter` | 221-237 | `(frontmatter, is_md, runtime_vars) -> Result<Scope>` | free fn |
| `collect_definitions_and_imports` | 240-322 | `(&mut self, body, scope, file_str, source, base_dir, runtime_vars, warnings) -> Result<(HashMap, bool, HashSet)>` | `ModuleCache` method |
| `validate_exports` | 324-331 | `(functions, has_explicit_exports, explicit_exports) -> Result<()>` | free fn |

**From `resolve()` (lines 64-181):**

| Extracted function | Lines | Signature | Owner |
|-------------------|-------|-----------|-------|
| `validate_and_read_file` | 84-162 | `(&mut self, path) -> Result<(String, PathBuf, bool)>` | `ModuleCache` method |

`collect_definitions_and_imports` needs `&mut self` because it calls `self.resolve_import()` which recurses through `self.resolve()`. The borrow checker is fine since these are sequential calls within the method, not overlapping borrows.

After extraction, `process_module()` becomes ~20-line orchestrator: `tokenize → parse → build_scope → collect_definitions → validate_exports → validate → evaluate → construct ResolvedModule`.

---

## Phase 3: Arc<FunctionDef> + CapturedScope + Arc<ResolvedModule>

**Files:** `src/scope.rs`, `src/resolver.rs`, `src/evaluator.rs`, `Cargo.toml`
**Issues:** #2 (Arc<FunctionDef>, CapturedScope, Arc<ResolvedModule>), #5 Item 4 (IndexSet)

### 3A. CapturedScope struct (`scope.rs`)

```rust
#[derive(Debug, Clone, Default)]
pub struct CapturedScope {
    pub namespaces: HashMap<String, NamespaceScope>,
    pub functions: HashMap<String, FunctionDef>,  // Owned — NOT Arc (avoids cycles)
    pub vars: HashMap<String, Value>,
}
```

FunctionDef changes to:
```rust
pub struct FunctionDef {
    pub params: Vec<String>,
    pub body: Vec<Node>,
    pub captured: CapturedScope,
}
```

Update all access sites: `func.captured_namespaces` → `func.captured.namespaces`, etc.

**Sites to update:**
- `evaluator.rs` lines 220-229: three `for` loops iterating captured fields
- `resolver.rs` lines 261-263: three assignment statements filling captures
- `scope.rs` line 21-27: `From<&DefineBlock>` impl uses `CapturedScope::default()`
- `scope.rs` test line 187-193: direct FunctionDef construction uses `captured: CapturedScope::default()`

### 3B. Arc<FunctionDef> at storage layer

| Type | Change |
|------|--------|
| `Frame::functions` | `HashMap<String, FunctionDef>` → `HashMap<String, Arc<FunctionDef>>` |
| `NamespaceScope::functions` | same |
| `ResolvedModule::functions` | same |
| `CapturedScope::functions` | **stays owned** `HashMap<String, FunctionDef>` |
| `Scope::set_function` | takes `Arc<FunctionDef>` |
| `Scope::get_function` | returns `Option<&Arc<FunctionDef>>` |
| `Scope::get_all_functions` | returns `HashMap<String, Arc<FunctionDef>>` (Arc::clone is O(1)) |

**Closure capture conversion (resolver.rs):**
```rust
// get_all_functions() returns HashMap<String, Arc<FunctionDef>>
// CapturedScope.functions needs HashMap<String, FunctionDef>
// Deref + clone at capture time:
func.captured.functions = scope.get_all_functions()
    .into_iter()
    .map(|(k, v)| (k, (*v).clone()))
    .collect();
```

This dereferences the Arc and clones the inner FunctionDef. Captures are infrequent (once per `@define` at compile time). The main performance win is at call sites (frequent), where `scope.get_function(name)?.clone()` becomes `Arc::clone` — O(1).

**Function definition in resolver.rs (line 256-265):**
```rust
let func = FunctionDef::from(def);
// ... fill captured scope ...
let func = Arc::new(func);
functions.insert(def.name.clone(), Arc::clone(&func));
scope.set_function(&def.name, func);
```

**Captured function restoration in evaluator.rs invoke_function:**
```rust
for (name, f) in &func.captured.functions {
    scope.set_function(name, Arc::new(f.clone()));  // Owned → Arc conversion
}
```

**Validator impact (validator.rs line 170-173):**
`scope.get_function(name)` returns `Option<&Arc<FunctionDef>>`. Access to `func.params.len()` auto-derefs through `&Arc<FunctionDef>` → `&FunctionDef`. No code changes needed.

**Evaluator call sites:**
- `call_function` line 249-252: `scope.get_function(name)?.clone()` — clone on `&Arc<FunctionDef>` = `Arc::clone` (O(1)), then pass `&func` to invoke_function (auto-deref)
- `call_qualified_function` line 279-283: `ns.functions.get(name)?.clone()` — same Arc::clone pattern

**collect_all generic (scope.rs line 156-164):**
`collect_all<T: Clone>` works unchanged. With `T = Arc<FunctionDef>`, `v.clone()` produces `Arc::clone` (O(1)).

### 3C. Arc<ResolvedModule> for cache

- `ModuleCache::modules` → `HashMap<PathBuf, Arc<ResolvedModule>>`
- `resolve()` returns `Result<Arc<ResolvedModule>, MdsError>`
- `resolve_source()` returns `Result<Arc<ResolvedModule>, MdsError>`
- Cache hit (line 114-116): `cached.clone()` → `Arc::clone(cached)` — O(1) vs deep clone
- Cache insert (line 178): `Arc::new(resolved)`, then `Arc::clone` for HashMap insert

**Cascade to callers:**
- `resolve_import` (line 348-431): `self.resolve()` returns `Arc<ResolvedModule>`. Field access via `.` auto-derefs: `resolved.get_export(name)`, `resolved.to_namespace()`. No changes needed.
- `get_export()` return type: `Option<FunctionDef>` → `Option<Arc<FunctionDef>>` (returns `self.functions.get(name).cloned()` — Arc::clone)
- `get_all_exports()` return type: `Vec<(String, FunctionDef)>` → `Vec<(String, Arc<FunctionDef>)>` (Arc::clone per entry)
- `to_namespace()` builds `NamespaceScope { functions: ... }` — functions are already `Arc<FunctionDef>`, so clone = Arc::clone (cheap)
- **lib.rs**: `compile_collecting_warnings` accesses `resolved.prompt_body` — auto-deref through Arc. No public API changes.

### 3D. IndexSet for cycle detection

- Add `indexmap = "2"` to `[dependencies]` in Cargo.toml
- `ModuleCache::resolving: HashSet<PathBuf>` → `IndexSet<PathBuf>` (import from `indexmap::IndexSet`)
- Remove `resolving_stack: Vec<PathBuf>` — IndexSet provides ordered iteration natively
- `self.resolving.insert(canonical.clone())` — API matches HashSet
- `self.resolving.shift_remove(&canonical)` — preserves insertion order (use `shift_remove`, not `swap_remove`)
- `build_cycle_string` changes parameter from `&[PathBuf]` to accepting `IndexSet` iteration: use `self.resolving.iter()` and find the cycle start with `.position()`
- Remove `resolving_stack` field from `ModuleCache` struct

### 3E. Test updates for struct changes

- `scope.rs` test `scope_function_lookup` (line 183-197): Change `FunctionDef { ... captured_namespaces: ..., captured_functions: ..., captured_vars: ... }` to `FunctionDef { params: ..., body: ..., captured: CapturedScope::default() }` and wrap in `Arc::new()`
- `evaluator.rs` test (line 524): `FunctionDef::from(&define)` — unchanged (From impl handles CapturedScope)
- `evaluator.rs` test: `scope.set_function("greet", func)` → `scope.set_function("greet", Arc::new(func))`

---

## Acceptance Criteria

### Phase 1 Acceptance Criteria

| ID | Criterion | Verification |
|----|-----------|-------------|
| P1-AC1 | EvalContext struct exists with fields: `call_stack: Vec<String>`, `total_iterations: usize`, `warnings: &mut Vec<String>` | Code review |
| P1-AC2 | All 10 evaluator functions use `(scope: &mut Scope, ctx: &mut EvalContext)` instead of 4-5 separate parameters | Code review — `grep "call_stack\|total_iterations" src/evaluator.rs` returns only EvalContext definition and struct field access |
| P1-AC3 | `evaluate_include` uses `scope: &Scope` (immutable) + `ctx: &mut EvalContext` | Code review |
| P1-AC4 | `HashSet<String>` removed from evaluator.rs — no `use std::collections::HashSet` import | `grep HashSet src/evaluator.rs` returns empty |
| P1-AC5 | `debug_assert!` guards call_stack.pop() in invoke_function | Code review |
| P1-AC6 | `exit_code()` function in main.rs categorizes MdsError variants into exit codes 1/2/3 | Code review |
| P1-AC7 | `process::exit(exit_code(&e))` replaces `process::exit(1)` | Code review |
| P1-AC8 | All existing tests pass | `cargo test` — 0 failures |
| P1-AC9 | No new clippy warnings | `cargo clippy` — 0 warnings |
| P1-AC10 | Recursion detection still works (direct + mutual) | Integration tests `recursion_detected` + `mutual_recursion_detected` pass |
| P1-AC11 | Call depth limiting still works at MAX_CALL_DEPTH=128 | Existing test coverage or manual verification |

### Phase 2 Acceptance Criteria

| ID | Criterion | Verification |
|----|-----------|-------------|
| P2-AC1 | `Lexer` struct exists with fields: source, file, chars, byte_offsets, pos, tokens, code_fence_backticks | Code review |
| P2-AC2 | `tokenize()` public function delegates to `Lexer::new(source, file).run()` | Code review |
| P2-AC3 | At least 7 `scan_*` methods extracted from the main loop | Code review — each `scan_*` is a separate `fn` on `impl Lexer` |
| P2-AC4 | No method on Lexer exceeds 40 lines | `skim src/lexer.rs` — verify method lengths |
| P2-AC5 | `build_scope_from_frontmatter` extracted as free function in resolver.rs | Code review |
| P2-AC6 | `validate_exports` extracted as free function in resolver.rs | Code review |
| P2-AC7 | `validate_and_read_file` extracted as method on ModuleCache | Code review |
| P2-AC8 | `process_module()` is ≤30 lines (orchestrator only) | Code review |
| P2-AC9 | Public API unchanged — `tokenize(source, file) -> Result<Vec<Token>>` signature preserved | Code review |
| P2-AC10 | All existing tests pass | `cargo test` — 0 failures |
| P2-AC11 | No new clippy warnings | `cargo clippy` — 0 warnings |

### Phase 3 Acceptance Criteria

| ID | Criterion | Verification |
|----|-----------|-------------|
| P3-AC1 | `CapturedScope` struct exists with fields: namespaces, functions (owned), vars | Code review |
| P3-AC2 | `FunctionDef` uses `captured: CapturedScope` instead of 3 separate captured_* fields | Code review — `grep captured_namespaces src/scope.rs` returns empty |
| P3-AC3 | `Frame::functions`, `NamespaceScope::functions`, `ResolvedModule::functions` all use `Arc<FunctionDef>` | Code review |
| P3-AC4 | `CapturedScope::functions` uses owned `FunctionDef` (NOT Arc) | Code review — no `Arc<FunctionDef>` in CapturedScope definition |
| P3-AC5 | `Scope::set_function` accepts `Arc<FunctionDef>` | Code review |
| P3-AC6 | `Scope::get_function` returns `Option<&Arc<FunctionDef>>` | Code review |
| P3-AC7 | Closure capture uses deref+clone: `(*arc).clone()` to convert `Arc<FunctionDef>` → owned `FunctionDef` | Code review of resolver.rs capture code |
| P3-AC8 | `ModuleCache::modules` uses `Arc<ResolvedModule>` | Code review |
| P3-AC9 | `resolve()` and `resolve_source()` return `Arc<ResolvedModule>` | Code review |
| P3-AC10 | `resolving` uses `IndexSet<PathBuf>` and `resolving_stack: Vec<PathBuf>` is removed | Code review + `grep resolving_stack src/resolver.rs` returns empty |
| P3-AC11 | `indexmap` added to Cargo.toml `[dependencies]` | Code review |
| P3-AC12 | All existing tests pass | `cargo test` — 0 failures |
| P3-AC13 | No new clippy warnings | `cargo clippy` — 0 warnings |
| P3-AC14 | Cross-module closure tests pass (lexical scope preservation) | Integration test `cross_module_function_preserves_lexical_scope` passes |
| P3-AC15 | Import tests pass (alias, merge, selective, re-export) | Integration tests for all import modes pass |

---

## Test Plan

### Phase 1 Test Plan

**Existing tests (must all pass — behavior preservation):**
- All ~245 existing unit + integration tests validate that EvalContext refactoring doesn't change behavior
- `recursion_detected` — verifies direct recursion (`@define fn: {fn()}`) is caught
- `mutual_recursion_detected` — verifies A→B→A cycle is caught
- All `@for` tests — verify total_iterations tracking still works through EvalContext
- All warning tests (empty `@include`, etc.) — verify warnings still collected through EvalContext

**New tests to add:**

| Test | Location | What it verifies |
|------|----------|-----------------|
| `exit_code_syntax_error` | `tests/integration.rs` | Compile error → exit code 1. Use `Command::new("cargo").args(["run", "--", "build", "nonexistent_directive.mds"])` and assert `status.code() == Some(1)` |
| `exit_code_file_not_found` | `tests/integration.rs` | Missing file → exit code 2. Use `Command::new("cargo").args(["run", "--", "build", "/tmp/no_such_file.mds"])` and assert `status.code() == Some(2)` |
| `exit_code_success` | `tests/integration.rs` | Valid compilation → exit code 0. Assert `status.success()` |

### Phase 2 Test Plan

**Existing tests (must all pass — pure structural refactoring):**
- All tokenizer-exercising tests (frontmatter, code fences, interpolation, directives, escapes)
- All resolver tests (imports, cycle detection, depth limits, symlink rejection, path traversal)
- All integration tests end-to-end

**No new tests needed** — decomposition doesn't change behavior. The comprehensive existing test suite provides full coverage of all scanning branches and resolver paths.

**Manual verification:**
- `skim -n src/lexer.rs` — verify no method exceeds 40 lines
- `skim -n src/resolver.rs` — verify process_module is ≤30 lines

### Phase 3 Test Plan

**Existing tests (must all pass — behavior preservation with new ownership model):**
- `cross_module_function_preserves_lexical_scope` — critical test for closure capture working through Arc
- `multilevel_imports` — tests deep import chains (Arc<ResolvedModule> cache behavior)
- `import_alias`, `import_merge`, `import_selective` — test all import modes with Arc<FunctionDef>
- `reexport`, `wildcard_reexport_barrel` — test re-export through Arc
- `scope_function_lookup` (unit test in scope.rs) — updated to use Arc::new()
- All evaluator unit tests — updated FunctionDef construction

**New tests to add:**

| Test | Location | What it verifies |
|------|----------|-----------------|
| `captured_scope_default` | `src/scope.rs` tests | `CapturedScope::default()` produces empty hashmaps |
| `indexset_cycle_detection` | `tests/integration.rs` | Circular import produces correct cycle path string (verifies IndexSet ordering matches previous Vec ordering) |

---

## Side Effects and Edge Cases

| Side Effect | Phase | Impact | Mitigation |
|-------------|-------|--------|------------|
| `HashSet` import removed from evaluator.rs | 1 | None — only used for call_stack | Verify no other HashSet usage in evaluator |
| `evaluate_include` now receives full `EvalContext` but only uses `warnings` | 1 | Minor unnecessary coupling | Acceptable — API consistency outweighs minimal coupling |
| CLI errors from `miette::miette!()` don't get categorized exit codes | 1 | Falls to exit code 1 | Intentional — CLI errors are user mistakes, not compilation failures |
| `get_all_functions()` return type changes to `HashMap<String, Arc<FunctionDef>>` | 3 | Captured scope must deref+clone | Handled — deref+clone at capture time (infrequent) |
| `get_export()` return type changes to `Option<Arc<FunctionDef>>` | 3 | All callers of get_export must handle Arc | Callers pass to `set_function` which now takes Arc — types align |
| `resolving_stack` removal changes `build_cycle_string` parameter | 3 | Must update cycle string construction to use IndexSet iteration | Replace `&[PathBuf]` parameter with IndexSet-compatible iteration |
| Unit tests constructing FunctionDef directly need updates | 3 | scope.rs + evaluator.rs tests | Use `CapturedScope::default()` and `Arc::new()` |
| `to_namespace()` now produces `NamespaceScope` with `Arc<FunctionDef>` in functions map | 3 | Clone cost changes from deep to cheap | This is the desired improvement |

---

## Files Modified Per Phase

| Phase | Files | Scope |
|-------|-------|-------|
| 1 | `src/evaluator.rs`, `src/main.rs` | Struct + signatures + exit codes |
| 2 | `src/lexer.rs`, `src/resolver.rs` | Decomposition only — no type changes |
| 3 | `src/scope.rs`, `src/resolver.rs`, `src/evaluator.rs`, `src/validator.rs` (no changes needed, auto-deref), `Cargo.toml` | Type changes + IndexSet |

## Verification (after each phase)

1. `cargo build` — compiles cleanly
2. `cargo test` — all tests pass
3. `cargo clippy` — no new warnings
4. `cargo run -- build tests/fixtures/hello.mds` — correct output
5. `cargo run -- check tests/fixtures/hello.mds` — no errors

## Risks

| Risk | Phase | Severity | Mitigation |
|------|-------|----------|------------|
| Borrow checker with `&mut Scope` + `&mut EvalContext` | 1 | Low | Separate values, clean borrows (verified in borrow analysis) |
| Vec pop() ordering invariant | 1 | Low | debug_assert before pop verifies LIFO |
| Lexer scan_* method state invariant (pos advancement) | 2 | Low | Each method must advance pos past consumed content — existing tests catch regressions |
| Arc boundary correctness (owned vs shared) | 3 | Medium | CapturedScope.functions stays owned — deref+clone at capture |
| Arc<ResolvedModule> return type cascade | 3 | Low | Auto-deref handles field access; get_export/get_all_exports return Arc |
| IndexSet removal of resolving_stack | 3 | Low | build_cycle_string adapted to use IndexSet iteration — existing cycle detection tests validate |
| get_all_functions() → captured conversion | 3 | Medium | Explicit `.map(\|(k, v)\| (k, (*v).clone()))` — compile-time only cost |

## GitHub Issues Resolved

- [x] #4 — EvalContext struct (Phase 1)
- [x] #5 Item 1 — HashSet→Vec call_stack (Phase 1)
- [x] #5 Item 2 — Arc<String> error source (**already done**)
- [x] #5 Item 3 — CLI exit codes (Phase 1)
- [x] #5 Item 4 — IndexSet cycle detection (Phase 3)
- [x] #3 — Lexer decomposition (Phase 2)
- [x] #3 — Resolver decomposition (Phase 2)
- [x] #2 — Arc<FunctionDef> (Phase 3)
- [x] #2 — CapturedScope struct (Phase 3)
- [x] #2 — Arc<ResolvedModule> cache (Phase 3)

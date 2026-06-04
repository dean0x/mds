# Resolution Summary

**Branch**: feat/compiler
**Review**: 2026-05-13_2320
**Resolved**: 2026-05-14

## Statistics

| Category | Count |
|----------|-------|
| Fixed | 12 |
| False Positives | 1 |
| Deferred (architectural) | ~15 (performance/complexity refactors) |

## Resolved Issues

### Batch 1: Core Blocking Fixes (commit a3a91ea)

| Issue | File | Severity | Resolution |
|-------|------|----------|------------|
| Evaluator overwrites resolver's closure captures | `evaluator.rs:80-82` | HIGH | `Node::Define` arm now skips — resolver handles with full lexical capture |
| is_leaf_loop bypasses MAX_TOTAL_ITERATIONS | `evaluator.rs:334-349` | HIGH | Removed `is_leaf_loop` optimization; all iterations counted unconditionally |
| Resource limits misclassified as MdsError::Io | `error.rs` + 4 call sites | HIGH | Added `MdsError::ResourceLimit` variant; migrated 4 usages |
| Missing error constructors | `error.rs` + `resolver.rs:99` | MEDIUM | Added `circular_import()`, `recursion_at()`, `export_error_at()`; updated direct variant construction |
| find_project_root unbounded loop | `resolver.rs:16-28` | MEDIUM | Replaced `loop` with `for _ in 0..256` |

### Batch 2: Value/Scope/Validator Fixes (commit e7525de)

| Issue | File | Severity | Resolution |
|-------|------|----------|------------|
| `as_array` returns `&Vec<Value>` not `&[Value]` | `value.rs:102` | MEDIUM | Return type changed to `Option<&[Value]>`; caller updated `.clone()` → `.to_vec()` |
| `is_truthy` missing `#[must_use]` | `value.rs:22` | LOW | Added `#[must_use]` attribute |
| `.expect()` in Scope setters can panic | `scope.rs:87,100,114` | MEDIUM | Replaced with `debug_assert!` + `if let` fallback |
| `validate_var_args` recurses without depth bound | `validator.rs:146` | MEDIUM | Added `depth` parameter with 256 limit |

### Batch 3: Main/Test Fixes (commit 6b60be3)

| Issue | File | Severity | Resolution |
|-------|------|----------|------------|
| `mds init` no path validation | `main.rs:304` | MEDIUM | Rejects `..` path components; absolute paths allowed (CLI user controls filesystem) |
| `file_not_found_error` test has no message assertion | `tests/integration.rs:164` | MEDIUM | Added error message content assertion |
| MAX_STDIN_SIZE independently defined | `main.rs:203` | MEDIUM | Exposed `MAX_FILE_SIZE` via `lib.rs`; main.rs references it |

## False Positives

| Issue | Reviewer | Reason |
|-------|----------|--------|
| `load_vars_file` missing `#[must_use]` | Consistency | Already has `#[must_use = "the loaded variables should be used"]` at line 331 |

## Deferred (Architectural — Not Merge-Blocking)

These issues require structural refactoring beyond the scope of issue resolution:

**Performance** (Arc-based sharing refactor):
- Closure capture clones entire scope per function — needs `Arc<FunctionDef>`
- ResolvedModule clone on cache hit — needs `Arc<ResolvedModule>`
- Lexer pre-collects full-source arrays — needs streaming tokenizer
- Various intermediate Vec allocations

**Complexity** (extraction refactors):
- `process_module` 140 lines / 7 responsibilities — extract pipeline stages
- `tokenize` 155 lines — extract into Lexer struct methods
- `parse_interpolation_expr` deep nesting — extract branches
- Evaluator parameter convoy — bundle into EvalCtx struct
- `ModuleCache::resolve` 100 lines — extract validation helper
- `run` function 113 lines — extract command handlers

**Other should-fix**:
- Token enum unnamed tuple fields (consistency)
- Validator duplicates @for type check (architecture — intentional defense-in-depth)
- Weak disjunctive error test assertions (testing — 5 remaining tests)
- Symlink check only on leaf path (security — mitigated by starts_with guard)

## Files Modified

- `src/error.rs` — Added `ResourceLimit` variant + 3 constructors
- `src/evaluator.rs` — Skipped `@define`, unconditional iteration counting, `ResourceLimit` usage, `.to_vec()`
- `src/resolver.rs` — Bounded `find_project_root`, `circular_import()` constructor, `resource_limit()` for file size
- `src/value.rs` — `as_array` return type, `#[must_use]` on `is_truthy`
- `src/scope.rs` — `debug_assert!` + fallback in 3 setters
- `src/validator.rs` — Depth-bounded `validate_var_args`
- `src/main.rs` — Path validation for init, shared `MAX_STDIN_SIZE`
- `src/lib.rs` — Exposed `MAX_FILE_SIZE` as pub const
- `tests/integration.rs` — Strengthened `file_not_found_error` assertion

## Verification

- **213/213 tests pass**
- **0 clippy warnings**
- **0 build warnings**

# Reliability Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

No CRITICAL or HIGH reliability issues found.

### MEDIUM

No MEDIUM issues found.

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing reliability issues found.

## Suggestions (Lower Confidence)

- **`warnings` Vec grows without bound in `compile_*_with_deps` callers** - `crates/mds-core/src/lib.rs:524` (Confidence: 65%) -- The `warnings` vec in `compile_with_deps` / `compile_str_with_deps` / `compile_virtual_with_deps` is created locally as `vec![]` and passed through the resolver. The evaluator already caps warnings at `MAX_WARNINGS = 1_000` internally, so this is bounded in practice, but that bound is an internal evaluator detail not visible at the API boundary. If the evaluator's cap were ever loosened, these vecs could grow unbounded. Low risk given current architecture.

- **`NativeFs::read` allocates full file into memory before size check** - `crates/mds-core/src/fs.rs:318` (Confidence: 62%) -- `NativeFs::read` calls `std::fs::read(path)` which allocates the entire file content before the `MAX_FILE_SIZE` check on line 320. For a 10 MB limit this is acceptable, but a truly adversarial 50 GB file would OOM before the check fires. This is a conscious TOCTOU-safe trade-off (documented in the comment on line 315-317) and the OS typically refuses reads of that size before Rust allocates, so the practical risk is very low.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

The changes in this PR demonstrate strong reliability engineering. Here is a summary of the reliability controls observed:

### Bounded Iteration (Category 1)
- `compute_line_column` (error.rs:46) iterates over `source[..offset].bytes()` -- bounded by `source.len()` with an early return on line 41-43 when `offset > source.len()`.
- `VirtualFs::normalize` (fs.rs:129-151) iterates over path segments with `MAX_PATH_SEGMENTS = 256` enforced at line 143, preventing unbounded segment accumulation.
- `NativeFs::find_project_root` (fs.rs:238) uses `for _ in 0..MAX_TRAVERSAL_DEPTH` -- bounded loop, returns fallback on exhaustion.
- Pre-existing: `MAX_IMPORT_DEPTH = 64` caps recursive import resolution. `MAX_CALL_DEPTH = 128` caps function call depth. `MAX_LOOP_ITERATIONS = 100_000` and `MAX_TOTAL_ITERATIONS = 1_000_000` bound for-loops. All these limits are enforced with explicit error returns.

### Assertion Density (Category 2)
- `check_lifo_pop` (resolver.rs:280-299) asserts the LIFO invariant on the resolving stack after every `process_module` call, with a double-fault preference policy that preserves the user-facing error. This is a production assertion, not just a test assertion.
- `check_import_depth` (resolver.rs:117-124) is a precondition check before recursion.
- The `serialize()` match on error.rs:537-567 is exhaustive with explicit no-span arms for `NotMdsFile`, `Io`, `ResourceLimit`, `YamlError`, `JsonError` -- any new variant added to `MdsError` will cause a compile error, preventing silent drift.

### Allocation Discipline (Category 3)
- `CompileOutput::dependencies` (lib.rs:75) uses `Vec<String>` which is sized proportionally to the number of imported modules -- bounded by `MAX_IMPORT_DEPTH = 64`.
- `IndexMap` and `IndexSet` in `ModuleCache` provide both O(1) lookup and insertion-order iteration without needing parallel data structures.
- `clean_output` (lib.rs:402) pre-allocates `String::with_capacity(s.len())` -- single allocation, no growth.
- `strip_type_mds` (lib.rs:359) pre-allocates `String::with_capacity(raw.len())`.

### Indirection Limits (Category 4)
- No excessive indirection observed. `Arc<ResolvedModule>` is the deepest pointer nesting -- single level, appropriate for shared module references in the cache.
- `Box<dyn FileSystem>` in `ModuleCache` is a single level of trait-object indirection.

### Metaprogramming Restraint (Category 5)
- No macros, no recursive generics, no reflection. The `#[derive(...)]` usage is standard and bounded.
- The `serde::Serialize` derive on `SerializedSpan`, `SerializedError`, and `CompileOutput` is straightforward -- no custom serialization logic or recursive types.

### Resource Cleanup
- `resolving.pop()` is always called after `process_module` regardless of success/failure (resolver.rs:191), with `check_lifo_pop` validating the invariant. This ensures the resolving set does not leak entries.
- `resolve_source` (resolver.rs:261-273) mirrors the same resolving bookkeeping with its own `check_lifo_pop` call.

# Consistency Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent assert level for LIFO invariant checks** - `src/evaluator.rs:196`, `src/resolver.rs:204`
**Confidence**: 90%
- Problem: The evaluator's call_stack LIFO invariant uses `assert!` (fires in release mode) at `src/evaluator.rs:196`, while the resolver's resolving-set LIFO invariant uses `debug_assert_eq!` (debug-only) at `src/resolver.rs:204`. Both protect structurally identical LIFO invariants with the same risk profile (silent corruption if violated). The evaluator comment explicitly justifies release-mode enforcement ("cost is negligible"), but the resolver provides no justification for the weaker check.
- Fix: Either promote the resolver to `assert_eq!` to match the evaluator's "enforce in release" rationale, or demote the evaluator to `debug_assert!` with justification. The evaluator's reasoning ("cost is negligible at MAX_CALL_DEPTH = 128") applies equally to the resolver's `resolving` set (bounded by MAX_IMPORT_DEPTH = 64), so promoting to `assert_eq!` is the consistent choice:
```rust
// src/resolver.rs:203-204
let popped = self.resolving.pop();
assert_eq!(popped.as_ref(), Some(&canonical), "resolving unmark must be LIFO");
```

### MEDIUM

**Doc comment for `MAX_CONFIG_SIZE` merges into `load_config` doc block** - `src/main.rs:33-34`
**Confidence**: 92%
- Problem: The `///` doc comment on line 33 ("Maximum allowed size for `mds.json`...") is syntactically part of the preceding `load_config` function's doc comment (lines 25-32) because there is no intervening blank line or non-doc item. This means `cargo doc` will render this constant's description as part of `load_config`'s documentation, and `MAX_CONFIG_SIZE` itself will have no doc comment. Every other `MAX_*` constant in the codebase has its own standalone `///` doc comment (e.g., `MAX_CALL_DEPTH` at `src/evaluator.rs:8-9`, `MAX_IMPORT_DEPTH` at `src/resolver.rs:43`).
- Fix: Insert a blank line to separate the doc blocks:
```rust
/// resolve relative `output_dir` values.

/// Maximum allowed size for `mds.json` (1 MB) to prevent runaway memory use.
const MAX_CONFIG_SIZE: u64 = 1024 * 1024;
```

**Inconsistent field-level doc comments between `CollectedDefs` and `ModuleCtx`** - `src/resolver.rs:525-538`
**Confidence**: 80%
- Problem: `ModuleCtx` has a doc comment on `file_str` but not on `source`, `base_dir`, or `runtime_vars`. `CollectedDefs` has no doc comments on any field. The existing codebase pattern for internal structs is mixed (e.g., `ModuleCache` documents all fields, `ResolvedModule` documents none, `Frame` documents `functions` and `namespaces` but not `vars`), so this is not a clear violation, but the newly added `file_str` doc comment in `ModuleCtx` creates an incomplete within-struct pattern.
- Fix: Either document all fields of `ModuleCtx` or remove the `file_str` doc comment to match `CollectedDefs`. Since these are internal-only structs with self-documenting field names, removing the partial doc comment is the lower-friction option:
```rust
struct ModuleCtx<'a> {
    file_str: &'a str,
    source: &'a str,
    base_dir: &'a Path,
    runtime_vars: &'a HashMap<String, Value>,
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`CollectedDefs` visibility vs `ModuleCtx` visibility** - `src/resolver.rs:525,532` (Confidence: 65%) -- Both structs are `pub`-free internal types, which is consistent. However, `CollectedDefs` was upgraded from a type alias to a struct in this PR but neither derives `Debug`. `ModuleCtx` also lacks `Debug`. All other structs in this module (`ResolvedModule`, `ModuleCache`) derive at least `Debug` or `Default`. Consider adding `#[derive(Debug)]` to both for parity with the rest of the module.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The branch maintains strong consistency overall: error handling patterns (double-fault preservation in `evaluate_for` and `invoke_function`), naming conventions, test style, path-traversal guards, and the `CollectedDefs` struct upgrade all align with existing codebase conventions. The version-qualifier addition to error messages (`"in MDS v0.1"`) is applied consistently to both `from_yaml` and `from_json` in `value.rs`. The resolver decomposition (`canonicalize_and_check` / `read_validated_file`) follows the same method-doc-comment style as the rest of the module. The one notable inconsistency -- `assert!` vs `debug_assert_eq!` for equivalent LIFO invariants -- should be resolved before merge.

# Architecture Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Frontmatter preservation logic placed in lib.rs violates layer responsibility** - `src/lib.rs:254,278`
**Confidence**: 82%
- Problem: The `prepend_frontmatter()` and `strip_type_mds()` functions are defined in `lib.rs` (the public API facade) and operate on data produced by the resolver (`raw_frontmatter`). This places output composition logic in the wrong layer. The pipeline is lexer -> parser -> validator -> resolver -> evaluator -> render, but frontmatter assembly happens in the API facade after the resolver returns, creating a new implicit "render" step outside the pipeline.
- Fix: Move `prepend_frontmatter()` and `strip_type_mds()` into the resolver's `process_module()` or into a dedicated `render.rs` module. The `ResolvedModule` could either store the already-cleaned frontmatter, or a new post-processing step could be added to the pipeline. This keeps the public API functions as thin wrappers and avoids duplicating the frontmatter prepend call in `compile_collecting_warnings` and `compile_str_collecting_warnings`.

**`assert!` in evaluator for parser invariants will panic in release builds** - `src/evaluator.rs:321,339`
**Confidence**: 85%
- Problem: The evaluator uses `assert!(!block.condition.is_empty())` and `assert!(!block.iterable.is_empty())` to enforce parser invariants. Unlike `debug_assert!`, these execute in release builds and will panic (abort the process) rather than returning a `Result::Err`. This is inconsistent with the validator (which uses `.first().ok_or_else()` for the same invariant) and violates the project's Result-based error handling principle. A malformed AST from a future parser bug would crash the entire process rather than producing a recoverable error.
- Fix: Replace the `assert!` calls with the same pattern used in the validator:
  ```rust
  let root = block.condition.first().ok_or_else(|| {
      MdsError::syntax("internal error: @if block has empty condition path")
  })?;
  let value = resolve_dot_path(root, &block.condition[1..], scope)?;
  ```

### MEDIUM

**`raw_frontmatter` field on `ResolvedModule` leaks an intermediate representation** - `src/resolver.rs:43`
**Confidence**: 80%
- Problem: `ResolvedModule` now carries `raw_frontmatter: Option<String>` which is the unparsed YAML text captured before scope building. This is a transient rendering concern (needed only to reconstruct the output) that is stored alongside semantic resolution results (functions, exports, prompt_body). It creates coupling between the resolver's output and the output formatting step in lib.rs, making `ResolvedModule` do double duty as both a semantic artifact and a rendering artifact.
- Fix: Consider either: (a) having `process_module` return a richer struct that separates semantic content from rendering metadata, or (b) performing the frontmatter assembly inside `process_module` so `prompt_body` already includes it. Given that imported modules should NOT have their frontmatter prepended (only the root module), option (a) or a flag distinguishing root vs. imported modules is architecturally cleaner.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Duplicated loop body pattern in `evaluate_for`** - `src/evaluator.rs:367-381,412-424`
**Confidence**: 80%
- Problem: The key-value iteration branch and the array iteration branch both contain an identical pattern: increment `ctx.total_iterations`, check `MAX_TOTAL_ITERATIONS`, `scope.push()`, set vars, `evaluate_nodes`, `scope.pop()`, `prefer_first_error`. This is classic code duplication that will diverge over time if one branch is updated but not the other.
- Fix: Extract a helper like `fn run_loop_body(scope, ctx, body, bindings) -> Result<String, MdsError>` that encapsulates the push/eval/pop/error-preference pattern. The two branches would then differ only in how they prepare the loop variable bindings.

## Pre-existing Issues (Not Blocking)

(none at CRITICAL severity)

## Suggestions (Lower Confidence)

- **`HashMap` for `Value::Object` does not preserve insertion order** - `src/value.rs:30` (Confidence: 70%) — YAML mappings have defined key order. Using `HashMap<String, Value>` discards this order. The code compensates by sorting keys during iteration (in `evaluate_for` and `Display`), but consumers who care about document order (e.g., frontmatter round-tripping) cannot recover it. Consider `IndexMap` if ordering becomes important.

- **Validator skips static type check on dot-path iterables** - `src/validator.rs:70-77` (Confidence: 65%) — The comment explains this is an accepted limitation for v0.1, but it means `@for item in obj.field:` where `field` is not an array will only produce an error at evaluation time with less precise span information. This is a design trade-off, not a bug.

- **`IfBlock.condition` and `ForBlock.iterable` as `Vec<String>` encode two concerns** - `src/ast.rs:91,103` (Confidence: 62%) — These fields encode both the root variable lookup and the field traversal path. A dedicated `DotPath { root: String, fields: Vec<String> }` type would make the invariant (non-empty) enforceable at construction time via a constructor, eliminating the need for runtime assertions and `.first().ok_or_else()` guards.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The architecture is fundamentally sound. The sequential pipeline (lexer -> parser -> validator -> resolver -> evaluator) is preserved, and the new Object type integrates cleanly at every layer. Each layer maintains single responsibility for its core concern. The main conditions are:

1. The `assert!` calls in the evaluator should be converted to Result-returning checks to maintain the "never panic in business logic" principle (HIGH).
2. The frontmatter prepend logic should ideally be moved into the pipeline proper rather than living in the API facade (HIGH), though this is a layering preference rather than a correctness issue.

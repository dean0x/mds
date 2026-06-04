# Reliability Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Unbounded `warnings` vector growth** - `src/evaluator.rs`, `src/resolver.rs` (multiple call sites)
**Confidence**: 82%
- Problem: The `warnings: &mut Vec<String>` parameter is threaded through the entire compilation pipeline (resolver, evaluator, lib.rs) with no upper bound on the number of warnings that can be accumulated. A crafted input with thousands of `@include` directives referencing modules with empty bodies could append an unbounded number of warning strings to this vector, consuming arbitrary memory before the output size limit is reached.
- Fix: Add a capacity guard before pushing to warnings:
```rust
const MAX_WARNINGS: usize = 1_000;

// Before each warnings.push():
if warnings.len() < MAX_WARNINGS {
    warnings.push(format!("..."));
}
```

### MEDIUM

**Recursion-based call depth in `resolve_args` without stack-depth tracking** - `src/evaluator.rs:145-177`
**Confidence**: 85%
- Problem: The `resolve_args` function calls itself recursively for nested `Arg::Call` arguments. While `call_function` (which `resolve_args` invokes) checks `call_stack.len() >= MAX_CALL_DEPTH`, the `resolve_args` recursion itself is bounded only by the `call_stack` HashSet check, which uses function *names* rather than call depth. If the same function appears in deeply nested argument positions (e.g., `f(f(f(f(...))))`), the `call_stack` HashSet will reject the second occurrence as recursion, so practical exploitation is limited to chains of *distinct* function names. The parser's `parse_args_inner` does enforce `MAX_NESTING_DEPTH=256` on the AST side, providing an implicit bound. However, the evaluator's `resolve_args` does not independently verify depth, relying on the parser having been called first.
- Fix: Add an explicit depth parameter to `resolve_args` as a defense-in-depth measure:
```rust
fn resolve_args(
    args: &[Arg],
    scope: &mut Scope,
    call_stack: &mut HashSet<String>,
    total_iterations: &mut usize,
    warnings: &mut Vec<String>,
    depth: usize,  // add this
) -> Result<Vec<Value>, MdsError> {
    if depth > MAX_CALL_DEPTH {
        return Err(MdsError::resource_limit("nested argument resolution depth exceeded"));
    }
    // ... pass depth + 1 to recursive call
}
```

**Lexer tokenizes entire source into a `Vec<char>` plus a `Vec<usize>` byte-offset map** - `src/lexer.rs:27-32`
**Confidence**: 80%
- Problem: For every source file (up to 10 MB), the lexer allocates two full-length vectors: `chars: Vec<char>` (4 bytes per char, up to ~40 MB for a 10 MB file) and `byte_offsets: Vec<usize>` (8 bytes per char, up to ~80 MB). Combined with the source string itself, a single 10 MB file could consume ~130 MB of memory just for tokenization. This is within the pre-allocated resource limits but is a steep multiplier. For normal-sized files this is fine, but it approaches problematic territory at the MAX_FILE_SIZE boundary.
- Fix: Consider documenting this memory multiplier as an accepted cost given the 10 MB limit, or reducing MAX_FILE_SIZE to 1-2 MB which would be more reasonable for prompt template files. Alternatively, switch to a streaming/iterator-based approach over `source.char_indices()` to avoid the up-front allocation.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`scope.pop()` error in `evaluate_for` not cleaning up on body evaluation failure** - `src/evaluator.rs:331-344`
**Confidence**: 82%
- Problem: In `evaluate_for`, if `evaluate_nodes` returns an error on any loop iteration, the `scope.pop()` on line 343 is skipped due to early `?` propagation. While this does not cause a user-visible bug today (the error unwinds the entire compilation), it is a latent scope leak if the evaluator ever adds error recovery or partial evaluation in the future. The function body in `invoke_function` (line 218-221) handles this correctly by deferring the `?` until after `call_stack.remove` and `scope.pop()`.
- Fix: Apply the same deferred-error pattern used in `invoke_function`:
```rust
for item in items {
    // ...
    scope.push();
    scope.set_var(&block.var, item);
    let result = evaluate_nodes(&block.body, scope, call_stack, total_iterations, warnings);
    scope.pop()?;
    output.push_str(&result?);
}
```

**`clean_output` iterates char-by-char for a potentially 50 MB string** - `src/lib.rs:277-306`
**Confidence**: 80%
- Problem: The `clean_output` function processes the output string character by character and builds a new `String`. For the maximum output of 50 MB (`MAX_OUTPUT_SIZE`), this means iterating 50M+ characters and performing a final `trim_end()` + `to_string()` copy. This results in two full copies of the output buffer at peak. Not a correctness issue, but a reliability concern at scale since it doubles peak memory.
- Fix: Consider an in-place approach or at minimum document the memory overhead. The function could also short-circuit for small outputs that don't contain triple newlines.

## Pre-existing Issues (Not Blocking)

(none - all code is new on this branch)

## Suggestions (Lower Confidence)

- **Scope frame stack unbounded** - `src/scope.rs:68-70` (Confidence: 65%) -- `scope.push()` has no upper bound on the number of frames. The parser's nesting limit (256) and evaluator's call depth limit (128) indirectly bound this, but the scope itself does not independently enforce a limit. If a future code path pushes frames without going through those bounded paths, the stack could grow unboundedly.

- **`ModuleCache.modules` HashMap unbounded** - `src/resolver.rs:49` (Confidence: 62%) -- The module cache accumulates all resolved modules without any eviction policy or size limit. Combined with the import depth limit of 64, the practical maximum is bounded, but the cache itself does not enforce any limit on the number of entries or total memory consumed by cached `ResolvedModule` values (which include cloned function bodies, namespaces, and captured scopes).

- **`find_project_root` walks up 256 directories** - `src/resolver.rs:16-29` (Confidence: 72%) -- The loop bound of 256 is reasonable for filesystem depth, but the function calls `dir.join(marker).exists()` twice per iteration (512 syscalls worst case). On network-mounted filesystems these calls could be slow. This is unlikely to be a real problem but worth noting.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The compiler demonstrates strong reliability practices overall. All major loops are bounded (MAX_LOOP_ITERATIONS, MAX_TOTAL_ITERATIONS), recursion is detected (call_stack HashSet), import depth is capped (MAX_IMPORT_DEPTH=64), nesting depth is capped (MAX_NESTING_DEPTH=256), file sizes are bounded (MAX_FILE_SIZE=10MB), output size is bounded (MAX_OUTPUT_SIZE=50MB), and cycle detection uses the correct HashSet+Vec dual structure. The `enter_block()`/depth decrement pattern is correctly paired across all three block types (if, for, define). The conditions for approval are:

1. Address the unbounded warnings vector (HIGH) -- add a cap to prevent OOM on adversarial input.
2. Consider the scope cleanup in `evaluate_for` (MEDIUM) -- apply the deferred-error pattern for robustness.

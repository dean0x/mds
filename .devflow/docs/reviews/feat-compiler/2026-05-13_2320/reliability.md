# Reliability Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13

## Issues in Your Changes (BLOCKING)

### HIGH

**Total iteration accounting bypassed for non-leaf outer loops** - `src/evaluator.rs:334-349`
**Confidence**: 85%
- Problem: The `is_leaf_loop` optimization skips iteration counting for outer loops that contain nested `@for` blocks (line 334). However, the outer loop's own iterations are never counted — only the innermost (leaf) loops count. A three-level nested loop with dimensions 100 x 100 x 100 would generate 1,000,000 leaf iterations (exactly at limit), but the outer two loops contribute 100 + 10,000 additional scope push/pop cycles and string concatenations that are completely unaccounted for. More critically, a structure like `@for a in big_array: @for b in small: @end expensive_text @end` where the outer loop has a non-`@for` body node — the outer loop IS a leaf for its text node path but the inner `@for` means `is_leaf_loop` is false, so neither the outer nor inner loop's iterations are properly bounded relative to each other. The real issue: if an outer loop body contains BOTH a `@for` node AND text/interpolation nodes, the outer loop's iterations go uncounted entirely.
- Fix: Count iterations unconditionally for all loops, and set the total limit to account for the expected maximum combined cost. Alternatively, count iterations at every level and use the total limit as-is:
```rust
// Remove the is_leaf_loop optimization entirely:
for item in items {
    *total_iterations += 1;
    if *total_iterations > MAX_TOTAL_ITERATIONS {
        return Err(MdsError::Io {
            message: format!(
                "total loop iterations exceeded maximum of {} across all loops in this compilation",
                MAX_TOTAL_ITERATIONS
            ),
        });
    }
    scope.push();
    // ...
}
```

### MEDIUM

**`validate_var_args` recurses without depth bound** - `src/validator.rs:146-184`
**Confidence**: 82%
- Problem: `validate_var_args` calls itself recursively on `Arg::Call { args: inner_args }` (line 180) without tracking or limiting recursion depth. While the parser's `parse_args_inner` does limit nesting to `MAX_NESTING_DEPTH` (256) during parsing, the validator has no independent depth check. If a code path were to construct an AST programmatically (bypassing the parser), or if the parser's nesting limit were removed in the future, this would stack-overflow. The parser currently provides an implicit bound, but defense-in-depth calls for an explicit check in the validator.
- Fix: Add a depth parameter to `validate_var_args` and check it against `MAX_NESTING_DEPTH`:
```rust
fn validate_var_args(
    args: &[Arg],
    scope: &Scope,
    file: &str,
    source: &str,
    offset: usize,
    depth: usize,
) -> Result<(), MdsError> {
    if depth > 256 {
        return Err(MdsError::syntax("nested argument validation depth exceeded"));
    }
    // ... recurse with depth + 1
}
```

**`find_project_root` loop terminates only via filesystem structure** - `src/resolver.rs:16-28`
**Confidence**: 80%
- Problem: The loop at line 18 walks up the directory tree via `dir.pop()`. On most real filesystems this terminates when hitting the root directory (where `pop()` returns false). However, the loop has no explicit upper bound. Per NASA/JPL Power of Ten Rule 2, all loops must have a fixed upper bound. While this is bounded by the filesystem depth in practice, an explicit iteration cap (e.g., 256 levels) would provide defense-in-depth.
- Fix:
```rust
fn find_project_root(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    for _ in 0..256 {
        for marker in [".git", ".mdsroot"] {
            if dir.join(marker).exists() {
                return dir;
            }
        }
        if !dir.pop() {
            return start.to_path_buf();
        }
    }
    start.to_path_buf()
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`.expect()` calls in `Scope` methods can panic in production** - `src/scope.rs:89,103,117`
**Confidence**: 85%
- Problem: Three methods (`set_var`, `set_function`, `set_namespace`) use `.expect("scope always has at least one frame")` on `self.frames.last_mut()`. While the invariant "frames is never empty" is maintained by the constructor and the guarded `pop()` method, `.expect()` will panic (crash the process) if the invariant is ever violated due to a bug. Per the project's Rust rules: "No `.unwrap()` outside tests". These are reachable from user-facing code paths.
- Fix: Return `Result` from these methods, or use a `debug_assert!` + fallback pattern:
```rust
pub fn set_var(&mut self, name: &str, value: Value) {
    debug_assert!(!self.frames.is_empty(), "scope always has at least one frame");
    if let Some(frame) = self.frames.last_mut() {
        frame.vars.insert(name.to_string(), value);
    }
}
```
Alternatively, since the invariant is structurally enforced (constructor creates one frame, `pop()` refuses to remove it), `debug_assert!` in hot paths and a non-panicking fallback keeps the safety contract without crashing production.

## Pre-existing Issues (Not Blocking)

(None -- all code is new on this branch.)

## Suggestions (Lower Confidence)

- **Output string growth not pre-sized** - `src/evaluator.rs:47` (Confidence: 65%) -- `evaluate_nodes` creates `String::new()` without capacity hints. For large templates with known-size iterables, pre-allocating with `String::with_capacity()` would reduce reallocations. Low impact for typical prompt sizes.

- **Lexer converts entire source to `Vec<char>`** - `src/lexer.rs:27-32` (Confidence: 70%) -- `tokenize` allocates both a `Vec<char>` and a `Vec<usize>` (byte_offsets) proportional to input length. For the 10 MB max file size, this means ~80 MB of transient allocation (8 bytes per char + 8 bytes per offset). Consider operating on byte slices with UTF-8-aware iteration to reduce peak memory.

- **Closure capture clones entire visible scope** - `src/resolver.rs:243-245` (Confidence: 62%) -- `get_all_namespaces()`, `get_all_functions()`, `get_all_vars()` each flatten and clone all scope frames. For modules with many imports/functions, this creates significant intermediate allocation per function definition. Consider lazy capture or Rc-shared scope snapshots.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The compiler demonstrates strong reliability fundamentals: bounded recursion (MAX_CALL_DEPTH=128), bounded import depth (MAX_IMPORT_DEPTH=64), bounded nesting (MAX_NESTING_DEPTH=256), bounded file size (MAX_FILE_SIZE=10MB), bounded output size (MAX_OUTPUT_SIZE=50MB), cycle detection, and bounded per-loop iterations (MAX_LOOP_ITERATIONS=100K). The total iteration accounting for nested loops has a logic gap that could allow outer loop work to go uncounted. The three `.expect()` calls in scope.rs violate the project's no-unwrap-in-production rule but are backed by structural invariants that make panic unlikely in practice. Overall, the bounds and safety checks are well above average for a template compiler.

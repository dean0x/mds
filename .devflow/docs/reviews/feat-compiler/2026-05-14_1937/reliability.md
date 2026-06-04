# Reliability Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**Commits reviewed**: 97b478f..1c2594a (5 commits)

## Issues in Your Changes (BLOCKING)

### HIGH

**Unnecessary file I/O on cache hit in `validate_and_read_file` / `resolve`** - `src/resolver.rs:162`
**Confidence**: 92%
- Problem: The refactored `resolve()` now calls `validate_and_read_file(path)` unconditionally on every call. This performs two `canonicalize()` syscalls, a `std::fs::read()`, and the UTF-8 conversion _before_ checking the module cache at line 165. Previously, the cache check occurred right after `canonicalize()`, before the file was read. On projects with many import statements referencing the same module, this causes redundant disk reads and allocations for every cache hit. The file contents are thrown away immediately when the cache returns a hit.
- Fix: Move the cache check into `validate_and_read_file` or restructure `resolve()` to canonicalize first, check cache, then read:
```rust
pub fn resolve(&mut self, path: &Path, ...) -> Result<Arc<ResolvedModule>, MdsError> {
    // 1. Canonicalize + security checks (no file read yet)
    let (canonical, is_md) = self.validate_path(path)?;

    // 2. Check cache early
    if let Some(cached) = self.modules.get(&canonical) {
        return Ok(Arc::clone(cached));
    }

    // 3. Check for circular imports
    if self.resolving.contains(&canonical) { ... }

    // 4. Now read the file
    let source = self.read_file(&canonical)?;

    // 5. Continue with process_module...
}
```

**`debug_assert!` on LIFO invariant is not checked in release builds** - `src/evaluator.rs:191-194`
**Confidence**: 85%
- Problem: The `debug_assert!` verifying the call_stack LIFO invariant (`ctx.call_stack.last().is_some_and(|s| s == call_key)`) is compiled out in release mode. If this invariant is ever violated (e.g., by a future code change that modifies the call stack during evaluation), the `ctx.call_stack.pop()` on line 195 would silently pop the wrong entry, corrupting recursion detection for all subsequent calls in the compilation. Given that this guards a safety-critical property (recursion detection prevents stack overflow), a `debug_assert!` is insufficient.
- Fix: Promote to a proper assertion or defensive check that survives release builds:
```rust
let popped = ctx.call_stack.pop();
assert!(
    popped.as_deref() == Some(call_key),
    "call_stack LIFO invariant violated: expected '{call_key}' on top, got {:?}",
    popped
);
```
Alternatively, if the cost of the assertion string formatting is a concern, use a conditional return of an `MdsError::syntax("internal error: ...")` which aligns with how `scope.pop()` handles its invariant violation.

### MEDIUM

**`evaluate_for` pops scope before consuming render result** - `src/evaluator.rs:287-289`
**Confidence**: 82%
- Problem: In `evaluate_for`, `evaluate_nodes` returns a `Result<String, MdsError>`. The result is bound to `rendered` (a `Result`), then `scope.pop()?` is called, and only then is `rendered?` unwrapped on line 289. If `evaluate_nodes` returns `Err`, `scope.pop()` still runs -- this is correct. However, the _previous_ version had the `scope.pop()` call happen _after_ the `?` operator would propagate the error (line 341 old: `let rendered = evaluate_nodes(...)?;`). The new ordering means that on error, the scope is still popped cleanly before propagating -- this is actually an improvement for cleanup. However, the pattern is subtle: if `scope.pop()` itself fails (returns Err) after a `evaluate_nodes` error, the `evaluate_nodes` error is silently lost and replaced by the `scope.pop()` error. This is a minor concern since `scope.pop()` only fails on a compiler bug (global frame underflow).
- Fix: This is a minor robustness nit. If you want to preserve both errors, consider logging the scope.pop error and returning the original:
```rust
let rendered = evaluate_nodes(&block.body, scope, ctx);
if let Err(pop_err) = scope.pop() {
    // If we also have a render error, prefer it (pop failure is a compiler bug)
    if rendered.is_err() {
        return rendered;
    }
    return Err(pop_err);
}
output.push_str(&rendered?);
```
Alternatively, accept the current behavior and add a comment explaining that the scope.pop error takes precedence on double-fault (a compiler bug scenario).

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`expect("BUG: scope has no frames")` panics instead of returning Result** - `src/scope.rs:104`, `src/scope.rs:118`, `src/scope.rs:132`
**Confidence**: 83%
- Problem: `set_var`, `set_function`, and `set_namespace` all use `.expect("BUG: scope has no frames")` which will panic if `frames` is empty. The `pop()` method returns `Result` to handle its invariant violation gracefully (returning an error diagnostic instead of panicking). The `set_*` methods are inconsistent -- they panic on the same class of invariant violation. While the comment correctly notes that `Scope::new()` guarantees at least one frame and `pop()` refuses to remove the last, a defensive posture would return `Result` here too, consistent with how `pop()` handles its invariant.
- Fix: The `expect` is acceptable given the documented invariant, but consider aligning with `pop()`'s pattern by returning `Result<(), MdsError>` from these methods. Alternatively, add an `assert!` at the top of `set_var`/`set_function`/`set_namespace` that is more descriptive. The current approach is functional but represents a reliability inconsistency -- `pop()` was explicitly changed to return Result to avoid panics on invariant violation, yet `set_*` methods still panic on the analogous invariant.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`collect_all` uses `flat_map` + `collect` with HashMap, relying on last-wins for shadowing** - `src/scope.rs:174-177`
**Confidence**: 80%
- Problem: The `collect_all` method iterates frames in outer-to-inner order and calls `.collect()` on a `HashMap`. HashMap's `FromIterator` implementation calls `insert` for each item, so duplicate keys are overwritten by the last (innermost) value -- which gives correct shadowing. However, this relies on an _implementation detail_ of `HashMap::from_iter`. While this behavior is documented and stable in Rust, the code has no comment explaining why iteration order matters for correctness. A future refactor that changes the iteration order (e.g., using `.rev()`) would silently break shadowing.
- Fix: Add a comment to `collect_all` explaining the correctness dependency:
```rust
// HashMap::from_iter overwrites duplicate keys, so iterating outer→inner
// ensures the innermost (most recent) binding wins — correct shadowing.
```

## Suggestions (Lower Confidence)

- **Unbounded String growth in `scan_text`/`scan_code_content`/`scan_directive`** - `src/lexer.rs:176,201,284` (Confidence: 65%) -- The `content`/`line`/`text` Strings in the lexer `scan_*` methods grow character-by-character via `push()`. For very large files (up to the 10MB limit), this causes repeated reallocations. Pre-sizing with a heuristic capacity or using slicing instead of character-by-character accumulation would reduce allocation pressure.

- **`load_config` reads `mds.json` without size check** - `src/main.rs:51` (Confidence: 72%) -- `std::fs::read_to_string` is called on `mds.json` without any size limit. While `mds.json` is expected to be small, the resolver applies `MAX_FILE_SIZE` to all files it reads. A maliciously large `mds.json` could cause memory exhaustion. Consider adding a size check consistent with the resolver's pattern.

- **`validate_and_read_file` performs security checks before checking the cache** - `src/resolver.rs:71-153` (Confidence: 70%) -- Related to the HIGH finding above, but from a different angle: the symlink detection, root_dir check, and depth check all happen before the cache lookup. These checks involve filesystem syscalls that are wasted on cache hits. Moving the cache check earlier (after canonicalization but before security checks) would be safe because cached modules have already passed all security checks on their first resolution.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Reliability Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The changes demonstrate strong reliability engineering overall: resource limits are well-defined and enforced (MAX_CALL_DEPTH, MAX_LOOP_ITERATIONS, MAX_TOTAL_ITERATIONS, MAX_OUTPUT_SIZE, MAX_WARNINGS), the `EvalContext` consolidation reduces the risk of parameter-threading bugs, `scope.pop()` returns Result instead of panicking, and the `IndexSet` replacement simplifies cycle detection while preserving ordering. The `Arc<FunctionDef>` / owned `CapturedScope::functions` split correctly breaks reference cycles.

The primary reliability concern is the redundant file I/O on cache hits in the refactored `resolve()` path, which degrades performance proportionally to the number of duplicate imports. The `debug_assert!`-only LIFO check is also worth promoting to a release-mode guard given that it protects recursion detection.

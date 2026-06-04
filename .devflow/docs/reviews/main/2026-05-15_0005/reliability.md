# Reliability Review Report

**Branch**: main (ba816b5)
**Base**: d0624a2
**Date**: 2026-05-15
**Scope**: Full codebase review -- all source files in `src/`

## Issues in Your Changes (BLOCKING)

### HIGH

**Potential panic from unchecked index on `names[0]`** - `src/main.rs:224`
**Confidence**: 90%
- Problem: In `auto_detect_mds_file()`, the `names` vector is built via `filter_map` with `file_name().and_then(|n| n.to_str())`. On systems with non-UTF-8 filenames (Linux with certain locales), all entries could be filtered out, leaving `names` empty. The subsequent `names[0]` at line 224 would panic with an index-out-of-bounds.
- Impact: Process abort on systems with non-UTF-8 filenames when multiple `.mds` files exist in the directory.
- Fix:
```rust
_ => {
    let mut names: Vec<String> = entries
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_owned))
        .collect();
    names.sort();
    let hint_name = names.first().map(|s| s.as_str()).unwrap_or("<file>.mds");
    Err(miette::miette!(
        "multiple .mds files found: {}\n  \
         hint: specify which file to compile, e.g. 'mds build {}'",
        names.join(", "),
        hint_name,
    ))
}
```

### MEDIUM

**TOCTOU gap in `mds.json` size check** - `src/main.rs:55-65`
**Confidence**: 80%
- Problem: `load_config` checks the file size via `metadata().len()` and then reads the file with `read_to_string()`. Between the two syscalls, the file could be replaced with a larger one. The resolver (`read_validated_file` at `resolver.rs:148-151`) correctly handles this by reading bytes first, then checking size. The `mds.json` loader does not follow the same pattern.
- Impact: In a local CLI tool, the practical risk is low. A malicious actor replacing `mds.json` between the two calls could cause unbounded memory allocation. This is defense-in-depth rather than a likely exploit.
- Fix: Use the same read-then-check pattern as `resolver.rs`:
```rust
let bytes = std::fs::read(&candidate).map_err(|e| {
    miette::miette!("cannot read {}: {e}", candidate.display())
})?;
if bytes.len() as u64 > MAX_CONFIG_SIZE {
    return Err(miette::miette!(
        "mds.json at {} is too large ({} bytes; maximum is 1 MB)",
        candidate.display(), bytes.len()
    ));
}
let raw = String::from_utf8(bytes).map_err(|e| {
    miette::miette!("invalid UTF-8 in {}: {e}", candidate.display())
})?;
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`assert_eq!` in production code can panic** - `src/resolver.rs:207`
**Confidence**: 82%
- Problem: The LIFO invariant check on `resolving.pop()` uses `assert_eq!`, which will abort the process with a panic if violated. The comment correctly identifies this as safety-critical, but the evaluator's equivalent check at `evaluator.rs:208-215` returns a structured `MdsError` instead of panicking. The two modules handle the same class of invariant differently.
- Impact: If a compiler bug corrupts the resolving stack, the user gets an opaque panic backtrace instead of a structured error diagnostic. The evaluator already demonstrates the preferred pattern.
- Fix: Return a structured error like the evaluator does:
```rust
let popped = self.resolving.pop();
if popped.as_ref() != Some(&canonical) {
    return Err(MdsError::syntax(format!(
        "internal error: resolving stack LIFO violated: expected '{}', got {:?}",
        canonical.display(), popped
    )));
}
```

**`expect()` calls in `Scope` methods can panic on invariant violation** - `src/scope.rs:106,120,134`
**Confidence**: 80%
- Problem: `set_var`, `set_function`, and `set_namespace` all use `.expect("BUG: scope has no frames")`. The code comment correctly argues that `frames` is never empty because `new()` pushes one frame and `pop()` refuses to remove the last. However, the `pop()` method itself returns `Result` rather than panicking, creating an inconsistency: if a bug causes `pop()` to be called too many times, the `expect()` calls become the crash point. In a public API, defensive error returns are preferable to panics.
- Impact: If a compiler bug leads to a state where `frames` is empty (e.g., through unsafe code or future refactoring), these `expect()` calls would panic instead of returning a structured error. The invariant is currently upheld, so this is defensive hardening.
- Fix: Consider returning `Result` from these methods, or document the panic contract explicitly with `#[doc(hidden)]` or `/// # Panics` doc annotations. Given the structural guarantee, this is LOW priority.

## Pre-existing Issues (Not Blocking)

(No pre-existing issues -- all code is new in this branch.)

## Suggestions (Lower Confidence)

- **Closure capture may clone large scope trees** - `src/resolver.rs:552-560` (Confidence: 65%) -- `collect_define` calls `scope.get_all_functions()` which clones every `Arc<FunctionDef>` in all frames. For deeply nested modules with many functions, this could allocate significantly. The `Arc` cloning is O(1) per entry, but the map allocation and iteration is O(n) per `@define`. If a module has many functions and many defines, this becomes O(n*m). Consider lazy capture or capture-on-demand if profiling shows this is a bottleneck.

- **Validator clones full scope for `@for` and `@define` body validation** - `src/validator.rs:59,64` (Confidence: 60%) -- `scope.clone()` copies all frames including all hashmaps. For large scopes with many variables/functions, this could be expensive. A cheaper approach would be to push/pop a temporary frame instead of cloning the entire scope.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

### Positive Observations

The codebase demonstrates strong reliability engineering throughout:

1. **Comprehensive resource limits**: Every unbounded operation has an explicit cap:
   - `MAX_CALL_DEPTH` (128) for recursion depth
   - `MAX_LOOP_ITERATIONS` (100,000) per loop and `MAX_TOTAL_ITERATIONS` (1,000,000) across all loops
   - `MAX_OUTPUT_SIZE` (50 MB) for output buffer
   - `MAX_NESTING_DEPTH` (256) for parser block nesting
   - `MAX_IMPORT_DEPTH` (64) for import chain depth
   - `MAX_FILE_SIZE` (10 MB) for input files
   - `MAX_CONFIG_SIZE` (1 MB) for project config
   - `MAX_VALUE_DEPTH` (64) for YAML/JSON value nesting
   - `MAX_WARNINGS` (1,000) for accumulated warnings
   - Bounded directory traversal (256 iterations) in `find_project_root` and `load_config`

2. **Cycle detection**: Import cycles are detected using `IndexSet` with insertion-order preservation for clear error messages.

3. **Error propagation**: The `?` operator is used consistently. `MdsError` is a structured error enum with source-span diagnostics via `miette`. No `unwrap()` or `expect()` in hot paths (only in scope frame access with structural invariant guarantees).

4. **LIFO invariant enforcement**: Both the evaluator call stack and resolver resolving set enforce LIFO ordering with explicit checks (evaluator returns `Result`, resolver uses `assert_eq!`).

5. **Security at boundaries**: Path traversal prevention, symlink detection, null byte rejection, and relative-path-only imports are all enforced.

6. **Graceful malformed input handling**: The lexer and parser return structured errors for all malformed input (unclosed braces, unclosed code fences, unclosed frontmatter, invalid identifiers, unknown directives).

7. **No thread safety issues**: The codebase is single-threaded by design. `Arc` is used for cheap cloning of `FunctionDef` and `NamedSource`, not for cross-thread sharing.

8. **`#[must_use]` on public API**: All public compile/check functions have `#[must_use]` annotations.

**Reliability Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions:
- Fix the `names[0]` panic path in `auto_detect_mds_file` (HIGH -- potential process abort)
- Consider aligning the `mds.json` size check pattern with the resolver's read-then-check approach (MEDIUM -- defense-in-depth)

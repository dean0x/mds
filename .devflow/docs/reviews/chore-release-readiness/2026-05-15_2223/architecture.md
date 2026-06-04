# Architecture Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### HIGH

**`resolve_input` extracted but only used by `run_check` -- inconsistent decomposition** - `src/main.rs:440-445,457-466`
**Confidence**: 85%
- Problem: The new `resolve_input` helper was extracted to consolidate the "resolve path or auto-detect" pattern, but `run_build` duplicates this logic inline (with the additional `eprintln!` for the banner message) rather than using `resolve_input`. This creates an asymmetry: `run_check` uses the helper, `run_build` does not. The SRP extraction is incomplete -- the two code paths can drift independently.
- Fix: Either have both functions call `resolve_input` (and handle the banner separately), or remove `resolve_input` and keep the inline patterns in both. The cleanest approach would be to have `resolve_input` return the path along with a boolean indicating whether auto-detection was used, then let `run_build` conditionally print the banner:
```rust
fn resolve_input(input: Option<PathBuf>) -> std::result::Result<(PathBuf, bool), miette::Error> {
    match input {
        Some(p) => Ok((p, false)),
        None => auto_detect_mds_file().map(|p| (p, true)),
    }
}
```

### MEDIUM

**Duplicated `MAX_TRAVERSAL_DEPTH` constant across two modules** - `src/main.rs:29`, `src/resolver.rs:47`
**Confidence**: 82%
- Problem: Both `main.rs` and `resolver.rs` define `const MAX_TRAVERSAL_DEPTH: usize = 256` independently. While the KNOWLEDGE.md acknowledges this ("they are separate named constants in their respective modules"), this violates the DRY principle and creates a coupling risk -- if one is updated without the other, the traversal bounds silently diverge. The two constants serve the identical purpose (bounding upward directory walks).
- Fix: Define a single constant in a shared location. Since `resolver.rs` already exports `MAX_FILE_SIZE` via `pub(crate)`, add `MAX_TRAVERSAL_DEPTH` similarly and import it in `main.rs`:
```rust
// src/resolver.rs
pub(crate) const MAX_TRAVERSAL_DEPTH: usize = 256;

// src/main.rs
use mds::resolver::MAX_TRAVERSAL_DEPTH;  // or via lib.rs re-export
```
Alternatively, define it in `lib.rs` alongside `MAX_FILE_SIZE` since both are cross-cutting resource limits.

**LIFO invariant check placed after `resolved?` can suppress the compiler bug diagnostic** - `src/resolver.rs:212-222`
**Confidence**: 80%
- Problem: The new error-returning LIFO check (replacing the old `assert_eq!`) intentionally prioritizes the module processing error over the LIFO violation. However, if `process_module` returns `Ok(resolved)` but the pop returns the wrong entry, the LIFO error is correctly surfaced. The concern is the opposite: if `process_module` fails AND the LIFO invariant is violated, the LIFO corruption (a compiler bug) is silently swallowed by `resolved?`. The comment says this is intentional ("prefer the user-facing module error"), but a LIFO violation is a safety-critical state corruption that should at minimum be logged, as the module error's diagnostic may itself be misleading when the resolving stack is corrupted.
- Fix: Log the LIFO violation even when the module error takes precedence, so the compiler bug is not completely invisible:
```rust
let resolved = match resolved {
    Ok(r) => r,
    Err(e) => {
        if popped.as_ref() != Some(&canonical) {
            eprintln!("internal: resolving stack LIFO invariant violated (suppressed by module error)");
        }
        return Err(e);
    }
};
if popped.as_ref() != Some(&canonical) {
    return Err(MdsError::syntax(
        "internal error: resolving stack LIFO invariant violated -- this is a compiler bug, please report it",
    ));
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`ResolvedModule` fields are all `pub` -- leaky abstraction for a crate-internal type** - `src/resolver.rs:36-41`
**Confidence**: 82%
- Problem: `ResolvedModule` has all fields as `pub` (`functions`, `prompt_body`, `has_explicit_exports`, `explicit_exports`), but it already has proper accessor methods (`get_export`, `get_all_exports`, `get_prompt_value`, `to_namespace`, `is_exported`). Since the PR adds `#[non_exhaustive]` and `pub(crate)` to `MdsError` and `Value` to harden the API surface, `ResolvedModule` fields should follow the same pattern. Direct field access bypasses the export visibility logic in `get_export`/`get_all_exports`, which is the exact kind of bug the accessors were designed to prevent.
- Fix: Change fields to `pub(crate)` since `ResolvedModule` is used within the crate but callers should go through the accessor methods:
```rust
pub struct ResolvedModule {
    pub(crate) functions: HashMap<String, Arc<FunctionDef>>,
    pub(crate) prompt_body: Option<String>,
    pub(crate) has_explicit_exports: bool,
    pub(crate) explicit_exports: HashSet<String>,
}
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`ModuleCache` has no mechanism to limit total cached modules** - `src/resolver.rs:54-62`
**Confidence**: 80%
- Problem: `ModuleCache::modules` is an unbounded `HashMap<PathBuf, Arc<ResolvedModule>>`. A project with hundreds of `.mds` files resolved transitively would accumulate all of them in memory. Combined with `Arc<ResolvedModule>` containing full `HashMap<String, Arc<FunctionDef>>` trees, memory growth is proportional to the total number of unique modules. The existing `MAX_IMPORT_DEPTH = 64` only limits chain depth, not the total number of distinct modules.
- Impact: For v0.1 this is acceptable since MDS projects are expected to be small. Worth noting for future releases.

## Suggestions (Lower Confidence)

- **`check_symlink` is a static method but other check methods are instance methods** - `src/resolver.rs:74` (Confidence: 65%) -- `check_symlink` takes `path: &Path` without `&self`, while `check_import_depth` and `check_path_traversal` take `&self`. This asymmetry is functionally correct (symlink detection does not need cache state) but creates a mixed calling convention in `canonicalize_and_check` (`Self::check_symlink(path)?` vs `self.check_import_depth()?`). Consider making the convention explicit with a comment or consistently using free functions for stateless checks.

- **Validator `scope.pop()` result discarded with `let _ =`** - `src/validator.rs:62,74` (Confidence: 70%) -- The KNOWLEDGE.md correctly documents this is safe ("cannot fail -- we just pushed"), and the anti-patterns section carves out this exception. However, using `let _ = scope.pop()` is an exception to the project's own documented rule ("Always use `scope.pop()?`"). A brief inline comment explaining the safety argument (already present) is the minimum; a `debug_assert!(result.is_ok())` would be more robust.

- **`run_build` does not use `resolve_input` for the auto-detect banner behavior** - `src/main.rs:457-466` (Confidence: 75%) -- Overlaps with the blocking issue above. The banner-printing behavior in `run_build` prevents reuse of `resolve_input`, suggesting the helper's interface could be richer.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The PR demonstrates strong architectural discipline overall. The extraction of `run_build`/`run_check`/`run_init` from the monolithic `run()` follows SRP well. The decomposition of `canonicalize_and_check` into focused `check_symlink`/`check_import_depth`/`check_path_traversal` helpers improves cohesion. The `#[non_exhaustive]` and `pub(crate)` hardening on `MdsError` and `Value` is correct API surface management for a v0.1 crate release. The import handler extraction (`resolve_alias_import`/`resolve_merge_import`/`resolve_selective_import`) reduces `resolve_import` to a clean dispatcher. The validator push/pop optimization eliminates unnecessary scope cloning without changing semantics.

The conditions are: (1) resolve the `resolve_input` inconsistency between `run_build` and `run_check`, and (2) consider the duplicated `MAX_TRAVERSAL_DEPTH` constant to prevent silent divergence.

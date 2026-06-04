# Rust Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**File I/O performed before cache check in `resolve()`** - `src/resolver.rs:162`
**Confidence**: 92%
- Problem: `validate_and_read_file(path)` is called at line 162, which performs full symlink detection, two `canonicalize()` calls, and reads the entire file into memory. Only afterward (line 165) is the module cache checked. On cache hits, the file was read and discarded unnecessarily. The previous implementation checked the cache before reading the file, so this is a regression. For projects with many transitive imports that hit the same module repeatedly, the redundant I/O scales linearly with the number of import statements rather than the number of unique modules.
- Fix: Split `validate_and_read_file` into two phases: (1) a cheap path-canonicalization step that returns the canonical path (needed for cache lookup), and (2) a file-read step that only runs on cache miss.

```rust
// Phase 1: canonicalize and security checks (no file read)
fn canonicalize_and_check(&mut self, path: &Path) -> Result<PathBuf, MdsError> {
    // symlink check, root_dir check, depth check — but NOT fs::read
    ...
}

pub fn resolve(...) -> Result<Arc<ResolvedModule>, MdsError> {
    let canonical = self.canonicalize_and_check(path)?;

    // Check cache BEFORE reading file
    if let Some(cached) = self.modules.get(&canonical) {
        return Ok(Arc::clone(cached));
    }

    // Cache miss — now read the file
    let (source, is_md) = self.read_validated_file(&canonical)?;
    ...
}
```

### MEDIUM

**`load_config` reads `mds.json` without size guard** - `src/main.rs:51`
**Confidence**: 82%
- Problem: `load_config` calls `std::fs::read_to_string(&candidate)` at line 51 without checking the file size first. The rest of the codebase consistently applies `MAX_FILE_SIZE` checks before reading (resolver and `load_vars_file`). A maliciously large `mds.json` in a parent directory could cause excessive memory allocation. While this is a CLI tool (limiting exposure), the inconsistency with the established pattern is worth addressing.
- Fix: Read as bytes first and check size, matching the pattern used in `validate_and_read_file` and `load_vars_file`. A reasonable cap for a config file would be much smaller than `MAX_FILE_SIZE` (e.g. 1 MB or even 64 KB).

```rust
let bytes = std::fs::read(&candidate).map_err(|e| {
    miette::miette!("cannot read {}: {e}", candidate.display())
})?;
if bytes.len() > 1_048_576 {  // 1 MB — generous for a config file
    return Err(miette::miette!(
        "mds.json at {} is too large ({} bytes)",
        candidate.display(), bytes.len()
    ));
}
let raw = String::from_utf8(bytes).map_err(|e| {
    miette::miette!("invalid UTF-8 in {}: {e}", candidate.display())
})?;
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`expect()` in scope setters could panic in library context** - `src/scope.rs:104,118,132`
**Confidence**: 82%
- Problem: `set_var`, `set_function`, and `set_namespace` all use `.expect("BUG: scope has no frames")`. The documentation comment explains the invariant (constructor pushes a frame, `pop()` refuses to remove the last one), and the `pop()` method correctly returns `Result`. However, the `Scope` struct is `pub` and these methods are `pub` -- a downstream library consumer could construct a `Scope` via other means or modify `frames` directly (it is a struct field). Per the Rust skill checklist: no `.unwrap()` or `.expect()` in library code. The previous code used `debug_assert!` + `.unwrap()`, which had the same issue but at least the assert was stripped in release builds. The current `expect()` panics unconditionally.
- Fix: Since the invariant is genuinely guaranteed by the constructor/pop pair and `frames` is private, this is acceptable for an internal-use type. However, if `Scope` is ever exposed as public API, consider returning `Result` from these methods or using `debug_assert!` before the `expect()` to document the invariant. The current approach is defensible given the private `frames` field, but adding a `debug_assert!(!self.frames.is_empty())` before the `expect` would make the intent even clearer without runtime cost in release builds.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`HashMap` used for `modules` cache in `ModuleCache`** - `src/resolver.rs:52`
**Confidence**: 80%
- Problem: `modules: HashMap<PathBuf, Arc<ResolvedModule>>` uses `HashMap` which has non-deterministic iteration order. While this does not affect correctness (the cache is only used for lookup, not iteration), switching to `IndexMap` would provide deterministic behavior for debugging and would be consistent with the `IndexSet` already used for `resolving`.
- Fix: Consider `IndexMap<PathBuf, Arc<ResolvedModule>>` for consistency. Low priority since correctness is not affected.

## Suggestions (Lower Confidence)

- **`CollectedDefs` type alias is a bare tuple** - `src/resolver.rs:512` (Confidence: 68%) -- `type CollectedDefs = (HashMap<String, Arc<FunctionDef>>, bool, HashSet<String>)` could be a named struct for clarity. Tuple position is easy to confuse when deconstructing. A struct with named fields (`functions`, `has_explicit_exports`, `explicit_exports`) would be self-documenting.

- **`EvalContext` fields are private but struct is `pub(crate)`** - `src/evaluator.rs:28` (Confidence: 65%) -- `EvalContext` has private fields but is `pub(crate)`. This is fine for the current single-crate architecture, but if the evaluator is ever extracted into a separate crate, the lack of a public constructor would be a barrier. Consider adding a `new()` constructor method for completeness.

- **`serde_yml` version is 0.0.12** - `Cargo.toml:11` (Confidence: 62%) -- The `serde_yml` crate is at version 0.0.12, indicating pre-stability. The previous `serde_yaml` was deprecated but at 0.9.x. Verify that `serde_yml` has sufficient community adoption and maintenance for a production dependency. The `libyml` transitive dependency it brings in depends on `anyhow`, which adds a non-trivial dependency to what was previously a `thiserror`-only error strategy.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Rust Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The overall Rust quality is strong. The refactoring demonstrates good Rust idioms: `Arc<FunctionDef>` for O(1) clone at storage layers with owned `FunctionDef` in `CapturedScope` to break reference cycles, `IndexSet` replacing a redundant `HashSet`+`Vec` pair, `EvalContext` reducing parameter threading, proper `#[must_use]` on `MdsError`, `thiserror`+`miette` for typed diagnostics, and `Result` returns throughout. The lexer decomposition into a `Lexer<'a>` struct with focused `scan_*` methods is clean and idiomatic.

The one high-severity issue -- reading files before checking the cache -- is a performance regression that should be addressed before merge. The file read overhead is wasted work on every cache hit, and the fix (splitting canonicalization from file reading) is straightforward.

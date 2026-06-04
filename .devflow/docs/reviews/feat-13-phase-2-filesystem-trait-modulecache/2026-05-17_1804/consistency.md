# Consistency Review Report

**Branch**: feat-13-phase-2-filesystem-trait-modulecache -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**`is_markdown` implementation divergence between VirtualFs and NativeFs** - `crates/mds-core/src/fs.rs:121-123` vs `crates/mds-core/src/fs.rs:261-266`
**Confidence**: 85%
- Problem: `VirtualFs::is_markdown` uses a simple `normalized.ends_with(".md")` check, while `NativeFs::is_markdown` uses `Path::new(normalized).extension().and_then(|e| e.to_str()) == Some("md")`. These differ in edge cases: `ends_with(".md")` would return `true` for a key like `"foo.md/bar"` or `"something_md"` (though neither is likely), and more importantly `Path::extension()` handles double-extensions differently (e.g. `"file.test.md"` works with both, but conceptually they use different strategies). When two implementations of the same trait method use different algorithms for the same semantic check, it creates a maintenance and correctness divergence risk.
- Fix: Unify to a single approach. The `Path::extension()` approach is more robust:
```rust
fn is_markdown(&self, normalized: &str) -> bool {
    // Match NativeFs approach: split on '.' for proper extension handling
    normalized.rsplit('.').next() == Some("md")
}
```
Or alternatively, make `NativeFs` use the simpler string check to match `VirtualFs`. Either is fine, but they should be the same algorithm.

### MEDIUM

**`compile_virtual` does not follow the established delegation pattern** - `crates/mds-core/src/lib.rs:440-457`
**Confidence**: 82%
- Problem: The existing public API follows a two-tier pattern: simple functions (`compile`, `compile_str`) delegate to `_collecting_warnings` variants, which do the actual work. For example, `compile()` delegates to `compile_collecting_warnings()`. The new `compile_virtual()` breaks this pattern by inlining the full pipeline (cache creation, resolution, body extraction, frontmatter prepending, warning emission) directly. There is no `compile_virtual_collecting_warnings` variant, meaning callers who need programmatic access to warnings (like the CLI's `--quiet` flag) have no option.
- Fix: Add a `compile_virtual_collecting_warnings` function and have `compile_virtual` delegate to it, matching the established pattern:
```rust
pub fn compile_virtual_collecting_warnings(
    modules: HashMap<String, String>,
    entry: &str,
    runtime_vars: Option<HashMap<String, Value>>,
) -> Result<(String, Vec<String>), MdsError> {
    let vars = runtime_vars.unwrap_or_default();
    let mut cache = ModuleCache::virtual_fs(modules);
    let mut warnings = vec![];
    let resolved = cache.resolve_key(entry, &vars, &mut warnings)?;
    let body = resolved
        .prompt_body
        .as_deref()
        .map(clean_output)
        .unwrap_or_default();
    let output = prepend_frontmatter(resolved.raw_frontmatter.as_deref(), body);
    Ok((output, warnings))
}

pub fn compile_virtual(
    modules: HashMap<String, String>,
    entry: &str,
    runtime_vars: Option<HashMap<String, Value>>,
) -> Result<String, MdsError> {
    let (output, warnings) = compile_virtual_collecting_warnings(modules, entry, runtime_vars)?;
    emit_warnings(&warnings);
    Ok(output)
}
```

**Module visibility: `fs` is `pub mod` while all other internal modules are `pub(crate) mod`** - `crates/mds-core/src/lib.rs:43`
**Confidence**: 83%
- Problem: Every other module in `lib.rs` (ast, error, evaluator, lexer, limits, parser, resolver, scope, validator, value) is declared as `pub(crate) mod`, with only specific types re-exported via `pub use`. The `fs` module is declared as `pub mod`, exposing its entire module path (e.g. `mds::fs::VirtualFs`) in addition to the re-exports at `mds::VirtualFs`. This is inconsistent with the crate's established encapsulation pattern where internal module structure is hidden and only specific types are exposed at the crate root.
- Fix: Change to `pub(crate) mod fs;` to match the other modules. The `pub use fs::{FileSystem, NativeFs, VirtualFs};` on line 52 already re-exports the needed types at the crate root:
```rust
pub(crate) mod fs;
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`validate_file_type` uses a third distinct extension-extraction algorithm** - `crates/mds-core/src/resolver.rs:702-705`
**Confidence**: 82%
- Problem: There are now three different approaches to extracting a file extension in this PR: (1) `VirtualFs::is_markdown` uses `ends_with(".md")`, (2) `NativeFs::is_markdown` uses `Path::extension()`, (3) `validate_file_type` uses `rsplit('.').next().filter(|e| *e != filename)`. While all three produce correct results for the common case, having three different extension-extraction strategies in the same codebase is a consistency smell. If the definition of "what is an extension" ever needs to change, three callsites need updating.
- Fix: Consider extracting a shared helper function for extension extraction from a key string, then using it in both `validate_file_type` and `VirtualFs::is_markdown`:
```rust
/// Extract the file extension from a normalized key string.
fn key_extension(key: &str) -> Option<&str> {
    let filename = key.rsplit(['/', '\\']).next().unwrap_or(key);
    filename.rsplit('.').next().filter(|e| *e != filename)
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing `check_virtual` counterpart** - `crates/mds-core/src/lib.rs` (Confidence: 65%) -- The existing API provides `compile` / `check` and `compile_str` / `check_str` pairs. The new `compile_virtual` lacks a corresponding `check_virtual`, breaking the compile/check symmetry. This may be intentional for Phase 2 scope, but is worth noting for future phases.

- **`resolve_key` is a thin wrapper over `resolve_by_key` with no additional logic** - `crates/mds-core/src/resolver.rs:213-220` (Confidence: 62%) -- `resolve_key` simply forwards to `resolve_by_key` with no transformation. While it exists to provide a public API surface (since `resolve_by_key` is private), the doc comment says "Resolve a module by its normalized key" which is identical to the `resolve_by_key` doc comment. Consider whether the public method should add any validation (e.g. empty key check) to justify its existence as a distinct method.

- **`NativeFs` missing `Debug` derive** - `crates/mds-core/src/fs.rs:132` (Confidence: 60%) -- `VirtualFs` does not derive `Debug` either, so this is consistent within the new code. However, `ResolvedModule` derives `Debug` + `Clone`, and `ModuleCache` could benefit from `Debug` for diagnostic purposes. Both `VirtualFs` and `NativeFs` contain only standard types that support `Debug`.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR is well-structured and the FileSystem trait abstraction is clean. The main consistency concerns are: (1) the `is_markdown` implementations use divergent algorithms across `VirtualFs` and `NativeFs`, which could cause subtle behavioral differences; (2) `compile_virtual` does not follow the established two-tier delegation pattern (simple -> `_collecting_warnings`); and (3) the `fs` module visibility breaks the `pub(crate) mod` convention used by every other module. None of these are blocking at CRITICAL level, but addressing the HIGH-severity `is_markdown` divergence would strengthen the trait contract before it becomes a public API that others implement against.

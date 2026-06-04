# Consistency Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**NativeFs::read uses metadata pre-check, deviating from the established TOCTOU-safe pattern** - `crates/mds-core/src/fs.rs:280-301`
**Confidence**: 90%
- Problem: The project has a documented and consistently applied pattern for file size checking: "read bytes first, then check size (avoids TOCTOU race)". This pattern was used in the original `read_validated_file` (on main) and is documented in KNOWLEDGE.md as a constraint and anti-pattern ("Using `metadata().len()` before `read()` for size checks ... introduces a TOCTOU race"). The new `NativeFs::read` adds a `metadata()` pre-check before the read, then also checks after reading. While the post-read check provides defense-in-depth, the pre-check itself re-introduces the TOCTOU race window that the project explicitly documents should be avoided. The comment calls this "defense-in-depth" but the project's stated pattern is the opposite: the read-first-then-check IS the defense.
- Fix: Remove the metadata pre-check and keep only the post-read size check, matching the original `read_validated_file` pattern and the `load_vars_file` pattern in `lib.rs`:
```rust
fn read(&self, normalized: &str) -> Result<String, MdsError> {
    let path = Path::new(normalized);
    let bytes = std::fs::read(path)
        .map_err(|e| MdsError::io(format!("cannot read {normalized}: {e}")))?;
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(MdsError::resource_limit(format!(
            "file too large ({} bytes, max {} bytes): {normalized}",
            bytes.len(),
            MAX_FILE_SIZE,
        )));
    }
    String::from_utf8(bytes)
        .map_err(|e| MdsError::io(format!("invalid UTF-8 in {normalized}: {e}")))
}
```

### MEDIUM

**NativeFs::normalize omits empty-path validation present in VirtualFs::normalize** - `crates/mds-core/src/fs.rs:253`
**Confidence**: 85%
- Problem: `VirtualFs::normalize` explicitly rejects empty `relative` paths at the top of the method (line 81-83). The `FileSystem` trait doc comment lists "Input sanitization: `normalize` must reject empty paths" as a security contract obligation. However, `NativeFs::normalize` has no such check. While an empty path would likely fail downstream at the `canonicalize()` step, the inconsistency between the two implementations and the trait's own documented security contract is a pattern deviation.
- Fix: Add the same empty-path guard at the top of `NativeFs::normalize`:
```rust
fn normalize(&self, base: &str, relative: &str) -> Result<String, MdsError> {
    if relative.is_empty() {
        return Err(MdsError::import_error("import path is empty"));
    }
    if relative.contains('\0') {
        // ...existing code...
    }
```

**Missing two-tier API symmetry: `compile_virtual` exists but `check_virtual` does not** - `crates/mds-core/src/lib.rs:440-489`
**Confidence**: 82%
- Problem: The existing public API follows a strict two-tier pattern for every entry point: `compile` / `compile_collecting_warnings`, `compile_str` / `compile_str_with` / `compile_str_collecting_warnings`, `check` / `check_collecting_warnings`, `check_str` / `check_str_with` / `check_str_collecting_warnings`. The new `compile_virtual` / `compile_virtual_collecting_warnings` pair breaks this symmetry by not providing `check_virtual` / `check_virtual_collecting_warnings` counterparts. The KNOWLEDGE.md documents: "All compile/check functions funnel through `ModuleCache::resolve`" -- the check variants share the same pipeline without the evaluation step.
- Fix: Add `check_virtual` and `check_virtual_collecting_warnings` functions following the same pattern as existing `check` / `check_collecting_warnings`:
```rust
#[must_use = "errors should be handled"]
pub fn check_virtual(
    modules: HashMap<String, String>,
    entry: &str,
    runtime_vars: Option<HashMap<String, Value>>,
) -> Result<(), MdsError> {
    let ((), warnings) = check_virtual_collecting_warnings(modules, entry, runtime_vars)?;
    emit_warnings(&warnings);
    Ok(())
}

#[must_use = "warnings should be used"]
pub fn check_virtual_collecting_warnings(
    modules: HashMap<String, String>,
    entry: &str,
    runtime_vars: Option<HashMap<String, Value>>,
) -> Result<((), Vec<String>), MdsError> {
    let vars = runtime_vars.unwrap_or_default();
    let mut cache = ModuleCache::virtual_fs(modules);
    let mut warnings = vec![];
    cache.resolve_key(entry, &vars, &mut warnings)?;
    Ok(((), warnings))
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**ModuleCtx::file_str now holds a borrowed `&str` from `key_display_name` which returns `&str`, but resolve_source constructs it differently** - `crates/mds-core/src/resolver.rs:156-161` vs `crates/mds-core/src/resolver.rs:258-263`
**Confidence**: 80%
- Problem: In `resolve_by_key`, `file_str` is set to `key_display_name(key)` which returns a `&str` borrowed from `key` (a reference into the source string). In `resolve_source`, `file_str` is set to the string literal `"<source>"`. This is functionally fine, but a subtle lifetime inconsistency: in one path, `file_str` borrows from the key parameter; in the other, it borrows from a static string. More importantly, `key_display_name` extracts just the filename portion, so error messages from imports within a module resolved via `resolve_by_key` will show only the filename (e.g. `"main.mds"`) rather than the full canonical path. The original code used `canonical.display().to_string()` as `file_str`, giving full paths in error messages. This changes error message content, which could affect users debugging import chains.
- Fix: Consider using the full key as `file_str` for error messages rather than the short display name, since the original code used full paths. The `key_display_name` function is better suited for cycle string display than for `file_str`:
```rust
let ctx = ModuleCtx {
    file_str: key,  // full key for precise error messages
    source: &source,
    base_key: key,
    runtime_vars,
};
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`ModuleCache` does not derive `Debug`** - `crates/mds-core/src/resolver.rs:45` (Confidence: 65%) -- `ResolvedModule` derives `Debug, Clone` but `ModuleCache` derives neither. Since `ModuleCache` is now public, adding `Debug` would follow the convention of other public types in the crate. However, `Box<dyn FileSystem>` makes deriving `Debug` non-trivial, so this may be intentional.

- **`fs` module is `pub(crate)` but its types are `pub use`'d from lib.rs** - `crates/mds-core/src/lib.rs:42,51` (Confidence: 60%) -- The module itself is `pub(crate)` while its types are re-exported as `pub`. This is a valid Rust pattern but differs from how `resolver` module exposes `ModuleCache` (also `pub(crate)` module with `pub use` re-export), so it is at least consistent within this PR. No action needed.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR introduces a well-structured `FileSystem` trait abstraction with generally good consistency. The `PathBuf` to `String` key migration is cleanly applied throughout the resolver. The new `compile_virtual` / `compile_virtual_collecting_warnings` functions follow the established two-tier warning API pattern. The major consistency concerns are: (1) the `NativeFs::read` metadata pre-check deviates from the project's documented "read first, check size after" TOCTOU-safe pattern, (2) `NativeFs::normalize` missing the empty-path guard that `VirtualFs` has and that the trait's own security contract requires, and (3) the missing `check_virtual` counterparts that break the compile/check API symmetry.

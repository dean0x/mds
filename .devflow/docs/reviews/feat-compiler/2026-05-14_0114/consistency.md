# Consistency Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent error variant for oversized file in `load_vars_file`** - `src/lib.rs:344`
**Confidence**: 95%
- Problem: When a vars file exceeds `MAX_FILE_SIZE`, `load_vars_file` returns `MdsError::import_error(...)` but the analogous check in `resolver.rs:126` returns `MdsError::resource_limit(...)`. These represent the same semantic condition (file too large) but produce different error codes (`mds::import` vs `mds::resource_limit`), which will confuse consumers matching on error variants or diagnostic codes.
- Fix: Use `MdsError::resource_limit` in `load_vars_file` to match the resolver pattern:
```rust
if bytes.len() as u64 > resolver::MAX_FILE_SIZE {
    return Err(MdsError::resource_limit(format!(
        "vars file exceeds maximum size of {} bytes: {}",
        resolver::MAX_FILE_SIZE,
        path.display()
    )));
}
```

### MEDIUM

**Missing convenience constructors for `Io`, `YamlError`, `JsonError`, `NotMdsFile` variants** - `src/error.rs:134-150`
**Confidence**: 85%
- Problem: The codebase establishes a clear constructor pattern: every error variant with a `span`/`src` pair gets both a `foo()` and `foo_at()` constructor on `MdsError`. However, the simpler variants (`Io`, `YamlError`, `JsonError`, `NotMdsFile`) are constructed via direct struct literals at 14 call sites across `resolver.rs`, `value.rs`, and `lib.rs`. This breaks the established pattern where callers use `MdsError::resource_limit(...)` style constructors rather than `MdsError::ResourceLimit { message: ... }` struct literals.
- Fix: Add convenience constructors for consistency:
```rust
pub fn io(message: impl Into<String>) -> Self {
    MdsError::Io { message: message.into() }
}

pub fn yaml_error(message: impl Into<String>) -> Self {
    MdsError::YamlError { message: message.into() }
}

pub fn json_error(message: impl Into<String>) -> Self {
    MdsError::JsonError { message: message.into() }
}

pub fn not_mds_file(path: impl Into<String>) -> Self {
    MdsError::NotMdsFile { path: path.into() }
}
```
Then update all call sites to use `MdsError::io(format!(...))` instead of `MdsError::Io { message: format!(...) }`.

**Inconsistent `Value` type path in integration tests** - `tests/integration.rs:2050-2054`
**Confidence**: 82%
- Problem: The public API re-exports `Value` at the crate root (`pub use value::Value;`), making `mds::Value` the canonical path. Most test code (7 sites) uses the module-qualified path `mds::value::Value`, but 3 sites at lines 2050-2054 use the re-exported `mds::Value`. Both work, but mixing them in the same file is inconsistent.
- Fix: Standardize on `mds::Value` throughout the integration tests since it is the intended public API surface. Add `use mds::Value;` at the top and use unqualified `Value` everywhere, or at minimum use `mds::Value` consistently.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**API asymmetry: `check` family lacks `_collecting_warnings` variants** - `src/lib.rs`
**Confidence**: 82%
- Problem: The `compile` family provides both convenience (`compile`, `compile_str`, `compile_str_with`) and caller-controlled (`compile_collecting_warnings`, `compile_str_collecting_warnings`) variants -- the two-tier API documented in the feature knowledge. The `check` family only provides convenience variants (`check`, `check_str`, `check_str_with`) with no `check_collecting_warnings` or `check_str_collecting_warnings`. This means callers who want to validate without printing to stderr (e.g., IDE integrations, test harnesses) have no supported way to capture warnings from `check`.
- Fix: Add `check_collecting_warnings` and `check_str_collecting_warnings` to complete the two-tier API pattern:
```rust
pub fn check_collecting_warnings(
    path: impl AsRef<Path>,
    runtime_vars: Option<HashMap<String, Value>>,
) -> Result<((), Vec<String>), MdsError> {
    let path = path.as_ref();
    let vars = runtime_vars.unwrap_or_default();
    let mut cache = ModuleCache::new();
    let mut warnings = vec![];
    cache.resolve(path, &vars, &mut warnings)?;
    Ok(((), warnings))
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Stray analysis file committed to repository** - `autobeat-orchestrator-analysis.md` (Confidence: 72%) -- A 191-line analysis report is committed at the repo root. This looks like a review artifact that should live in `.docs/` or be excluded from the branch.

- **`parse_export_directive` ignores `_offset` parameter** - `src/parser.rs:411` (Confidence: 65%) -- The function signature accepts `_offset: usize` but never uses it, while `parse_import_directive` does thread `offset` into its AST nodes. The `ExportDirective` variants (`Named`, `Wildcard`) lack `offset` fields, so span-aware error reporting for exports is structurally impossible. This may be intentional for v0.1 but is asymmetric with imports.

- **`collect_all` flattening may not preserve correct shadowing with `HashMap::collect`** - `src/scope.rs:146-154` (Confidence: 60%) -- The `flat_map` + `collect` pattern on `HashMap` relies on later entries overwriting earlier ones with the same key. This works correctly with the standard `HashMap::collect` implementation but is not formally guaranteed by the Rust standard library documentation to resolve duplicates by keeping the last entry. In practice this is fine, but a fold-based approach would make the shadowing semantics explicit.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates strong internal consistency: the `MdsError` constructor pattern (`foo`/`foo_at`), the two-tier API (`compile`/`compile_collecting_warnings`), the warning collection via `&mut Vec<String>`, and the `Result` return discipline are all well-established and followed almost everywhere. The one blocking HIGH issue (wrong error variant for oversized vars file) is a straightforward one-line fix. The MEDIUM items are pattern completeness gaps that would be good to address before this becomes a stable API surface.

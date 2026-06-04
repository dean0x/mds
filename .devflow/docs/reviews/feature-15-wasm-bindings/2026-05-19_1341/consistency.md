# Consistency Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### HIGH

**Cargo.toml missing workspace fields vs sibling crates** - `crates/mds-wasm/Cargo.toml`
**Confidence**: 92%
- Problem: Both `mds-core/Cargo.toml` and `mds-cli/Cargo.toml` include `rust-version.workspace = true`, `readme.workspace = true`, and `keywords.workspace = true`. The new `mds-wasm/Cargo.toml` omits all three. This is a pattern deviation from the established crate manifest convention.
- Fix: Add the missing workspace fields to `crates/mds-wasm/Cargo.toml`:
```toml
[package]
name = "mds-wasm"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
description = "MDS compiler WebAssembly bindings"
license.workspace = true
readme.workspace = true
repository.workspace = true
keywords.workspace = true
```

**Error code naming inconsistency: underscores vs snake_case** - `crates/mds-wasm/src/lib.rs` (lines 146, 286)
**Confidence**: 90%
- Problem: mds-core diagnostic codes use snake_case consistently but with a specific convention: multi-word codes use underscores *within* the mds namespace (e.g. `mds::undefined_var`, `mds::file_not_found`, `mds::circular_import`, `mds::type_error`, `mds::name_collision`, `mds::resource_limit`). The new WASM crate introduces `mds::invalid_options` and `mds::filename_collision`. While `mds::invalid_options` follows the pattern, `mds::filename_collision` is fine but is a *new* code that exists only in the WASM boundary layer, not registered in `MdsError`. This means errors from `mds-wasm` have codes that don't correspond to any `MdsError` variant, creating a two-tier error code system. Similarly, `mds::internal` is a WASM-only code.
- Fix: This is acceptable for WASM-boundary errors that have no core equivalent, but document the distinction. Consider adding a comment at the top of `lib.rs` near the error helpers listing the WASM-only codes:
```rust
// WASM-only error codes (not in MdsError):
// - mds::internal         — unrecoverable panic caught at WASM boundary
// - mds::invalid_options  — malformed JS options object
// - mds::filename_collision — source filename conflicts with modules key
```

**`to_js` serialization error missing `code` property** - `crates/mds-wasm/src/lib.rs:308-315`
**Confidence**: 85%
- Problem: Every other error construction site in `mds-wasm` sets a `code` property on the JS Error object. The `to_js` function at line 312 creates an error with only a `message` but no `code` property. This breaks the contract documented in the `compile` and `check` doc comments which state errors always have a `code` property.
- Fix: Add a `code` property to the serialization error:
```rust
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| {
            let js_err = js_sys::Error::new(&format!("failed to serialize result: {e}"));
            let _ = Reflect::set(
                &js_err,
                &JsValue::from_str("code"),
                &JsValue::from_str("mds::internal"),
            );
            JsValue::from(js_err)
        })
}
```

### MEDIUM

**Removed `.memory/` and `.docs/` from `.gitignore`** - `.gitignore`
**Confidence**: 85%
- Problem: The original `.gitignore` contained entries for `.memory/` and `.docs/`. This PR removed both while adding `crates/mds-wasm/pkg/`. Removing ignore rules is a silent change that could cause previously-ignored files to appear in future `git status` output and accidentally get committed.
- Fix: Restore the removed entries and add the new one:
```
.memory/
.docs/
/target
crates/mds-wasm/pkg/
```

**`Value::from_json` promoted to `pub` without `#[must_use]`** - `crates/mds-core/src/value.rs:101`
**Confidence**: 82%
- Problem: In mds-core, all public methods on `Value` that return meaningful values carry `#[must_use]`: `is_truthy()`, `as_array()`, `type_name()`. The `from_yaml` counterpart is `pub(crate)` so it doesn't need the attribute, but `from_json` is now `pub` and returns `Result<Value, MdsError>` -- it should follow the same pattern as the other public methods for consistency.
- Fix:
```rust
/// Convert a serde_json::Value into our Value enum.
#[must_use = "the converted value should be used"]
pub fn from_json(json: serde_json::Value) -> Result<Value, MdsError> {
```

## Issues in Code You Touched (Should Fix)

*No issues found.*

## Pre-existing Issues (Not Blocking)

*No issues found.*

## Suggestions (Lower Confidence)

- **Repeated JS error construction boilerplate** - `crates/mds-wasm/src/lib.rs` (16 occurrences of `Reflect::set` for `code` property) (Confidence: 70%) -- The pattern of creating a `js_sys::Error`, then calling `Reflect::set` for `code` is repeated verbatim throughout `parse_options` and `build_modules`. A small helper like `fn js_error(msg: &str, code: &str) -> JsValue` would reduce boilerplate and ensure all WASM-boundary errors consistently set `code`. This is a style preference rather than a correctness issue.

- **`json_type_name` in mds-wasm duplicates `Value::type_name` pattern from mds-core** - `crates/mds-wasm/src/lib.rs:259-268` (Confidence: 65%) -- `value.rs` in mds-core has both `yaml_type_name` and `Value::type_name` for similar purposes. The WASM crate adds a third `json_type_name` function. The type name sets are nearly identical (both return "null", "boolean", "number", "string", "array", "object"). While they operate on different enum types (`serde_json::Value` vs `Value`), this is worth noting as a pattern that could eventually be unified.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 3 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The new `mds-wasm` crate is well-structured and generally follows existing codebase patterns (section comment style, snake_case naming, `Result` return types, doc comment conventions, workspace dependency references). The main consistency gaps are: (1) missing workspace metadata fields in `Cargo.toml` that all sibling crates include, (2) a broken error contract where `to_js` serialization failures don't carry the `code` property that all other errors do, (3) the `.gitignore` regression removing existing ignore entries, and (4) the newly-public `Value::from_json` missing `#[must_use]` that all other public `Value` methods carry.

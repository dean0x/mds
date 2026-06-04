# Reliability Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19
**Scope**: Incremental review of resolution commits (420e2259...HEAD)

## Issues in Your Changes (BLOCKING)

### MEDIUM

**No size guard on aggregate `options.modules` payload** - `crates/mds-wasm/src/lib.rs:190-216`
**Confidence**: 82%
- Problem: `check_source_size` guards the main `source` argument (line 370/407), and the `VirtualFs::read` method in mds-core guards individual module reads against `MAX_FILE_SIZE`. However, `parse_modules` iterates over all entries in `options.modules` without checking the total count or aggregate byte size of module values. A caller could pass thousands of small modules (each under 10 MiB) that collectively exhaust WASM linear memory during `HashMap` construction. The `MAX_IMPORT_DEPTH` (64) in the resolver limits how many get _resolved_, but all modules are allocated into the HashMap before resolution begins.
- Impact: A malicious or careless JS caller could trigger an OOM abort in the WASM module by passing a very large `modules` object. In practice the 4 GB WASM memory ceiling and serde deserialization cost provide an implicit ceiling, but the failure mode is an uncontrolled abort rather than a structured error.
- Fix: Add a bound on the number of modules and/or aggregate size in `parse_modules`:
```rust
const MAX_MODULE_COUNT: usize = 256;

fn parse_modules(
    map: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<HashMap<String, String>, JsValue> {
    match map.remove("modules") {
        Some(serde_json::Value::Object(mods)) => {
            if mods.len() > MAX_MODULE_COUNT {
                return Err(options_error(&format!(
                    "options.modules contains {} entries, maximum is {}",
                    mods.len(), MAX_MODULE_COUNT
                )));
            }
            // ... existing parsing ...
        }
        // ...
    }
}
```

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

_No issues found._

## Suggestions (Lower Confidence)

- **`set_prop` silently swallows failures in release builds** - `crates/mds-wasm/src/lib.rs:60-64` (Confidence: 65%) -- The `debug_assert!` means that if `Reflect::set` ever fails in production (e.g., due to a browser engine quirk), error properties like `code`, `help`, and `span` would silently be missing from the JS error object. The doc comment explains the rationale (only fails on frozen/non-extensible objects), but a `log::warn!` or even a no-op would make the failure mode explicit. Low risk since all targets are freshly-created objects.

- **`serde_wasm_bindgen::from_value` deserializes full options before any field-level size checks** - `crates/mds-wasm/src/lib.rs:259` (Confidence: 62%) -- The entire JS options object is deserialized into `serde_json::Value` before `parse_modules` runs. If a caller passes an extremely large options object with deeply nested or enormous string values in unexpected fields, the allocation happens before any application-level guard. serde_json's default recursion limit (128) and WASM memory ceiling provide implicit bounds, making this low practical risk.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Observations

The incremental changes demonstrate strong reliability practices:

1. **`MAX_SOURCE_SIZE` at the WASM boundary** (line 51) -- Re-enforces mds-core's `MAX_FILE_SIZE` at the trust boundary where the file layer is bypassed. This is the correct defensive pattern.
2. **`check_source_size` called before allocation** (lines 370, 407) -- Both `compile` and `check` validate input size _before_ cloning the source string, preventing wasteful allocation of oversized inputs.
3. **`debug_assert!` in `set_prop`** (line 63) -- Catches `Reflect::set` failures during development without release overhead. Correctly documents the invariant.
4. **Sanitized panic messages in `catch_panic`** (lines 140-152) -- Generic public message with raw payload in `detail` only. Prevents leaking internal paths while preserving debuggability.
5. **`load_vars_str` size guard** (mds-core line 760-765) -- Bounds the JSON parsing input, consistent with `load_vars_file`.
6. **Bounded recursion in `Value::from_json`** -- `MAX_VALUE_DEPTH = 64` prevents stack overflow from deeply nested vars.
7. **`MAX_IMPORT_DEPTH = 64`** in the resolver -- Prevents deep import chains from exhausting the stack.

The one MEDIUM finding (unbounded module count) is a hardening gap rather than a likely production failure, given the WASM memory ceiling. The condition for APPROVED: consider adding a module count guard in a follow-up.

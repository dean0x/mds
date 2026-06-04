# Rust Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**No size validation on `options.modules` values at the WASM boundary** - `crates/mds-wasm/src/lib.rs:190-216`
**Confidence**: 82%
- Problem: `check_source_size` validates the primary `source` argument against `MAX_SOURCE_SIZE`, and `mds-core` validates files read from disk via `MAX_FILE_SIZE`. However, modules passed through `options.modules` in the WASM path bypass both checks. A caller can supply arbitrarily large strings as module values in the `modules` record, which are inserted directly into the `HashMap<String, String>` and forwarded to `compile_virtual_with_deps`. Since the VirtualFs resolver in mds-core does not enforce per-module size limits for in-memory modules, this allows memory exhaustion at the WASM boundary. The `source` check at line 370/407 only guards the entry module, not the imported modules.
- Fix: Add a size check inside `parse_modules` for each module value:
```rust
fn parse_modules(
    map: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<HashMap<String, String>, JsValue> {
    match map.remove("modules") {
        Some(serde_json::Value::Object(mods)) => {
            let mut result = HashMap::with_capacity(mods.len());
            for (key, val) in mods {
                match val {
                    serde_json::Value::String(s) => {
                        if s.len() > MAX_SOURCE_SIZE {
                            return Err(options_error(&format!(
                                "options.modules[\"{key}\"] exceeds maximum size of {MAX_SOURCE_SIZE} bytes ({} bytes provided)",
                                s.len()
                            )));
                        }
                        result.insert(key, s);
                    }
                    // ... existing error arm
                }
            }
            Ok(result)
        }
        // ... existing arms
    }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`usize as f64` cast in `span_to_js` can lose precision on non-WASM targets** - `crates/mds-wasm/src/lib.rs:113-114`
**Confidence**: 60%
- Problem: `span.offset as f64` and `span.length as f64` cast `usize` to `f64`. On `wasm32` targets `usize` is 32-bit, so all values are exactly representable in `f64` (which has 53 bits of mantissa). However, the crate also builds with `crate-type = ["rlib"]`, meaning it can be compiled and tested on 64-bit native targets where `usize` is 64-bit. For very large files (> 2^53 bytes), precision would be lost. In practice, the `MAX_FILE_SIZE` limit of 10 MiB ensures values never approach this threshold on any target, so this is theoretical.
- Note: Moved to Suggestions due to confidence below 80%.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`usize as f64` cast safety** - `crates/mds-wasm/src/lib.rs:113-114` (Confidence: 60%) -- The casts are safe given the 10 MiB limit, but a brief inline comment noting why (e.g., `// Safe: MAX_SOURCE_SIZE << 2^53`) would document the invariant for future maintainers.

- **`unwrap_or(false)` in `set_prop` silently swallows errors in release builds** - `crates/mds-wasm/src/lib.rs:61-63` (Confidence: 65%) -- `Reflect::set` returns `Err` only for frozen/sealed objects which this code never passes, and the `debug_assert!` catches mistakes during development. However, in release builds a silent failure means a JS error object could be missing properties (e.g., `code`, `help`) without any indication. The doc comment correctly explains the rationale, so this is a design tradeoff rather than a bug.

- **No validation of total aggregate module size** - `crates/mds-wasm/src/lib.rs:289-306` (Confidence: 62%) -- Even with per-module size limits, a caller could pass many modules each just under the limit. A total aggregate cap (e.g., sum of all module sizes) would provide defense in depth, but may be over-engineering for the current use case.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The code demonstrates strong Rust idioms throughout:
- Zero `.unwrap()` calls in library code -- all error propagation uses `?` or explicit handling
- No `unsafe` blocks
- Ownership is well-managed: the refactoring from `map.get()` + `.clone()` to `map.remove()` eliminates unnecessary allocations (lines 171, 193, 225)
- The `set_prop` + `debug_assert!` pattern is a clean tradeoff between debug-time safety and release performance
- `MAX_SOURCE_SIZE` is correctly derived from `mds::MAX_FILE_SIZE` rather than duplicating the constant
- The function decomposition (`parse_filename`, `parse_modules`, `parse_vars`) follows single-responsibility well
- `catch_panic` correctly avoids leaking internal details in the public error message while preserving them in `detail`
- Panic payload is handled exhaustively (`&str`, `String`, and the catch-all)

The single blocking MEDIUM issue is the missing size validation on module values passed through `options.modules`, which creates an asymmetry with the source input guard. The condition for approval is addressing this gap.

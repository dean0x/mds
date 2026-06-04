# Performance Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Double serialization in options parsing** - `crates/mds-wasm/src/lib.rs:141`
**Confidence**: 85%
- Problem: `parse_options` deserializes the entire JS options object into `serde_json::Value` via `serde_wasm_bindgen::from_value(options)` (line 141), which traverses and copies every field into an intermediate `serde_json::Value` tree. Fields like `modules` and `vars` are then cloned again (`.clone()` on line 163, 197, 235) when extracted from the borrowed `&opts_json`. For typical small options objects this is negligible, but for large `modules` maps (many virtual files with large source content), this doubles memory usage and allocation cost at the WASM boundary.
- Fix: Consider using `serde_wasm_bindgen::from_value` directly into a typed Rust struct (e.g., `#[derive(Deserialize)] struct RawOptions { filename: Option<String>, modules: Option<HashMap<String,String>>, vars: Option<serde_json::Value> }`) so the data is moved once rather than cloned from a borrowed intermediate. Alternatively, keep the current approach but destructure `opts_json` with `into_iter()` / pattern matching to move values out instead of cloning:
```rust
let serde_json::Value::Object(mut map) = opts_json else { ... };
let filename = match map.remove("filename") {
    Some(serde_json::Value::String(s)) => s,
    None => "input.mds".to_string(),
    ...
};
```
  Using `map.remove()` instead of `map.get()` avoids cloning strings.

**`val.clone()` in vars iteration forces redundant deep-copy** - `crates/mds-wasm/src/lib.rs:235`
**Confidence**: 82%
- Problem: Each var value is cloned (`val.clone()`) before being passed to `Value::from_json()`. Since `from_json` consumes the `serde_json::Value` by value, this clone is necessary only because `vars_map` is borrowed from the outer `opts_json`. If the options map were destructured by ownership (as suggested above), the clone would be eliminated. For large nested variable objects this produces a full deep copy of the JSON tree.
- Fix: Addressed by the same fix above -- destructure with `map.remove("vars")` to take ownership, then iterate the owned map.

### LOW

**`wasm-opt = false` disables Binaryen optimization** - `crates/mds-wasm/Cargo.toml:24`
**Confidence**: 85%
- Problem: The release metadata sets `wasm-opt = false`, which disables the Binaryen optimizer pass. Binaryen typically reduces WASM binary size by 10-20% and can improve runtime performance through dead code elimination and instruction folding. The binary is reported at ~455 KB; with wasm-opt enabled it would likely be ~370-410 KB.
- Fix: Enable wasm-opt for release builds:
```toml
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz"]
```
  If wasm-opt was disabled due to a specific compatibility issue, document the reason in a comment. The 500 KB budget is met either way, but smaller binaries improve download time for JS consumers.

**`panic = "unwind"` set globally affects CLI crate binary size** - `Cargo.toml:30-34`
**Confidence**: 80%
- Problem: `panic = "unwind"` is set at the workspace level for both `[profile.dev]` and `[profile.release]`. While this is required for `catch_unwind` in the WASM crate, it also applies to `mds-cli`, which does not use `catch_unwind`. The `abort` panic strategy produces smaller binaries (typically 5-10% reduction) and slightly faster code because the compiler can skip generating unwind tables. Setting `panic = "unwind"` globally sacrifices this for the CLI binary.
- Fix: Consider using per-package profile overrides to limit `panic = "unwind"` to only `mds-wasm`:
```toml
# Workspace root Cargo.toml
[profile.release]
lto = true
# panic defaults to "unwind" anyway, but you could set "abort" here
# for the CLI benefit, then override per-package:

[profile.release.package.mds-wasm]
opt-level = "z"
strip = true
codegen-units = 1
panic = "unwind"
```
  Note: Cargo currently inherits workspace panic strategy for dependencies. Verify that per-package `panic` override works correctly in your Rust toolchain version before applying.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Serializer reuse across calls** - `crates/mds-wasm/src/lib.rs:310` (Confidence: 65%) -- `serde_wasm_bindgen::Serializer::json_compatible()` is created fresh on every call to `to_js`. If this allocates internal buffers, a `thread_local!` cached serializer could save micro-allocations in hot-loop scenarios (e.g., repeated compile calls in a WASM worker). Likely negligible for single calls.

- **`source.to_string()` copies input on every call** - `crates/mds-wasm/src/lib.rs:347,382` (Confidence: 62%) -- Both `compile` and `check` clone the source `&str` into an owned `String` for `UnwindSafe` compliance. This is a necessary trade-off for panic safety. If profiling shows this copy is significant for very large templates, an alternative design could use a `Mutex<String>` or restructure the catch_unwind boundary. Currently correct and likely not a bottleneck.

- **`HashMap::new()` without capacity hint for default modules** - `crates/mds-wasm/src/lib.rs:135,215` (Confidence: 60%) -- When options are null/undefined or modules are missing, `HashMap::new()` creates zero-capacity maps. These will reallocate on the first insert in `build_modules`. Using `HashMap::with_capacity(1)` for the default path (since at minimum the source is inserted) would avoid one reallocation. Micro-optimization.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 2 |
| Should Fix | - | 0 | 0 | 0 |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The WASM bindings are well-structured with appropriate size optimization (`opt-level = "z"`, `strip`, `codegen-units = 1`, LTO). The ~455 KB binary is within the 500 KB budget. The main performance concern is the double-serialization and redundant cloning in `parse_options`, which is a MEDIUM issue because it primarily affects memory efficiency for callers passing large `modules` or `vars` maps. For typical use (small option objects), the overhead is negligible. The `wasm-opt = false` and global `panic = "unwind"` settings are LOW-priority items that leave modest optimization on the table. Overall, this is a solid WASM binding layer with no critical or high-severity performance issues.

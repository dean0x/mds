# Performance Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20T19:15

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Options deserialization traverses the JS object twice** - `crates/mds-napi/src/lib.rs:310`, `crates/mds-napi/src/lib.rs:367`
**Confidence**: 82%
- Problem: `parse_compile_opts` and `parse_file_opts` call `env.from_js_value(opts_obj)` which traverses the entire JS options object to build a `serde_json::Value` intermediate tree. Then `parse_vars_field` iterates the `vars` sub-object again to convert each `serde_json::Value` into `mds::Value` via `Value::from_json`. This means every variable value is visited twice: once for JS-to-JSON, once for JSON-to-MDS. For typical small options objects (a handful of vars), this is negligible. For users passing large `vars` maps (hundreds of deeply nested values), the double traversal and intermediate `serde_json::Value` allocations add measurable overhead.
- Fix: The current approach is acceptable for the typical use case (small options). A future optimization could use napi's native `Object::get_named_property` to extract `basePath` and `vars` directly, then convert only the `vars` values via a custom JS-to-`mds::Value` walker that skips the `serde_json` intermediate. This would halve allocations for the vars path. Not blocking since the options object is typically small.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing `codegen-units = 1` in release profile for mds-napi** - `Cargo.toml:49-51`
**Confidence**: 88%
- Problem: The `[profile.release.package.mds-napi]` section sets `opt-level = 3` and `strip = true` but omits `codegen-units = 1`. The sibling `mds-wasm` package sets `codegen-units = 1`, which enables better cross-module inlining and dead code elimination during LTO (already enabled workspace-wide). For a native addon where call overhead matters (every JS-to-Rust transition), maximizing inlining is worthwhile.
- Fix: Add `codegen-units = 1` to the mds-napi release profile:
  ```toml
  [profile.release.package.mds-napi]
  opt-level = 3
  strip = true
  codegen-units = 1
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing performance issues found.

## Suggestions (Lower Confidence)

- **`source: String` parameter forces napi-rs to copy the JS string** - `crates/mds-napi/src/lib.rs:420` (Confidence: 65%) -- napi-rs copies JS strings into owned Rust `String`s when the parameter type is `String`. Since `compile_str_with_deps` takes `&str`, the owned string is only needed for the `move` closure passed to `catch_unwind`. If `catch_unwind` were replaced with a non-capturing pattern (or if the core API accepted owned strings), one copy could be eliminated. However, napi-rs v3 does not currently expose zero-copy string references for `#[napi]` functions, so there is no actionable fix today.

- **`catch_unwind` landing pad has negligible cost on the happy path** - `crates/mds-napi/src/lib.rs:215` (Confidence: 70%) -- `catch_unwind` is essentially free when no panic occurs (the compiler inserts landing pad metadata, not runtime checks). The `AssertUnwindSafe` wrapper is a ZST. No action needed -- this is noted for completeness since the PR description flagged it as a concern area.

- **`format!` allocations in `check_source_size` error path** - `crates/mds-napi/src/lib.rs:248` (Confidence: 60%) -- The `format!` call allocates a string on every resource-limit error. Since this is an error-only path (not hot), the allocation is acceptable. Mentioning only because it was within the PR's stated focus areas.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

The binding layer is well-designed from a performance perspective:

1. **Minimal overhead beyond delegation** -- All four exported functions (`compile`, `compileFile`, `check`, `checkFile`) are thin wrappers that parse options, validate input, call the core API, and return results. No unnecessary computation is added.

2. **Correct API selection** -- `check` and `checkFile` use `check_str_collecting_warnings` and `check_collecting_warnings` respectively (not compile variants), avoiding unnecessary output rendering.

3. **Move semantics, not copies** -- `CompileOutput` fields (`output`, `warnings`, `dependencies`) are moved into `CompileResult`, not cloned. The struct decomposition at lines 431-435 and 465-469 is zero-cost.

4. **Appropriate resource limits** -- `MAX_SOURCE_SIZE` check at the napi boundary prevents oversized strings from reaching the core, where they would cause larger allocations before failing.

5. **HashMap pre-sizing** -- `parse_vars_field` uses `HashMap::with_capacity(vars_map.len())` at line 279, avoiding rehashing.

6. **Release profile** -- `opt-level = 3` is correct for a native addon (favoring speed over size, unlike the wasm target's `opt-level = "z"`). The workspace-wide `lto = true` ensures cross-crate inlining.

The only actionable item is adding `codegen-units = 1` to the release profile for full LTO effectiveness. The options deserialization double-traversal is a known trade-off for validation ergonomics and is acceptable for the typical use case.

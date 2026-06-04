# Reliability Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Silently discarded `Reflect::set` results may mask JS interop failures (16 occurrences)** - Confidence: 82%
- `crates/mds-wasm/src/lib.rs:45`, `:49`, `:55`, `:60`, `:66`, `:72`, `:79`, `:106`, `:143`, `:170`, `:204`, `:221`, `:246`, `:283`
- Problem: Every `Reflect::set` call discards its `Result` with `let _ = ...`. While `Reflect::set` on a freshly-created `js_sys::Error` or `js_sys::Object` is extremely unlikely to fail at runtime, silently discarding all 16 results means a JS engine edge case (frozen prototype, non-configurable property, exotic object) would produce an error object missing its `code`, `help`, or `span` properties with no diagnostic trace. The consumer would see a bare `Error` with only a `message`, making debugging difficult.
- Fix: This is low-risk in practice because the target objects are newly created and plain. However, for defense-in-depth, consider a helper that logs or debug-asserts on failure:
  ```rust
  /// Set a property on a JS object. Debug-asserts success in dev builds.
  fn set_prop(target: &JsValue, key: &str, value: &JsValue) {
      let result = Reflect::set(target, &JsValue::from_str(key), value);
      debug_assert!(result.is_ok(), "Reflect::set failed for key: {key}");
  }
  ```
  This would catch regressions in development while keeping zero overhead in release builds.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`load_vars_str` has no input size limit, unlike its sibling `load_vars_file`** - `crates/mds-core/src/lib.rs:759`
**Confidence**: 85%
- Problem: `load_vars_file` (line 711) enforces `MAX_FILE_SIZE` (10 MB) before parsing JSON. The new `load_vars_str` function (line 759) parses the JSON string directly via `serde_json::from_str` with no size check. A caller passing an arbitrarily large string could cause unbounded memory allocation during deserialization. While the WASM boundary itself does not call `load_vars_str` directly (it uses `serde_wasm_bindgen::from_value` for options parsing), `load_vars_str` is a new public API that other consumers will use. The asymmetry with `load_vars_file` is a reliability gap.
- Fix: Add a size guard consistent with the file-based sibling:
  ```rust
  pub fn load_vars_str(json: &str) -> Result<HashMap<String, Value>, MdsError> {
      if json.len() as u64 > MAX_FILE_SIZE {
          return Err(MdsError::resource_limit(format!(
              "vars JSON string exceeds maximum size of {MAX_FILE_SIZE} bytes"
          )));
      }
      let parsed: serde_json::Value =
          serde_json::from_str(json).map_err(|e| MdsError::json_error(e.to_string()))?;
      // ...
  }
  ```

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing reliability issues found.

## Suggestions (Lower Confidence)

- **No module count limit on `options.modules`** - `crates/mds-wasm/src/lib.rs:191` (Confidence: 65%) -- The `parse_options` function accepts an unbounded number of modules from the JS caller. While `VirtualFs` enforces per-file size limits and the import resolver has a `MAX_IMPORT_DEPTH` of 64, there is no explicit cap on the total number of virtual modules passed in. A malicious or buggy JS caller could pass thousands of module entries, causing large HashMap allocations. In practice, the WASM linear memory limit provides a natural ceiling, and this is a library called by trusted JS code, so the risk is low.

- **`panic = "unwind"` applied workspace-wide widens blast radius beyond mds-wasm** - `Cargo.toml:30,34` (Confidence: 70%) -- Setting `panic = "unwind"` at the workspace level (both dev and release profiles) means mds-cli and mds-core also use unwind rather than abort. The `catch_unwind` mechanism is only used by mds-wasm. For mds-cli, `panic = "abort"` would produce smaller binaries and faster panics. This is a minor optimization concern rather than a reliability defect, and a per-crate override for mds-cli could be added later.

- **No explicit `source` emptiness check at the WASM boundary** - `crates/mds-wasm/src/lib.rs:345,380` (Confidence: 60%) -- The `compile` and `check` functions accept an empty `source` string without validation. The underlying mds-core functions likely handle this gracefully, but an explicit precondition assertion (e.g., documenting or testing the empty-source behavior) would improve confidence in the boundary contract.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The WASM boundary layer demonstrates strong reliability practices overall:
- Panic catching via `catch_unwind` prevents WASM module abort on internal panics
- `AssertUnwindSafe` usage is correctly documented and justified (cloned owned data)
- No `.unwrap()` or `.expect()` in production code -- all error paths return `Result`
- The underlying `VirtualFs` enforces file size limits (10 MB), path segment limits (256), value nesting depth limits (64), and import depth limits (64)
- Loops in `parse_options` iterate over bounded deserialized maps (finite JSON objects)
- Pre-sized allocations via `HashMap::with_capacity` in hot paths

The single conditional item is adding a size guard to `load_vars_str` for parity with `load_vars_file`. The `Reflect::set` discarding is a minor defense-in-depth gap but not blocking given the controlled object creation pattern.

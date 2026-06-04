# Security Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19T14:04
**Scope**: Incremental review of 7 resolution commits (420e2259...HEAD)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Panic payload leaks internal paths via `detail` property** - `crates/mds-wasm/src/lib.rs:145-151`
**Confidence**: 85%
- Problem: The `catch_panic` function attaches the raw panic payload string as a `detail` property on the JS error object. Rust panic messages routinely contain absolute filesystem paths from `unwrap()`, `assert!()`, and `expect()` calls (e.g. `"called Result::unwrap() on an Err value at /home/user/project/crates/mds-core/src/evaluator.rs:87"`). While the main `message` field is correctly sanitized to the generic "internal compiler error", the `detail` field exposes the raw payload to any JavaScript caller. In a WASM context embedded in a web application, this leaks server-side build paths, internal module structure, and potentially sensitive assertion context to untrusted clients.
- Impact: Information disclosure (OWASP A05 — Security Misconfiguration). Build paths and internal structure revealed to untrusted callers. Attackers can use this to map internal project layout for targeted exploitation.
- Fix: Either remove the `detail` property entirely, or gate it behind a debug/development feature flag:
  ```rust
  // Option A: Remove detail entirely (recommended for production WASM)
  fn catch_panic<F, T>(f: F) -> Result<T, JsValue>
  where
      F: std::panic::UnwindSafe + FnOnce() -> Result<T, JsValue>,
  {
      std::panic::catch_unwind(f).unwrap_or_else(|_payload| {
          Err(js_error("internal compiler error", "mds::internal"))
      })
  }

  // Option B: Gate behind a cargo feature
  #[cfg(feature = "debug-panics")]
  set_prop(&err, "detail", &detail);
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**No aggregate size limit on `options.modules` map** - `crates/mds-wasm/src/lib.rs:190-217`
**Confidence**: 80%
- Problem: The `parse_modules` function accepts an unbounded number of module entries from the JS caller. While individual module reads are bounded by `MAX_FILE_SIZE` in `VirtualFs::read()` (line 167 of `fs.rs`), the WASM boundary itself does not limit the total aggregate size of all modules passed via `options.modules`. A caller could pass thousands of modules each just under 10 MiB, causing significant memory allocation within the WASM linear memory before the compiler ever runs. The `source` input is correctly bounded by `check_source_size`, but the modules map is not.
- Impact: Denial of service via memory exhaustion in the WASM linear memory. The WASM module runs in-process with the host JS runtime, so exhausting its memory can crash the entire tab/worker.
- Fix: Add an aggregate size check after parsing modules:
  ```rust
  fn parse_modules(
      map: &mut serde_json::Map<String, serde_json::Value>,
  ) -> Result<HashMap<String, String>, JsValue> {
      match map.remove("modules") {
          Some(serde_json::Value::Object(mods)) => {
              let mut result = HashMap::with_capacity(mods.len());
              let mut total_size: usize = 0;
              for (key, val) in mods {
                  match val {
                      serde_json::Value::String(s) => {
                          total_size = total_size.saturating_add(s.len());
                          if total_size > MAX_SOURCE_SIZE {
                              return Err(js_error(
                                  "total modules size exceeds maximum",
                                  "mds::resource_limit",
                              ));
                          }
                          result.insert(key, s);
                      }
                      // ...existing error handling...
                  }
              }
              Ok(result)
          }
          // ...
      }
  }
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Module count limit** - `crates/mds-wasm/src/lib.rs:194` (Confidence: 65%) -- Even with a size limit, a very large number of small modules could cause HashMap overhead. Consider capping `mods.len()` to a reasonable maximum (e.g. 1000 modules) as an additional defense layer.

- **`serde_wasm_bindgen::from_value` deserialization of untrusted options** - `crates/mds-wasm/src/lib.rs:259` (Confidence: 60%) -- The options object is deserialized from an arbitrary JS value. While serde_wasm_bindgen handles this safely for JSON-like structures, deeply nested or recursive JS objects could cause stack exhaustion during deserialization. The serde_json::Value intermediate representation mitigates this since it limits to JSON-representable types, but worth monitoring if exotic JS objects are passed.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Security Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Positive Security Observations

The incremental changes demonstrate strong security awareness:

1. **Input size limits**: `check_source_size()` is correctly applied before any allocation or processing in both `compile()` and `check()`. The limit derives from `mds::MAX_FILE_SIZE` (single source of truth).
2. **Panic sanitization**: The main error message is correctly generic ("internal compiler error") rather than exposing raw panic text. The `catch_panic` boundary prevents WASM module abort.
3. **Filename collision detection**: `build_modules` correctly rejects modules that would shadow the source entry, preventing a source substitution attack.
4. **Empty filename rejection**: `parse_filename` rejects empty/whitespace-only filenames, preventing potential path confusion.
5. **Type-safe options parsing**: All option fields are validated by type with clear error codes, preventing type confusion attacks.
6. **`load_vars_str` size guard**: The new size check in `mds-core` prevents unbounded allocation from the string-based vars loading path.
7. **VirtualFs isolation**: No OS filesystem access from the WASM boundary -- all compilation uses `VirtualFs`, eliminating path traversal risks.

### Rationale

One HIGH finding (panic detail leakage) warrants a fix before merge. The panic `detail` property exposes raw Rust panic messages that commonly contain filesystem paths and internal assertion context. While the main message is correctly sanitized, the detail field undermines this sanitization. The modules aggregate size limit (MEDIUM) is a defense-in-depth improvement that should be addressed but could reasonably be deferred to a follow-up given that individual module reads are already bounded.

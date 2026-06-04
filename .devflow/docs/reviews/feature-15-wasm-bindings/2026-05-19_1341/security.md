# Security Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19T13:41

## Issues in Your Changes (BLOCKING)

### MEDIUM

**No input size limits at the WASM boundary** - `crates/mds-wasm/src/lib.rs:345,380`
**Confidence**: 82%
- Problem: The `compile()` and `check()` functions accept `source: &str` and `options: JsValue` without enforcing any size limits before processing. While `mds-core`'s `load_vars_file` enforces a 10 MB `MAX_FILE_SIZE` limit for file-based inputs, the WASM boundary bypasses the file layer entirely. A caller could pass an extremely large source string or an options object containing thousands of large module entries, causing excessive memory allocation within the WASM linear memory. In a browser context this could lead to tab crashes or denial of service against the hosting page.
- Fix: Add a size check at the WASM boundary before proceeding with compilation. For example:
  ```rust
  const MAX_SOURCE_SIZE: usize = 10 * 1024 * 1024; // 10 MB, matching MAX_FILE_SIZE
  
  if source.len() > MAX_SOURCE_SIZE {
      let js_err = js_sys::Error::new("source exceeds maximum size of 10 MB");
      let _ = Reflect::set(&js_err, &JsValue::from_str("code"), &JsValue::from_str("mds::resource_limit"));
      return Err(js_err.into());
  }
  ```
  Consider also limiting the total size of modules in `parse_options` or `build_modules`.

**`.gitignore` removes `.memory/` and `.docs/` exclusions** - `.gitignore:1-2`
**Confidence**: 85%
- Problem: The `.gitignore` diff removes the entries `.memory/` and `.docs/` that previously prevented these directories from being tracked. The `.memory/` directory currently exists in the working tree (containing `decisions/` and `knowledge/` subdirectories). If a contributor runs `git add .` or `git add -A`, these directories could be committed to the repository. Depending on their contents, they may contain project-specific notes, internal decision records, or other data not intended for version control.
- Fix: Restore the `.gitignore` entries or add equivalent exclusions:
  ```
  /target
  crates/mds-wasm/pkg/
  .memory/
  .docs/
  .devflow/
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Panic message may leak internal details** - `crates/mds-wasm/src/lib.rs:96-113`
**Confidence**: 80%
- Problem: The `catch_panic` function extracts the panic payload and includes it verbatim in the JS error message (`"internal compiler panic: {s}"`). Panic messages from the Rust compiler internals or from dependencies can contain file paths, assertion details, or internal state descriptions that expose implementation details to JS consumers. While this is a template compiler (not a service handling user auth), the principle of minimal information disclosure still applies -- especially if the WASM module is used in a multi-tenant service context.
- Fix: Consider sanitizing or truncating the panic message, or providing a generic message with a debug-only detail field:
  ```rust
  let js_err = js_sys::Error::new("internal compiler error");
  // Optionally set a detail property for debugging:
  let _ = Reflect::set(&js_err, &JsValue::from_str("detail"), &JsValue::from_str(&msg));
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing CRITICAL security issues were identified in unchanged code.

## Suggestions (Lower Confidence)

- **No limit on number of modules in options** - `crates/mds-wasm/src/lib.rs:191-228` (Confidence: 65%) -- A caller could pass thousands of modules in the `options.modules` object, each potentially large. Consider adding a count or total-size limit on the modules map to prevent resource exhaustion at the WASM boundary.

- **`AssertUnwindSafe` soundness** - `crates/mds-wasm/src/lib.rs:349,384` (Confidence: 60%) -- The closures wrapped in `AssertUnwindSafe` capture `options: JsValue` which is passed directly from JS. The comment claims "callers ensure this is safe by cloning all captured data" but `JsValue` (a JS heap reference) is not cloned -- only `source` is. If a panic occurs mid-execution and the `JsValue` reference is in an inconsistent state, the `AssertUnwindSafe` wrapper silently suppresses this. In practice this is unlikely to cause issues because wasm-bindgen manages the JS reference lifecycle, but the safety comment is slightly misleading.

- **`Reflect::set` return values silently discarded** - `crates/mds-wasm/src/lib.rs:45-79` (Confidence: 62%) -- All `Reflect::set` calls use `let _ =` to discard the result. If a `Reflect::set` fails (e.g., on a frozen object), the error metadata (code, help, span) would be silently dropped and the JS consumer would receive a partially-formed error. This is a minor robustness concern rather than a security vulnerability.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Positive Security Observations

The WASM bindings demonstrate strong security practices overall:

1. **VirtualFs isolation** -- Compilation runs entirely in-memory via `VirtualFs`. The WASM boundary never accesses the OS filesystem. Path traversal attacks are properly guarded (null byte rejection, segment counting, `..` traversal above root rejected).
2. **Panic containment** -- `catch_unwind` prevents panics from aborting the WASM module, converting them to structured JS errors with `mds::internal` code.
3. **Input validation** -- All options fields are type-checked with clear error messages. Filename collisions are detected. Null/undefined options gracefully default.
4. **Depth limits** -- `Value::from_json` enforces a 64-level nesting depth limit, preventing stack overflow from deeply nested JSON input.
5. **No unsafe code** -- The entire `mds-wasm` crate uses safe Rust only.
6. **Release profile** -- `strip = true` removes debug symbols from the WASM binary, limiting information leakage from the compiled artifact.
7. **No hardcoded secrets** -- No credentials, tokens, or API keys found anywhere in the changed files.
8. **Structured error handling** -- Errors are converted to `js_sys::Error` with typed `code` properties rather than exposing raw Rust error types.

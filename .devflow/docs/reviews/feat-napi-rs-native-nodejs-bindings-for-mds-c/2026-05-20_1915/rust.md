# Rust Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing `// SAFETY:` comments on all unsafe code (5 locations)** - `crates/mds-napi/src/lib.rs:85`, `crates/mds-napi/src/lib.rs:112`, `crates/mds-napi/src/lib.rs:127`, `crates/mds-napi/src/lib.rs:150`, `crates/mds-napi/src/lib.rs:197`
**Confidence**: 95%
- Problem: There are 3 `unsafe fn` declarations (lines 85, 112, 127) and 2 `unsafe` blocks (lines 150, 197), none of which have `// SAFETY:` comments documenting why the unsafe usage is sound. Per the Rust API Guidelines and Clippy's `undocumented_unsafe_blocks` lint, every `unsafe` block or function should document its safety invariants. This is especially important here because the code performs raw FFI calls to N-API (`napi_create_string_utf8`, `napi_create_error`, `napi_throw`, etc.) where the caller must uphold specific contracts: valid `napi_env`, correct string pointer/length pairs, and valid `napi_value` handles.
- Fix: Add `// SAFETY:` comments to each unsafe site. Example for `raw_create_error`:

```rust
/// ...existing doc comment...
///
/// # Safety
///
/// - `env` must be a valid `napi_env` handle from an active N-API callback.
/// - The returned `napi_value` (if non-null) is only valid for the current
///   N-API callback scope.
unsafe fn raw_create_error(
    env: sys::napi_env,
    code: &str,
    message: &str,
) -> sys::napi_value {
    // ...
}
```

And for the `unsafe` blocks in `throw_mds_error` (line 150) and `throw_coded_error` (line 197):

```rust
// SAFETY: `env.raw()` returns a valid napi_env for the current callback.
// String slices are passed with explicit lengths, so null-termination is
// not required by napi_create_string_utf8. The err_obj null-check guards
// against creation failure before calling napi_throw.
unsafe {
    let err_obj = raw_create_error(raw_env, &serialized.code, &serialized.message);
    // ...
}
```

---

**Silently discarded N-API return statuses in `raw_create_error` can cascade into null pointer dereference** - `crates/mds-napi/src/lib.rs:94-106`
**Confidence**: 82%
- Problem: In `raw_create_error`, the return status of `napi_create_string_utf8` at lines 94 and 100 is discarded with `let _ =`. If the first call fails (e.g., due to a pending exception or invalid env), `code_val` remains `ptr::null_mut()`. This null pointer is then passed to `napi_create_error` at line 106, which may exhibit undefined behavior depending on the N-API implementation. While the final null check on `err_val` (line 152/199) provides a fallback path, the intermediate null propagation through `napi_create_error` is not guaranteed to be safe by the N-API specification.
- Fix: Check the status of string creation calls and return null early on failure:

```rust
unsafe fn raw_create_error(
    env: sys::napi_env,
    code: &str,
    message: &str,
) -> sys::napi_value {
    let mut code_val: sys::napi_value = ptr::null_mut();
    let mut msg_val: sys::napi_value = ptr::null_mut();
    let mut err_val: sys::napi_value = ptr::null_mut();

    if sys::napi_create_string_utf8(env, code.as_ptr().cast(), code.len() as isize, &mut code_val)
        != sys::Status::napi_ok
    {
        return ptr::null_mut();
    }
    if sys::napi_create_string_utf8(env, message.as_ptr().cast(), message.len() as isize, &mut msg_val)
        != sys::Status::napi_ok
    {
        return ptr::null_mut();
    }
    let _ = sys::napi_create_error(env, code_val, msg_val, &mut err_val);

    err_val
}
```

### MEDIUM

**`extern crate napi_build` is unnecessary in Rust 2021 edition** - `crates/mds-napi/build.rs:1`
**Confidence**: 95%
- Problem: The workspace uses `edition = "2021"`, which makes `extern crate` declarations unnecessary (the compiler resolves crate names from `Cargo.toml` automatically). While this compiles fine, it is a Rust 2015 idiom that is out of place in a 2021 edition crate.
- Fix:
```rust
fn main() {
    napi_build::setup();
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`AssertUnwindSafe` may be unnecessary** - `crates/mds-napi/src/lib.rs:427,461,493,520` (Confidence: 65%) -- The closures capture only owned types (`String`, `PathBuf`, `HashMap<String, Value>`, `Option<...>`) which all implement `UnwindSafe`. The `AssertUnwindSafe` wrapper may be removable, though it serves as a future-proofing measure if `mds-core` types change. Consider adding a comment explaining the rationale if kept intentionally.

- **`run_catching` type bound could use `+ Send` for future async compatibility** - `crates/mds-napi/src/lib.rs:211-214` (Confidence: 60%) -- If this crate ever needs to support async N-API operations (e.g., `napi::Task`), the `run_catching` closure would need `Send`. Adding it now would be forward-compatible. Low priority since all current operations are synchronous.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The overall Rust quality is high. The code demonstrates strong patterns: proper `Result`-based error handling throughout, correct use of `catch_unwind` at the FFI boundary, well-structured options parsing with exhaustive validation, appropriate resource limits mirroring the core crate, and good use of the napi-rs type system. The `thiserror`-based `MdsError` is properly converted to structured JS errors with rich metadata (code, help, span). The `unsafe` FFI code is functionally correct but needs documentation of safety invariants per Rust conventions. The one substantive concern is the cascading null propagation in `raw_create_error` when intermediate N-API calls fail.

# Reliability Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20T19:15

## Issues in Your Changes (BLOCKING)

### HIGH

**Unchecked napi_create_string_utf8 return status before passing to napi_create_error** - `crates/mds-napi/src/lib.rs:94-106`
**Confidence**: 85%
- Problem: In `raw_create_error`, the return values of `napi_create_string_utf8` (lines 94, 100) are discarded with `let _ =`. If either call fails (e.g., out-of-memory in the V8 heap), `code_val` or `msg_val` remains `ptr::null_mut()`. The subsequent `napi_create_error(env, code_val, msg_val, &mut err_val)` on line 106 then receives null pointers. While the N-API spec says `napi_create_error` should reject null `msg`, the behavior for a null `code` is under-specified — this could result in a V8 crash or a confusing error object. The caller does check `err_val.is_null()` as a fallback (line 175), which mitigates partial failure, but the root issue is proceeding with potentially null intermediate values.
- Fix: Check the return status of each `napi_create_string_utf8` call and return `ptr::null_mut()` early if either fails. This will reliably trigger the fallback path in `throw_mds_error` (line 176-178).
```rust
unsafe fn raw_create_error(
    env: sys::napi_env,
    code: &str,
    message: &str,
) -> sys::napi_value {
    let mut code_val: sys::napi_value = ptr::null_mut();
    let mut msg_val: sys::napi_value = ptr::null_mut();
    let mut err_val: sys::napi_value = ptr::null_mut();

    if sys::napi_create_string_utf8(
        env,
        code.as_ptr().cast(),
        code.len() as isize,
        &mut code_val,
    ) != sys::Status::napi_ok
    {
        return ptr::null_mut();
    }
    if sys::napi_create_string_utf8(
        env,
        message.as_ptr().cast(),
        message.len() as isize,
        &mut msg_val,
    ) != sys::Status::napi_ok
    {
        return ptr::null_mut();
    }
    let _ = sys::napi_create_error(env, code_val, msg_val, &mut err_val);

    err_val
}
```

### MEDIUM

**`span.offset as u32` and `span.length as u32` silently truncate on large files** - `crates/mds-napi/src/lib.rs:160-161`
**Confidence**: 82%
- Problem: `SerializedSpan::offset` and `SerializedSpan::length` are `usize` (64-bit on 64-bit platforms). The casts `span.offset as u32` and `span.length as u32` silently truncate values exceeding `u32::MAX` (~4 GiB). Given the `MAX_SOURCE_SIZE` is 10 MiB, the offset will always fit in a `u32` in practice, but the code does not assert this invariant. If `MAX_SOURCE_SIZE` were ever raised (or if the error originates from a file-based path that resolves imports of many files), the truncation would produce incorrect span information.
- Fix: Use `u32::try_from` with a saturating fallback, or assert the invariant explicitly:
```rust
raw_set_uint32_prop(raw_env, span_obj, "offset",
    u32::try_from(span.offset).unwrap_or(u32::MAX));
raw_set_uint32_prop(raw_env, span_obj, "length",
    u32::try_from(span.length).unwrap_or(u32::MAX));
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **AssertUnwindSafe on closures capturing mutable state** - `crates/mds-napi/src/lib.rs:427,461,493,520` (Confidence: 65%) — All four napi exports wrap their closures in `AssertUnwindSafe`. The closures capture only owned values via `move` (`String`, `PathBuf`, `Option<HashMap<...>>`), so there is no shared mutable state that could be left in an inconsistent state after a panic. The usage is correct and safe here. However, there is no comment explaining *why* `AssertUnwindSafe` is safe in this context. A brief comment would help future maintainers understand that the invariants hold because all captured values are owned and will be dropped after unwind.

- **`len() as isize` cast in napi_create_string_utf8 calls** - `crates/mds-napi/src/lib.rs:97,103,118` (Confidence: 60%) — Casting `usize` to `isize` can overflow to negative on strings larger than `isize::MAX`. This is extremely unlikely in practice (would require a ~9 exabyte string on 64-bit), but the N-API spec uses -1 as a sentinel for "null-terminated" mode. A negative value from overflow could be misinterpreted. The `MAX_SOURCE_SIZE` of 10 MiB makes this purely theoretical.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Observations

The reliability posture of this code is strong overall:

1. **Bounded iteration**: The only loop (`for (key, val) in vars_map`) iterates over a finite map. No unbounded retries or pagination. All compilation is delegated to `mds-core`, which has its own depth bounds (`MAX_IMPORT_DEPTH = 64`, `MAX_VALUE_DEPTH = 64`).

2. **Resource limits enforced at boundary**: `check_source_size` enforces `MAX_SOURCE_SIZE` (10 MiB) before entering the compiler, mirroring the core crate's file-level limit. This prevents oversized allocations from reaching the compiler.

3. **Panic safety**: `catch_unwind` correctly wraps all compiler calls. The `AssertUnwindSafe` wrapper is appropriate since all closures capture only owned values via `move`, so no shared mutable state can be left inconsistent after an unwind. Panics are converted to structured JS errors with code `mds::internal`.

4. **Error propagation completeness**: Every error path is handled — `MdsError` is converted via `throw_mds_error`, options parsing errors via `throw_options_error`, and resource limits via `throw_resource_limit`. The fallback from raw napi error creation to `env.throw_error()` (lines 176-178, 201-203) ensures errors are always surfaced even if low-level napi calls fail.

5. **No allocation in hot paths**: The napi boundary code does minimal allocation — it parses options once, converts to core types, and delegates to the compiler. No per-character or per-line allocation patterns.

6. **`debug-panics` feature gated**: Panic detail exposure is behind a compile-time feature flag with explicit documentation that it must never be enabled in production (Cargo.toml line 13).

### Conditions for Approval

Fix the HIGH-severity finding (unchecked `napi_create_string_utf8` return status) before merge. The MEDIUM finding (u32 truncation) is a hardening improvement that should be addressed but is not blocking given the 10 MiB source limit.

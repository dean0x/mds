# Complexity Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20

## Function Complexity Analysis

Before categorizing issues, here is the per-function complexity audit for `crates/mds-napi/src/lib.rs` (525 lines):

| Function | Lines | Cyclomatic | Max Nesting | Params | Assessment |
|----------|-------|------------|-------------|--------|------------|
| `raw_create_error` | 24 (85-109) | 1 | 1 | 3 | Good |
| `raw_set_string_prop` | 12 (112-124) | 3 | 2 | 4 | Good |
| `raw_set_uint32_prop` | 8 (127-134) | 3 | 2 | 4 | Good |
| `throw_mds_error` | 36 (146-182) | 8 | 5 | 2 | Warning |
| `throw_options_error` | 3 (185-187) | 1 | 0 | 2 | Good |
| `throw_resource_limit` | 3 (190-192) | 1 | 0 | 2 | Good |
| `throw_coded_error` | 12 (195-206) | 2 | 1 | 3 | Good |
| `run_catching` | 24 (211-238) | 5 | 3 | 2 | Good |
| `check_source_size` | 12 (243-255) | 2 | 1 | 2 | Good |
| `json_type_name` | 9 (259-268) | 6 | 1 | 1 | Good |
| `parse_vars_field` | 24 (273-296) | 4 | 2 | 2 | Good |
| `parse_compile_opts` | 51 (304-354) | 8 | 2 | 2 | Warning |
| `parse_file_opts` | 36 (359-395) | 5 | 1 | 2 | Good |
| `compile` | 14 (418-436) | 1 | 0 | 3 | Good |
| `compile_file` | 13 (453-470) | 1 | 0 | 3 | Good |
| `check` | 14 (484-498) | 1 | 0 | 3 | Good |
| `check_file` | 9 (512-525) | 1 | 0 | 3 | Good |

**File-level**: 525 lines total (493 code + 32 doc header). Above the 300-line warning but below the 500-line critical threshold for code lines. The file is a single-module FFI boundary, so a unified file is reasonable.

## Issues in Your Changes (BLOCKING)

### HIGH

**`throw_mds_error` nesting depth reaches 5 levels** - `crates/mds-napi/src/lib.rs:146-182`
**Confidence**: 85%
- Problem: The `throw_mds_error` function reaches 5 levels of nesting depth at its deepest point (lines 162-169): `unsafe` > `if !err_obj.is_null()` > `if let Some(span)` > `if napi_create_object == napi_ok` > `if let Some(line)` / `if let Ok(ckey)`. The function has cyclomatic complexity of 8 (null check, help optional, span optional, create_object status, line optional, column optional, CString ok, else fallback). This is the most complex function in the file and the hardest to reason about at a glance.
- Fix: Extract the span-building logic into a dedicated helper function. This reduces `throw_mds_error` to 3 nesting levels and makes the span construction independently testable in concept:

```rust
/// Build a JS span object from serialized span data using raw N-API.
/// Returns null on failure.
unsafe fn raw_create_span_obj(
    env: sys::napi_env,
    span: &mds::SerializedSpan,  // adjust type to match actual
) -> sys::napi_value {
    let mut span_obj: sys::napi_value = ptr::null_mut();
    if sys::napi_create_object(env, &mut span_obj) != sys::Status::napi_ok {
        return ptr::null_mut();
    }
    raw_set_uint32_prop(env, span_obj, "offset", span.offset as u32);
    raw_set_uint32_prop(env, span_obj, "length", span.length as u32);
    if let Some(line) = span.line {
        raw_set_uint32_prop(env, span_obj, "line", line as u32);
    }
    if let Some(column) = span.column {
        raw_set_uint32_prop(env, span_obj, "column", column as u32);
    }
    span_obj
}
```

Then `throw_mds_error` simplifies to:

```rust
fn throw_mds_error(env: &Env, err: mds::MdsError) -> napi::Error {
    let serialized = err.serialize();
    let raw_env = env.raw();

    unsafe {
        let err_obj = raw_create_error(raw_env, &serialized.code, &serialized.message);
        if !err_obj.is_null() {
            if let Some(help) = &serialized.help {
                raw_set_string_prop(raw_env, err_obj, "help", help);
            }
            if let Some(span) = &serialized.span {
                let span_obj = raw_create_span_obj(raw_env, span);
                if !span_obj.is_null() {
                    if let Ok(ckey) = CString::new("span") {
                        let _ = sys::napi_set_named_property(raw_env, err_obj, ckey.as_ptr(), span_obj);
                    }
                }
            }
            let _ = sys::napi_throw(raw_env, err_obj);
        } else {
            let _ = env.throw_error(&serialized.message, Some(&serialized.code));
        }
    }

    napi::Error::new(Status::PendingException, "")
}
```

### MEDIUM

**`parse_compile_opts` at 51 lines with cyclomatic complexity 8** - `crates/mds-napi/src/lib.rs:304-354`
**Confidence**: 80%
- Problem: This function is the longest in the file at 51 lines (above the 50-line warning threshold) with 8 decision paths: early return on `None`, object check, `basePath` string match (4 arms: String, None, Null, other), empty string check, vars parse, unknown key check. Each path is individually simple due to early returns, but the aggregate size and branch count push it into warning territory.
- Fix: The `basePath` extraction (lines 317-339) can be pulled into a small helper to bring the function under 30 lines:

```rust
fn extract_base_path(
    env: &Env,
    map: &mut serde_json::Map<String, serde_json::Value>,
) -> napi::Result<Option<PathBuf>> {
    match map.remove("basePath") {
        Some(serde_json::Value::String(s)) => {
            if s.is_empty() {
                return Err(throw_options_error(env, "options.basePath must be a non-empty string"));
            }
            Ok(Some(PathBuf::from(s)))
        }
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(other) => Err(throw_options_error(
            env,
            &format!("options.basePath must be a string, got {}", json_type_name(&other)),
        )),
    }
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Structural duplication between `parse_compile_opts` and `parse_file_opts`** - `crates/mds-napi/src/lib.rs:304-395` (Confidence: 70%) -- Both functions share the same deserialization-then-validate skeleton (deserialize Object, check type, parse vars, reject unknowns). A shared `parse_opts_common` that returns the raw map could reduce the ~20 lines of duplicated boilerplate, though the current code is clear and the two functions diverge enough that this is a judgment call.

- **Test file is 429 lines with repetitive assertion shapes** - `crates/mds-napi/__test__/index.spec.mjs:1-429` (Confidence: 65%) -- Many error-shape tests (E-1 through E-9) and options-validation tests (V-1 through V-6) repeat the same `assert.throws(() => ..., (err) => { assert.equal(err.code, ...); return true; })` pattern. A small `assertThrowsWithCode(fn, expectedCode)` helper could cut 50+ lines while making test intent clearer. This is a test ergonomics suggestion, not a blocking concern.

- **`parse_compile_opts` basePath match has separate `None` and `Null` arms** - `crates/mds-napi/src/lib.rs:327-329` (Confidence: 65%) -- Lines 327 and 329 could be combined into a single arm `None | Some(serde_json::Value::Null) => None` to reduce one branch. Minor readability improvement.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates good structural discipline overall. Functions are well-decomposed with clear separation of concerns (raw FFI helpers, error conversion, options parsing, public API). The public API functions (`compile`, `compile_file`, `check`, `check_file`) are all under 15 lines with cyclomatic complexity of 1 -- excellent. The use of early returns keeps most functions flat. The two flagged items (`throw_mds_error` nesting and `parse_compile_opts` length) are localized and addressable with small extractions. No critical complexity issues exist.

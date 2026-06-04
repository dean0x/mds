# Complexity Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### HIGH

**`parse_options` exceeds function length threshold (126 lines)** - `crates/mds-wasm/src/lib.rs:130`
**Confidence**: 95%
- Problem: `parse_options` spans lines 130-256 (126 lines). This is well above the 50-line CRITICAL threshold and contains 3 sequential match blocks (filename, modules, vars) each with 3 arms and nested error construction. Cyclomatic complexity is approximately 12 (3 match blocks x 3 arms + 2 if-guards + early return). The function is understandable but costly to modify -- adding a new option requires copying the same match/error boilerplate.
- Fix: Extract each field parser into its own function. This reduces `parse_options` to a ~20-line orchestrator and makes each field's validation independently testable:

```rust
fn parse_filename(map: &serde_json::Map<String, serde_json::Value>) -> Result<String, JsValue> {
    match map.get("filename") {
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
        None => Ok("input.mds".to_string()),
        Some(serde_json::Value::String(_)) => Err(options_error("options.filename must be a non-empty string")),
        Some(other) => Err(options_error(&format!(
            "options.filename must be a string, got {}", json_type_name(other)
        ))),
    }
}

fn parse_modules(map: &serde_json::Map<String, serde_json::Value>) -> Result<HashMap<String, String>, JsValue> { ... }
fn parse_vars(map: &serde_json::Map<String, serde_json::Value>) -> Result<Option<HashMap<String, Value>>, JsValue> { ... }

fn parse_options(options: JsValue) -> Result<ParsedOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        return Ok(ParsedOptions { filename: "input.mds".to_string(), extra_modules: HashMap::new(), vars: None });
    }
    let opts_json: serde_json::Value = serde_wasm_bindgen::from_value(options)
        .map_err(|e| options_error(&format!("invalid options: {e}")))?;
    let serde_json::Value::Object(map) = &opts_json else {
        return Err(options_error("options must be a plain object"));
    };
    Ok(ParsedOptions {
        filename: parse_filename(map)?,
        extra_modules: parse_modules(map)?,
        vars: parse_vars(map)?,
    })
}
```

**Repeated JS error construction boilerplate (11 occurrences)** - `crates/mds-wasm/src/lib.rs:142,152,166,181,200,217,242,280,105,312`
**Confidence**: 92%
- Problem: The 4-line pattern of `js_sys::Error::new` + `Reflect::set` for the `code` property + conversion to `JsValue` is repeated 11 times across the file. In `parse_options` alone it appears 7 times. This inflates every function that needs to report errors and is the primary driver of the `parse_options` length problem. Each instance is 4-5 lines of identical boilerplate.
- Fix: Extract a helper that creates a coded JS error in one call:

```rust
/// Create a JS Error with a `code` property.
fn js_error(message: &str, code: &str) -> JsValue {
    let err = js_sys::Error::new(message);
    let _ = Reflect::set(&err, &JsValue::from_str("code"), &JsValue::from_str(code));
    err.into()
}

/// Shorthand for options validation errors.
fn options_error(message: &str) -> JsValue {
    js_error(message, "mds::invalid_options")
}
```

This would reduce each error site from 5 lines to 1, cutting ~50 lines from the file and significantly reducing `parse_options` length.

### MEDIUM

**`mds_error_to_js` has 4 levels of nesting in the span block** - `crates/mds-wasm/src/lib.rs:53-80`
**Confidence**: 82%
- Problem: Lines 53-80 contain an `if let Some(span)` block that nests to 4 levels deep (function -> if-let -> Reflect::set calls -> nested if-let for optional line/column). The function itself is 44 lines, within the warning zone (30-50 lines). The nesting is driven by 5 sequential `Reflect::set` calls for span properties.
- Fix: Extract span serialization to a dedicated helper:

```rust
fn span_to_js(span: &mds::SerializedSpan) -> js_sys::Object {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(&obj, &JsValue::from_str("offset"), &JsValue::from_f64(span.offset as f64));
    let _ = Reflect::set(&obj, &JsValue::from_str("length"), &JsValue::from_f64(span.length as f64));
    if let Some(line) = span.line {
        let _ = Reflect::set(&obj, &JsValue::from_str("line"), &JsValue::from_f64(line as f64));
    }
    if let Some(column) = span.column {
        let _ = Reflect::set(&obj, &JsValue::from_str("column"), &JsValue::from_f64(column as f64));
    }
    obj
}
```

This flattens `mds_error_to_js` to ~20 lines and max nesting depth of 2.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`compile` and `check` are near-identical** - `crates/mds-wasm/src/lib.rs:345,380` (Confidence: 65%) -- The two public functions share the same structure: clone source, catch_panic, parse_options, build_modules, call mds core, serialize. The bodies differ only in which mds function is called and how the result is wrapped. A shared `run_with_options` inner function could eliminate this duplication, but at 13 lines each the current form is not harmful.

- **Test file uses 4 nearly identical helper constructors** - `crates/mds-wasm/tests/web.rs:22,28,34` (Confidence: 62%) -- `vars_opts`, `modules_opts`, and `filename_opts` each construct a JSON object and convert it. A single `opts(json)` helper accepting any `serde_json::Value` could replace all three.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The `parse_options` function at 126 lines with cyclomatic complexity ~12 is the primary concern. The repeated JS error boilerplate (11 instances) is both the root cause of the length problem and a maintainability risk on its own -- adding a new option field or error code requires copying the same 5-line pattern. Extracting a `js_error` helper and splitting `parse_options` into per-field parsers would bring every function under the 30-line target and reduce the file from 393 lines to approximately 300.

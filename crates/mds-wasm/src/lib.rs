//! WebAssembly bindings for the MDS compiler.
//!
//! Exposes [`compile`] and [`check`] to JavaScript via `wasm-bindgen`.
//! All compilation runs against an in-memory virtual filesystem — no
//! OS file access occurs inside the WASM boundary.
//!
//! ## Error codes
//!
//! Errors thrown at the WASM boundary carry a `code` property. Codes that
//! originate inside `mds-core` (e.g. `"mds::syntax"`) are defined by
//! [`mds::MdsError`]. The following codes are **WASM-only** — they are
//! synthesised here and do not exist in the core crate:
//!
//! | Code                      | Meaning                                          |
//! |---------------------------|--------------------------------------------------|
//! | `mds::internal`           | Unexpected panic caught at the WASM boundary     |
//! | `mds::invalid_options`    | Malformed or type-incorrect options object       |
//! | `mds::resource_limit`     | Input exceeds an enforced size limit             |
//! | `mds::filename_collision` | `options.modules` key collides with `filename`   |
//!
//! ## Usage (JavaScript)
//!
//! ```js
//! import init, { compile, check } from 'mds-wasm';
//!
//! await init();
//!
//! const result = compile('Hello {name}!\n', {
//!   vars: { name: 'World' },
//!   filename: 'input.mds',
//! });
//! console.log(result.output); // "Hello World!\n"
//!
//! check('Hello {name}!\n', { vars: { name: 'World' } });
//! ```

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;

use js_sys::Reflect;
use mds::Value;
use serde::Serialize;
use wasm_bindgen::prelude::*;

// ── JS interop primitives ─────────────────────────────────────────────────────

/// Set a property on a JS object, asserting success in debug builds.
///
/// `Reflect::set` can only fail when the target is a non-extensible or
/// frozen object. We never set properties on such objects, so failure is a
/// programming error rather than a runtime condition. Silent discard is wrong
/// because it would produce incomplete error objects with no diagnostic trace;
/// a debug assertion surfaces the bug immediately during development.
#[inline]
fn set_prop(target: &JsValue, key: &str, value: &JsValue) {
    let ok = Reflect::set(target, &JsValue::from_str(key), value)
        .unwrap_or(false);
    debug_assert!(ok, "Reflect::set failed for key {key:?}");
}

/// Build a JS `Error` with a `code` property attached.
///
/// Every error thrown at the WASM boundary must carry `code` so callers can
/// branch programmatically (e.g. `if (err.code === "mds::syntax") …`).
#[inline]
fn js_error(message: &str, code: &str) -> JsValue {
    let err = js_sys::Error::new(message);
    set_prop(&err, "code", &JsValue::from_str(code));
    err.into()
}

/// Shorthand for a `js_error` with `code = "mds::invalid_options"`.
#[inline]
fn options_error(message: &str) -> JsValue {
    js_error(message, "mds::invalid_options")
}

// ── Error conversion helpers ──────────────────────────────────────────────────

/// Convert an [`mds::MdsError`] into a JS `Error` with structured metadata.
///
/// The returned object is a `js_sys::Error` with additional properties:
/// - `code`: diagnostic code string (e.g. `"mds::syntax"`)
/// - `help`: optional hint string (may be undefined)
/// - `span`: optional `{ offset, length, line, column }` object (may be undefined)
fn mds_error_to_js(err: mds::MdsError) -> JsValue {
    let serialized = err.serialize();

    let js_err = js_sys::Error::new(&serialized.message);
    set_prop(&js_err, "code", &JsValue::from_str(&serialized.code));

    if let Some(help) = &serialized.help {
        set_prop(&js_err, "help", &JsValue::from_str(help));
    }

    if let Some(span) = &serialized.span {
        let span_obj = span_to_js(span);
        set_prop(&js_err, "span", &span_obj);
    }

    js_err.into()
}

/// Serialise a [`mds::SerializedSpan`] into a plain JS object.
///
/// Always sets `offset` and `length`; sets `line` and `column` only when
/// the compiler was able to resolve them from the source text.
fn span_to_js(span: &mds::SerializedSpan) -> js_sys::Object {
    let obj = js_sys::Object::new();
    set_prop(&obj, "offset", &JsValue::from_f64(span.offset as f64));
    set_prop(&obj, "length", &JsValue::from_f64(span.length as f64));
    if let Some(line) = span.line {
        set_prop(&obj, "line", &JsValue::from_f64(line as f64));
    }
    if let Some(column) = span.column {
        set_prop(&obj, "column", &JsValue::from_f64(column as f64));
    }
    obj
}

/// Wrap a fallible closure in `catch_unwind` to prevent panics from aborting
/// the WASM module. Panics are converted to JS `Error` values with
/// `code = "mds::internal"`.
///
/// `AssertUnwindSafe` is required because the closure captures data that is
/// not `UnwindSafe` by default (e.g. `String`, `HashMap`). Callers ensure
/// this is safe by cloning all captured data before calling `catch_panic`.
fn catch_panic<F, T>(f: F) -> Result<T, JsValue>
where
    F: std::panic::UnwindSafe + FnOnce() -> Result<T, JsValue>,
{
    std::panic::catch_unwind(f).unwrap_or_else(|payload| {
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            format!("internal compiler panic: {s}")
        } else if let Some(s) = payload.downcast_ref::<String>() {
            format!("internal compiler panic: {s}")
        } else {
            "internal compiler panic: unknown internal error".to_string()
        };

        Err(js_error(&msg, "mds::internal"))
    })
}

// ── Options parsing ───────────────────────────────────────────────────────────

/// Parsed options extracted from the JS options object.
struct ParsedOptions {
    filename: String,
    extra_modules: HashMap<String, String>,
    vars: Option<HashMap<String, Value>>,
}

/// Parse the JS options argument into structured Rust data.
///
/// - `options` may be `null` or `undefined` — all fields default.
/// - `filename`: string key for the source in the virtual FS; default `"input.mds"`.
/// - `modules`: `Record<string, string>` of additional virtual files.
/// - `vars`: `Record<string, any>` of runtime variable overrides.
fn parse_options(options: JsValue) -> Result<ParsedOptions, JsValue> {
    // null / undefined → all defaults
    if options.is_null() || options.is_undefined() {
        return Ok(ParsedOptions {
            filename: "input.mds".to_string(),
            extra_modules: HashMap::new(),
            vars: None,
        });
    }

    // Deserialize options object → serde_json::Value for structured access.
    let opts_json: serde_json::Value = serde_wasm_bindgen::from_value(options)
        .map_err(|e| options_error(&format!("invalid options: {e}")))?;

    let serde_json::Value::Object(map) = &opts_json else {
        return Err(options_error("options must be a plain object"));
    };

    // Extract filename (string, default "input.mds").
    let filename = match map.get("filename") {
        Some(serde_json::Value::String(s)) => s.clone(),
        None => "input.mds".to_string(),
        Some(other) => {
            return Err(options_error(&format!(
                "options.filename must be a string, got {}",
                json_type_name(other)
            )));
        }
    };

    // Validate filename is non-empty.
    if filename.trim().is_empty() {
        return Err(options_error("options.filename must be a non-empty string"));
    }

    // Extract modules (Record<string, string>, default empty).
    let extra_modules = match map.get("modules") {
        Some(serde_json::Value::Object(mods)) => {
            let mut result = HashMap::with_capacity(mods.len());
            for (key, val) in mods {
                match val {
                    serde_json::Value::String(s) => {
                        result.insert(key.clone(), s.clone());
                    }
                    other => {
                        return Err(options_error(&format!(
                            "options.modules[\"{key}\"] must be a string, got {}",
                            json_type_name(other)
                        )));
                    }
                }
            }
            result
        }
        None => HashMap::new(),
        Some(other) => {
            return Err(options_error(&format!(
                "options.modules must be a plain object, got {}",
                json_type_name(other)
            )));
        }
    };

    // Extract vars (Record<string, any>, default None).
    let vars = match map.get("vars") {
        Some(serde_json::Value::Object(vars_map)) => {
            let mut result = HashMap::with_capacity(vars_map.len());
            for (key, val) in vars_map {
                let mds_val = Value::from_json(val.clone()).map_err(mds_error_to_js)?;
                result.insert(key.clone(), mds_val);
            }
            Some(result)
        }
        None => None,
        Some(other) => {
            return Err(options_error(&format!(
                "options.vars must be a plain object, got {}",
                json_type_name(other)
            )));
        }
    };

    Ok(ParsedOptions { filename, extra_modules, vars })
}

/// Return a human-readable JSON value type name for error diagnostics.
fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Build the virtual filesystem module map.
///
/// Inserts `source` under `filename`, then merges `extra_modules`. Returns
/// an error if `extra_modules` already contains `filename` (collision).
fn build_modules(
    source: String,
    filename: &str,
    extra_modules: HashMap<String, String>,
) -> Result<HashMap<String, String>, JsValue> {
    if extra_modules.contains_key(filename) {
        return Err(js_error(
            &format!(
                "options.modules already contains key \"{filename}\"; this would shadow the source — use a different filename"
            ),
            "mds::filename_collision",
        ));
    }

    let mut modules = extra_modules;
    modules.insert(filename.to_string(), source);
    Ok(modules)
}

// ── Output types ──────────────────────────────────────────────────────────────

/// Serializable output for the `check` function.
#[derive(Serialize)]
struct CheckOutput {
    warnings: Vec<String>,
}

/// Serialize a value to JS using the JSON-compatible serializer.
///
/// This ensures maps/structs become plain JS objects (not `Map` instances),
/// matching the behavior JavaScript callers expect from a JSON-like API.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| js_error(&format!("failed to serialize result: {e}"), "mds::internal"))
}

// ── Public WASM exports ───────────────────────────────────────────────────────

/// Compile an MDS template source string and return a structured result object.
///
/// ## Arguments
///
/// - `source`: MDS template source text.
/// - `options`: optional configuration object with the following optional fields:
///   - `filename` (string, default `"input.mds"`): the entry module key.
///   - `modules` (`Record<string, string>`): additional virtual modules for import resolution.
///   - `vars` (`Record<string, any>`): runtime variable overrides.
///
/// ## Returns
///
/// On success, a JS object `{ output: string, warnings: string[], dependencies: string[] }`.
///
/// On failure, throws a JS `Error` with additional properties:
/// - `code`: diagnostic code (e.g. `"mds::syntax"`)
/// - `help`: optional hint (may be absent)
/// - `span`: optional `{ offset, length, line?, column? }` (may be absent)
///
/// ## Example (JavaScript)
///
/// ```js
/// const result = compile('Hello {name}!\n', { vars: { name: 'World' } });
/// console.log(result.output); // "Hello World!\n"
/// ```
#[wasm_bindgen]
pub fn compile(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    // source must be cloned into an owned String so the closure is UnwindSafe.
    let source = source.to_string();

    catch_panic(AssertUnwindSafe(move || {
        let opts = parse_options(options)?;
        let modules = build_modules(source, &opts.filename, opts.extra_modules)?;
        let result =
            mds::compile_virtual_with_deps(modules, &opts.filename, opts.vars)
                .map_err(mds_error_to_js)?;

        to_js(&result)
    }))
}

/// Check (validate) an MDS template source string without rendering output.
///
/// ## Arguments
///
/// - `source`: MDS template source text.
/// - `options`: optional configuration object (same fields as [`compile`]).
///
/// ## Returns
///
/// On success, a JS object `{ warnings: string[] }`.
///
/// On failure, throws a JS `Error` with the same structure as [`compile`].
///
/// ## Example (JavaScript)
///
/// ```js
/// const result = check('---\nname: World\n---\nHello {name}!\n');
/// console.log(result.warnings); // []
/// ```
#[wasm_bindgen]
pub fn check(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    // source must be cloned into an owned String so the closure is UnwindSafe.
    let source = source.to_string();

    catch_panic(AssertUnwindSafe(move || {
        let opts = parse_options(options)?;
        let modules = build_modules(source, &opts.filename, opts.extra_modules)?;
        let ((), warnings) =
            mds::check_virtual_collecting_warnings(modules, &opts.filename, opts.vars)
                .map_err(mds_error_to_js)?;

        to_js(&CheckOutput { warnings })
    }))
}

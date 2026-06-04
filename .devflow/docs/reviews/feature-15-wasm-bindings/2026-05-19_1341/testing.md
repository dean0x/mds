# Testing Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing error span/help property assertions in error tests** - `crates/mds-wasm/tests/web.rs:119-143`
**Confidence**: 85%
- Problem: The error tests (`compile_undefined_variable_returns_error`, `compile_error_has_code_property`, `compile_error_is_js_error`) verify that `message` and `code` are present on JS errors, but never verify the `span` or `help` properties that `mds_error_to_js` conditionally attaches (lib.rs:48-79). The `mds_error_to_js` function has significant logic for building a `span` object with `offset`, `length`, `line`, and `column` fields, plus conditional `help` text -- none of which is exercised by any test assertion. An undefined-variable error should produce span information, making this a natural place to verify the full error shape.
- Fix: Add assertions to an existing error test (or a new focused test) that verifies the structured span object:
```rust
#[wasm_bindgen_test]
fn compile_error_has_span_properties() {
    let err = mds_wasm::compile("Hello {undefined_var}!\n", JsValue::NULL).unwrap_err();
    let span = get_prop(&err, "span");
    assert!(!span.is_undefined(), "error.span must be present for source errors");
    // Verify span sub-fields exist
    let offset = js_sys::Reflect::get(&span, &JsValue::from_str("offset")).unwrap();
    assert!(offset.as_f64().is_some(), "span.offset must be a number");
    let length = js_sys::Reflect::get(&span, &JsValue::from_str("length")).unwrap();
    assert!(length.as_f64().is_some(), "span.length must be a number");
}
```

**No test for `check()` with `modules` option** - `crates/mds-wasm/tests/web.rs:145-174`
**Confidence**: 82%
- Problem: The `check()` function accepts the same options as `compile()`, including `modules` for virtual FS imports. There is a test for `compile_with_modules_import` (line 72) but no corresponding test for `check()` with modules. The `check` function exercises a different underlying mds-core path (`check_virtual_collecting_warnings` vs `compile_virtual_with_deps`), so import resolution behavior should be verified independently. Without this, a regression in the check-path's module resolution would go undetected.
- Fix: Add a parallel test for `check()` with modules:
```rust
#[wasm_bindgen_test]
fn check_with_modules_import() {
    let source = "@import \"./lib.mds\"\n{greet(\"World\")}\n";
    let opts = modules_opts(&serde_json::json!({
        "lib.mds": "@define greet(x):\nHello {x}!\n@end\n"
    }));
    let result = mds_wasm::check(source, opts).unwrap();
    let warnings_arr = js_sys::Array::from(&get_prop(&result, "warnings"));
    assert_eq!(warnings_arr.length(), 0, "valid import should produce no warnings");
}
```

### MEDIUM

**No test for `check()` with runtime vars** - `crates/mds-wasm/tests/web.rs:145-174`
**Confidence**: 82%
- Problem: `compile_with_runtime_vars` (line 64) verifies that the `vars` option works for `compile()`, but there is no corresponding test for `check()` with runtime vars. Since `check()` also passes `opts.vars` through to `check_virtual_collecting_warnings`, a type mismatch or dropped vars in the check path would not be caught.
- Fix: Add:
```rust
#[wasm_bindgen_test]
fn check_with_runtime_vars() {
    let source = "Hello {name}!\n";
    let opts = vars_opts(&serde_json::json!({ "name": "World" }));
    let result = mds_wasm::check(source, opts).unwrap();
    let warnings_arr = js_sys::Array::from(&get_prop(&result, "warnings"));
    assert_eq!(warnings_arr.length(), 0, "should have no warnings when vars are provided");
}
```

**No test for `check()` validation options (empty filename, collision, invalid vars)** - `crates/mds-wasm/tests/web.rs:176-222`
**Confidence**: 80%
- Problem: Options validation tests only exercise `compile()`. The `parse_options` and `build_modules` functions are shared by both `compile()` and `check()`, so this is lower risk -- but `check()` wraps them in its own `catch_panic` call, so a panic-safety regression in the check path would not be detected. At minimum, one representative validation test through `check()` would guard this.
- Fix: Add one representative test:
```rust
#[wasm_bindgen_test]
fn check_empty_filename_returns_error() {
    let opts = filename_opts("");
    let err = mds_wasm::check("Hello!\n", opts).unwrap_err();
    let code = get_str(&err, "code");
    assert_eq!(code, "mds::invalid_options", "got: {code}");
}
```

**Dependencies field content not verified** - `crates/mds-wasm/tests/web.rs:92-97`
**Confidence**: 80%
- Problem: `compile_has_dependencies_field` only checks that `dependencies` is an array. `compile_with_modules_import` (line 72) exercises a template with an `@import` but never checks that the imported module appears in the `dependencies` array. This means the dependency tracking feature -- which is new in this branch via `compile_virtual_with_deps` -- has no assertion on its actual content in the WASM layer.
- Fix: Extend the existing modules test or add a new one:
```rust
#[wasm_bindgen_test]
fn compile_with_modules_tracks_dependencies() {
    let source = "@import \"./lib.mds\"\n{greet(\"World\")}\n";
    let opts = modules_opts(&serde_json::json!({
        "lib.mds": "@define greet(x):\nHello {x}!\n@end\n"
    }));
    let result = mds_wasm::compile(source, opts).unwrap();
    let deps = js_sys::Array::from(&get_prop(&result, "dependencies"));
    assert_eq!(deps.length(), 1, "should have 1 dependency");
    assert_eq!(deps.get(0).as_string().unwrap(), "lib.mds");
}
```

## Issues in Code You Touched (Should Fix)

_None identified._

## Pre-existing Issues (Not Blocking)

_None identified._

## Suggestions (Lower Confidence)

- **No test for panic recovery path** - `crates/mds-wasm/src/lib.rs:92-113` (Confidence: 65%) -- The `catch_panic` function converts panics to JS errors with code `"mds::internal"`, but no test triggers a panic to verify this path. While triggering a panic in a WASM test is difficult (requires a bug in mds-core), this is a safety-critical boundary. Consider a unit test if a way to trigger a controlled panic can be found.

- **`check_error_has_code_property` duplicates `check_invalid_template_returns_error`** - `crates/mds-wasm/tests/web.rs:162-174` (Confidence: 70%) -- Both tests call `check()` with an undefined variable and assert on `code`. Lines 163-167 assert `!code.is_empty()`, while lines 170-173 assert `code.starts_with("mds::")`. The second test is strictly stronger. Consider merging them into a single test to reduce redundancy, or differentiating the input to cover a distinct error variant.

- **No test for `options.modules` with non-string value** - `crates/mds-wasm/src/lib.rs:199-209` (Confidence: 72%) -- `parse_options` validates that each module value is a string and returns `mds::invalid_options` for non-string values. This branch has no test coverage via `web.rs`. While the code is straightforward, a test would guard against accidental removal of the validation.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 3 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The test suite is well-structured with 21 WASM tests and 13 mds-core tests covering the new `Value::from_json` and `load_vars_str` additions. Tests follow good patterns: clear Arrange-Act-Assert structure, descriptive names, helper functions to reduce boilerplate, and good coverage of both happy paths and error cases including input validation.

The main gaps are: (1) the structured error properties (`span`, `help`) that represent significant code in `mds_error_to_js` have zero assertion coverage, and (2) the `check()` function has notably less test coverage than `compile()` despite exercising a different mds-core code path. The dependency tracking feature content is asserted only as "is an array" without verifying actual dependency entries. These are all addressable with a small number of additional tests.

# Architecture Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### HIGH

**Repetitive error-construction boilerplate in parse_options** - `crates/mds-wasm/src/lib.rs:130-256`
**Confidence**: 90%
- Problem: The `parse_options` function contains 7 nearly-identical blocks that construct a `js_sys::Error`, set a `code` property via `Reflect::set`, and return `Err(js_err.into())`. This violates DRY and is a shallow module smell (Ousterhout): the repetition makes the function harder to maintain and increases the chance of inconsistency if the error structure changes.
- Fix: Extract a helper that constructs a structured JS error from a message and code:
  ```rust
  fn js_error(code: &str, message: &str) -> JsValue {
      let js_err = js_sys::Error::new(message);
      let _ = Reflect::set(&js_err, &JsValue::from_str("code"), &JsValue::from_str(code));
      js_err.into()
  }
  ```
  Then each validation site becomes a one-liner: `return Err(js_error("mds::invalid_options", &msg));`. The same helper can replace the identical pattern in `mds_error_to_js` (lines 42-45), `catch_panic` (lines 105-109), and `build_modules` (lines 280-288). This collapses ~60 lines of boilerplate into ~5 and ensures all JS errors have a consistent shape from a single source of truth.

### MEDIUM

**`parse_options` does too much -- single function with mixed concerns** - `crates/mds-wasm/src/lib.rs:130-256`
**Confidence**: 82%
- Problem: `parse_options` handles deserialization, type validation, and semantic validation (empty filename) for three distinct option fields in a single 126-line function. While not egregious for a boundary-layer parser, the function has multiple reasons to change (adding a new option field, changing validation for one field, changing the deserialization format). This is a mild SRP concern.
- Fix: Consider splitting into focused extraction functions like `parse_filename(map) -> Result<String>`, `parse_modules(map) -> Result<HashMap>`, `parse_vars(map) -> Result<Option<HashMap>>`. The top-level `parse_options` becomes an orchestrator that calls each. This also makes individual field parsers independently testable. Not critical given the current 3-field scope, but will matter as the options surface grows.

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

_No issues found._

## Suggestions (Lower Confidence)

- **Consider a typed Options struct via serde instead of manual parsing** - `crates/mds-wasm/src/lib.rs:130-256` (Confidence: 65%) -- serde_wasm_bindgen could deserialize directly into a typed `#[derive(Deserialize)] struct JsOptions { filename: Option<String>, modules: Option<HashMap<String,String>>, vars: Option<serde_json::Value> }`, replacing the manual field extraction with automatic type checking. Trade-off: custom error messages would need `#[serde(deny_unknown_fields)]` and custom deserializer, and the current approach gives precise per-field diagnostics. Worth evaluating as options grow.

- **Workspace `panic = "unwind"` affects all crates, not just mds-wasm** - `Cargo.toml:29-35` (Confidence: 70%) -- Setting `panic = "unwind"` at workspace level in both `[profile.dev]` and `[profile.release]` is required for `catch_unwind` in the WASM crate, but it also changes the panic strategy for `mds-cli` and `mds-core` in release builds (where `panic = "abort"` is a common choice for smaller binaries and faster code). The per-package `[profile.release.package.mds-wasm]` override only tunes optimization, not panic. This is an intentional trade-off documented in the commit message, but worth noting as the CLI binary size may be slightly larger than necessary.

- **`AssertUnwindSafe` usage merits a note on soundness** - `crates/mds-wasm/src/lib.rs:349,384` (Confidence: 62%) -- The comment at line 89-91 explains why `AssertUnwindSafe` is safe, and the cloning of captured data before the closure is correct practice. However, if future changes add `&mut` references or shared state into the closure without cloning, the `AssertUnwindSafe` wrapper would silently suppress the compiler's unwind-safety check. A `// SAFETY:` comment (Rust convention for unsafe-adjacent invariants) at each call site would make the contract explicit for future maintainers.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The WASM bindings crate demonstrates strong architectural design:

- **Clean layering**: `mds-wasm` depends on `mds-core` (via the `mds` alias) and never reaches into internal modules. The dependency direction is strictly inward, respecting the Dependency Rule.
- **Boundary isolation**: The WASM boundary is a proper adapter layer. All JS<->Rust conversions happen in `mds-wasm/src/lib.rs` and never leak into core. `VirtualFs` ensures no OS filesystem access from WASM -- a clean port/adapter separation.
- **Additive core changes**: The two `mds-core` changes (`Value::from_json` visibility promotion and `load_vars_str` addition) are minimal, backward-compatible, and follow existing patterns exactly (`load_vars_str` mirrors `load_vars_file`).
- **DIP compliance**: `mds-wasm` depends on the `mds-core` public API abstractions (traits, error types), not on internal implementation details.
- **Error contract**: Structured JS errors with `code`, `help`, and `span` properties provide a well-defined error interface for JS consumers.

The one blocking condition is the error-construction boilerplate (HIGH): extracting a `js_error` helper would significantly reduce repetition and establish a single source of truth for the JS error shape. The `parse_options` SRP concern (MEDIUM) is worth addressing but not blocking.

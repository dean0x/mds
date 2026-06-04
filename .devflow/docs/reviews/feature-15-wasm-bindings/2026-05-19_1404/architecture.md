# Architecture Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19
**Scope**: Incremental review of 7 resolution commits (420e2259...HEAD)

## Issues in Your Changes (BLOCKING)

No blocking architectural issues found.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Duplicated default-options construction** - `crates/mds-wasm/src/lib.rs:250-255`
**Confidence**: 82%
- Problem: The `ParsedOptions` default values (`"input.mds"`, empty `HashMap`, `None`) are constructed in two places: the early-return null/undefined guard in `parse_options` (line 250-255) and implicitly as defaults inside `parse_filename` (line 179), `parse_modules` (line 211), and `parse_vars` (line 234). If a default changes (e.g. the default filename), developers must update both the early-return path and the per-field parser, violating DRY and risking inconsistency.
- Fix: Extract defaults into constants or a `Default` impl for `ParsedOptions`, then use it in both paths:
  ```rust
  impl Default for ParsedOptions {
      fn default() -> Self {
          Self {
              filename: "input.mds".to_string(),
              extra_modules: HashMap::new(),
              vars: None,
          }
      }
  }
  
  fn parse_options(options: JsValue) -> Result<ParsedOptions, JsValue> {
      if options.is_null() || options.is_undefined() {
          return Ok(ParsedOptions::default());
      }
      // ... rest unchanged
  }
  ```
  This makes `parse_filename`/`parse_modules`/`parse_vars` the single source for per-field defaults, and the null/undefined path simply returns the aggregate default.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`compile` and `check` share near-identical control flow** - `crates/mds-wasm/src/lib.rs:369-384` and `crates/mds-wasm/src/lib.rs:406-421`
**Confidence**: 85%
- Problem: The two public WASM exports (`compile` and `check`) follow the same pattern: `check_source_size` -> clone source -> `catch_panic(AssertUnwindSafe(move || { parse_options -> build_modules -> call_mds_core -> to_js }))`. The only difference is which `mds::` function is called and the output type. This is an Open-Closed Principle concern: adding a third entry point (e.g. `format`, `lint`) would require copying the same boilerplate again.
- Fix: This is acceptable for two functions. If a third is added, extract a generic harness:
  ```rust
  fn wasm_entry<F, T: Serialize>(source: &str, options: JsValue, core_fn: F) -> Result<JsValue, JsValue>
  where
      F: FnOnce(HashMap<String, String>, &str, Option<HashMap<String, Value>>) -> Result<T, mds::MdsError>
          + std::panic::UnwindSafe,
  {
      check_source_size(source)?;
      let source = source.to_string();
      catch_panic(AssertUnwindSafe(move || {
          let opts = parse_options(options)?;
          let modules = build_modules(source, &opts.filename, opts.extra_modules)?;
          let result = core_fn(modules, &opts.filename, opts.vars).map_err(mds_error_to_js)?;
          to_js(&result)
      }))
  }
  ```
  Not blocking since the current duplication is between exactly two functions and both are well-documented.

## Suggestions (Lower Confidence)

- **Unknown options fields silently ignored** - `crates/mds-wasm/src/lib.rs:266-268` (Confidence: 68%) -- After extracting `filename`, `modules`, and `vars` via `remove`, any remaining keys in the map are silently discarded. A typo like `{ varss: {...} }` would produce no warning. Consider emitting a warning or error for unrecognised keys (checking `map.is_empty()` after parsing).

- **`set_prop` silently swallows failures in release builds** - `crates/mds-wasm/src/lib.rs:60-64` (Confidence: 62%) -- The `debug_assert!` in `set_prop` means a failed `Reflect::set` in release mode silently produces an incomplete error object (e.g. missing `code` property). This is documented as intentional since failure only occurs on frozen/non-extensible objects which the crate never creates, but the consequence is that a corrupted error object could confuse callers. An alternative is `assert!` given `set_prop` is not on a hot path.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED

### Rationale

The incremental changes demonstrate strong architectural improvements:

1. **Separation of Concerns (excellent)**: The JS interop layer (`set_prop`, `js_error`, `options_error`, `span_to_js`) is cleanly separated from business logic. Each helper has a single responsibility and the extraction eliminates boilerplate duplication across the crate.

2. **Layering (excellent)**: The `mds-wasm` crate acts as a thin adapter layer over `mds-core`, consistent with Hexagonal Architecture. Dependencies flow inward only: `mds-wasm -> mds-core`. No domain logic leaks into the WASM boundary. The `VirtualFs` abstraction from core is reused correctly -- no filesystem coupling.

3. **DIP compliance (good)**: The WASM crate depends on `mds-core` public API abstractions (`compile_virtual_with_deps`, `check_virtual_collecting_warnings`, `Value::from_json`) rather than internal implementation details.

4. **Single Responsibility (good)**: The split of `parse_options` into `parse_filename`, `parse_modules`, and `parse_vars` follows SRP -- each parser validates one field. The `check_source_size` extraction eliminates duplicated guards.

5. **Error architecture (strong)**: Structured error codes at the boundary (`mds::internal`, `mds::invalid_options`, `mds::resource_limit`, `mds::filename_collision`) are well-defined and documented. The panic-to-error conversion via `catch_panic` prevents WASM module abort while sanitizing internal details.

6. **Workspace profile concern addressed**: The `panic = "unwind"` workspace-wide requirement is clearly documented with rationale, mitigating confusion for future contributors.

The minor duplication noted between `compile` and `check` is acceptable at this scale and the default-value DRY concern is a straightforward improvement. Overall the architecture is clean, well-layered, and follows established patterns.

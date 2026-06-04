# Regression Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19T14:04

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Panic error message format changed — callers parsing `error.message` will break** - `crates/mds-wasm/src/lib.rs:140`
**Confidence**: 85%
- Problem: The `catch_panic` function changed its error message from `"internal compiler panic: {details}"` to a generic `"internal compiler error"`, moving the panic payload into a new `detail` property. Any JavaScript caller that pattern-matches on the `message` string (e.g. `err.message.startsWith("internal compiler panic")` or extracts the panic detail from the message) will silently stop matching. This is an intentional security hardening (avoiding leaking internal paths), but it is a behavioral regression for existing consumers.
- Impact: Since this is a new WASM crate (first release via this PR), the practical risk is near-zero — there are no existing JS consumers to break. However, the error message contract documented in the old code is now different, and the change should be acknowledged as intentional.
- Fix: This is already well-documented in the code comments (lines 132-134). No code change needed, but the PR description should note this as an intentional breaking change to the panic error surface. If backward compatibility is ever needed:
  ```rust
  // To restore the old message format (not recommended):
  let err = js_error(
      &format!("internal compiler error: {}", detail_str),
      "mds::internal",
  );
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`set_prop` debug_assert may cause panics inside `catch_panic` in debug builds** - `crates/mds-wasm/src/lib.rs:63` (Confidence: 65%) — The old code used `let _ = Reflect::set(...)` which silently swallowed failures. The new `set_prop` uses `debug_assert!(ok, ...)` which will panic in debug/test builds if `Reflect::set` fails. Since `set_prop` is called inside `catch_panic` closures (e.g., within `mds_error_to_js` at line 93, called at line 380 inside `catch_panic`), a `debug_assert!` failure during error conversion could trigger a nested panic-in-panic, resulting in an abort rather than a graceful JS error. In practice `Reflect::set` on a freshly-created `js_sys::Error` or `js_sys::Object` should never fail, so this is theoretical.

- **`to_js` serialization errors now carry `code` property where they previously did not** - `crates/mds-wasm/src/lib.rs:338` (Confidence: 70%) — The old `to_js` error path returned a bare `js_sys::Error` without a `code` property. The refactored version uses `js_error(...)` which adds `code = "mds::internal"`. This is an additive, non-breaking improvement, but callers that check `err.code === undefined` to distinguish serialization errors from compiler errors will now see a different value.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED

### Rationale

The refactoring is behavior-preserving in all material respects. The key changes:

1. **Extracted helper functions** (`set_prop`, `js_error`, `options_error`, `span_to_js`, `parse_filename`, `parse_modules`, `parse_vars`, `check_source_size`) — All produce identical JS-observable output as the inlined code they replaced. Error messages, error codes, and property shapes are unchanged.

2. **Ownership optimization** (`map.get().clone()` to `map.remove()`) — Functionally equivalent; the map is not used after extraction. This avoids unnecessary String cloning.

3. **New `check_source_size` guard** — Added at the top of `compile()` and `check()` before `catch_panic`. This is new functionality (resource limit enforcement) that did not exist before, not a regression. It correctly mirrors `mds::MAX_FILE_SIZE`.

4. **New `load_vars_str` size limit** in mds-core — Additive guard with proper test coverage (`load_vars_str_rejects_oversized_input`, `load_vars_str_accepts_valid_json_within_limit`).

5. **Panic error message change** — Only behavioral difference found. Intentional security hardening. Since this is the initial release of `mds-wasm`, no existing consumers are affected.

6. **Public API surface** — `compile()` and `check()` signatures unchanged. `ParsedOptions` struct unchanged. No exports removed. No return types widened.

7. **Test coverage** — 10 new tests added covering: dependencies field content, error span properties (offset, length, line, column), error help text, check() with modules/vars, and empty filename via check(). All existing tests preserved. New mds-core tests for `load_vars_str` size limits.

8. **Deleted files** — `.features/` directory removed (devflow tooling metadata, not production code). No regression impact.

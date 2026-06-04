# Rust Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Silenced `Reflect::set` return values (8 occurrences)** -- Confidence: 82%
- `crates/mds-wasm/src/lib.rs:45`, `crates/mds-wasm/src/lib.rs:49`, `crates/mds-wasm/src/lib.rs:55`, `crates/mds-wasm/src/lib.rs:60`, `crates/mds-wasm/src/lib.rs:66`, `crates/mds-wasm/src/lib.rs:73`, `crates/mds-wasm/src/lib.rs:79`, `crates/mds-wasm/src/lib.rs:106`
- Problem: Every call to `Reflect::set()` returns a `Result<bool, JsValue>` that is discarded with `let _ =`. While `Reflect::set` on a plain `js_sys::Error` or `js_sys::Object` is extremely unlikely to fail in practice (these are extensible JS objects, not proxies with traps), discarding the result silently hides any failure to set metadata properties on errors. If a set fails, the JS consumer would receive an `Error` missing `code`, `help`, or `span` -- degraded diagnostics with no indication of why.
- Fix: Since this is a WASM boundary crate (not a hot path), consider at minimum debug-asserting success. A pragmatic approach: leave `let _ =` but add a single explanatory comment at the top of the file documenting the rationale, so future maintainers know the suppression is intentional and not an oversight:

```rust
// NOTE: Reflect::set on plain JS objects (Error, Object) cannot fail
// unless the target is a Proxy with a throwing set trap. We use `let _ =`
// throughout to keep error-construction code readable. If this assumption
// ever changes, grep for `let _ = Reflect::set` to audit.
```

Alternatively, extract a small helper that asserts in debug builds:

```rust
fn set_prop(obj: &js_sys::Object, key: &str, val: &JsValue) {
    let ok = Reflect::set(obj, &JsValue::from_str(key), val);
    debug_assert!(ok.is_ok(), "Reflect::set failed on plain object");
}
```

**Workspace-wide `panic = "unwind"` affects all crates** -- Confidence: 85%
- `Cargo.toml:29-34`
- Problem: The `[profile.dev]` and `[profile.release]` sections set `panic = "unwind"` at the workspace level. While this is required for `catch_unwind` in `mds-wasm`, it now applies to `mds-core` and `mds-cli` as well. On `dev` profile this is the Rust default (so no change), but on `release` the Rust default is `abort` (smaller, faster). Setting `panic = "unwind"` in `[profile.release]` prevents the compiler from eliding unwind tables in `mds-cli` release builds, slightly increasing binary size and potentially reducing optimization opportunities.
- Fix: Use a per-package override so only `mds-wasm` is forced to `unwind` in release builds. `mds-core` must also be `unwind` since `mds-wasm` depends on it, but `mds-cli` could remain at default:

```toml
[profile.release]
lto = true
# panic = "unwind" removed from workspace-wide setting

[profile.release.package.mds-core]
panic = "unwind"  # required: mds-wasm calls catch_unwind on mds-core code

[profile.release.package.mds-wasm]
opt-level = "z"
strip = true
codegen-units = 1
panic = "unwind"  # required: catch_unwind at WASM boundary
```

Note: The `[profile.dev]` line (`panic = "unwind"`) is already the Rust default for dev builds, so it is effectively a no-op but worth removing for clarity.

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

_No issues found._

## Suggestions (Lower Confidence)

- **Avoidable clones in `parse_options` via ownership destructuring** - `crates/mds-wasm/src/lib.rs:151` (Confidence: 65%) -- Line 151 borrows `opts_json` with `let serde_json::Value::Object(map) = &opts_json`, forcing `.clone()` on every extracted string (lines 163, 197, 236). Destructuring by value (`let serde_json::Value::Object(mut map) = opts_json`) would allow `.remove()` instead of `.get()` + `.clone()`, eliminating allocations. The data is small (options object), so this is a minor optimization.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Rust Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The Rust code quality is strong. Key strengths:

- Zero `.unwrap()` in library code -- all error paths use `?` or explicit `map_err`
- Zero `unsafe` blocks
- Proper `#[must_use]` annotations consistent with existing codebase convention
- `AssertUnwindSafe` usage is well-documented with clear safety justification
- Boundary validation is thorough (type checking on all option fields, empty filename guard, collision detection)
- Depth limits on JSON parsing protect against stack overflow
- `Value::from_json` visibility promotion (`pub(crate)` to `pub`) is minimal and correct
- Test coverage is comprehensive: 21 wasm-bindgen-test cases + 12 new mds-core tests

The two MEDIUM findings are about defensive coding (silenced Reflect::set results) and build configuration scope (panic=unwind affecting all workspace members). Neither represents a correctness bug, but both are worth addressing for maintainability. The conditions for approval are:

1. Add a comment explaining the `let _ = Reflect::set` rationale (or extract a debug-asserting helper)
2. Scope `panic = "unwind"` to only the crates that need it in release profile

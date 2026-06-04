# Consistency Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20T19:15

## Issues in Your Changes (BLOCKING)

### HIGH

**Cargo.toml missing workspace metadata fields** - `crates/mds-napi/Cargo.toml:1-8`
**Confidence**: 95%
- Problem: All three existing crates (mds-core, mds-cli, mds-wasm) inherit `readme.workspace = true`, `keywords.workspace = true`, and define a crate-specific `categories` field. The mds-napi Cargo.toml omits all three.
- Impact: Breaks the workspace-wide convention. If mds-napi is ever published to crates.io, it will lack the shared metadata. More importantly, this is a consistency violation across all four crates.
- Fix:
  ```toml
  # crates/mds-napi/Cargo.toml — add after repository.workspace = true
  readme.workspace = true
  keywords.workspace = true
  categories = ["api-bindings"]
  ```

**debug-panics detail surfacing inconsistent with mds-wasm** - `crates/mds-napi/src/lib.rs:219-233`
**Confidence**: 85%
- Problem: When `debug-panics` is enabled, mds-wasm attaches the raw panic payload as a separate `err.detail` property (line 168 of mds-wasm). The mds-napi crate instead concatenates the detail into the error message string: `format!("internal compiler error (panic): {detail}")`. This means consumers switching between the two binding layers cannot use a consistent pattern to extract panic details.
- Impact: API shape inconsistency between the two JS binding crates. Consumers who check `err.detail` in wasm will not find it in napi. While `debug-panics` is a dev-only feature, the precedent set by mds-wasm is that detail goes on a dedicated property, not in the message.
- Fix: Add `err.detail` as a separate property on the thrown error object, matching mds-wasm:
  ```rust
  #[cfg(feature = "debug-panics")]
  let msg = "internal compiler error".to_string();
  #[cfg(feature = "debug-panics")]
  let detail = if let Some(s) = payload.downcast_ref::<&str>() {
      Some((*s).to_string())
  } else if let Some(s) = payload.downcast_ref::<String>() {
      Some(s.clone())
  } else {
      Some("unknown panic payload".to_string())
  };

  // After creating the error object, attach detail if present:
  // raw_set_string_prop(raw_env, err_obj, "detail", &detail);
  ```

### MEDIUM

**Release profile missing codegen-units vs mds-wasm** - `Cargo.toml:49-51`
**Confidence**: 82%
- Problem: The mds-wasm release profile sets `codegen-units = 1` (line 47) for maximum optimization. The new mds-napi release profile omits `codegen-units`. While mds-napi intentionally uses `opt-level = 3` (throughput-optimized) rather than `opt-level = "z"` (size-optimized), the absence of `codegen-units = 1` is inconsistent with the pattern of maximizing optimization for binding crates.
- Impact: Minor performance difference. `codegen-units = 1` enables better cross-function optimization at the cost of compile time. For a native addon, this is generally desirable.
- Fix:
  ```toml
  [profile.release.package.mds-napi]
  opt-level = 3
  strip = true
  codegen-units = 1
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**rust-version bump from 1.80 to 1.88 is undocumented** - `Cargo.toml:8`
**Confidence**: 80%
- Problem: The workspace `rust-version` was bumped from `1.80` to `1.88`. The commit message for `6f77328` mentions "resolve clippy lints (is_none_or, type_complexity)" which explains the need (the `is_none_or` method on `Option` was stabilized in Rust 1.82), but the bump to 1.88 is larger than the minimum required. This is a workspace-wide change that affects all crates, not just mds-napi.
- Impact: All crates now require Rust 1.88+. This may break builds for contributors on older toolchains. The change is bundled with the napi feature branch rather than being a separate, clearly scoped commit.
- Fix: No code change needed, but verify 1.88 is the intended MSRV (or whether 1.82 would suffice for the `is_none_or` usage in `parser.rs:628`).

## Pre-existing Issues (Not Blocking)

_No pre-existing consistency issues found._

## Suggestions (Lower Confidence)

- **Test ID naming convention differs from mds-wasm** - `crates/mds-napi/__test__/index.spec.mjs:31` (Confidence: 65%) -- The mds-napi tests use prefixed IDs like `F-C1`, `F-CF1`, `F-K1`, `E-1`, `V-1`, `R-1`, `P-1`. The mds-wasm tests use descriptive function names (`compile_simple_no_options`, `compile_undefined_variable_returns_error`). This is a different testing framework (JS node:test vs Rust wasm_bindgen_test), so the naming difference is arguably natural, but the ID-prefix style is unique to this crate.

- **Internal type naming: CheckResult vs CheckOutput** - `crates/mds-napi/src/lib.rs:71` (Confidence: 60%) -- mds-wasm names its check return type `CheckOutput` while mds-napi names it `CheckResult`. Both are internal types (not exposed to JS consumers), but the naming divergence could cause confusion when reading both crates side by side. The napi crate uses `CompileResult`/`CheckResult` (consistent with each other), while wasm uses plain serialization (no named compile output struct). Neither is wrong.

- **Test fixture directory naming** - `crates/mds-napi/__test__/` (Confidence: 62%) -- Uses `__test__` (double-underscore, napi-rs convention) while mds-cli uses `tests/` and mds-wasm uses `tests/`. This is a framework convention difference (napi-rs scaffold uses `__test__`), so it may be intentional, but it diverges from the Rust-side test directory pattern.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The mds-napi crate follows the established codebase patterns well overall -- section comment style, error code conventions (`mds::*`), resource limit enforcement, `debug-panics` feature gating, and API surface design are all consistent with mds-wasm. The main issues are the missing Cargo.toml workspace metadata fields (a clear convention violation) and the `debug-panics` detail surfacing inconsistency. Both are straightforward fixes.

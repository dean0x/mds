# Architecture Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20

## Issues in Your Changes (BLOCKING)

### HIGH

**Workspace-wide rust-version bump from 1.80 to 1.88 lacks justification** - `Cargo.toml:8`
**Confidence**: 90%
- Problem: The workspace `rust-version` was bumped from 1.80 to 1.88 for all crates (mds-core, mds-cli, mds-wasm, mds-napi) even though only `mds-napi` requires the newer toolchain (napi-rs v3 requirements) and a single `is_none_or` call in `parser.rs` (stabilized in Rust 1.82). This forces all downstream consumers and contributors to upgrade their Rust toolchain to 1.88 even if they only use `mds-core` or `mds-cli`. For a pre-release project with zero users this is low-risk today, but it establishes a precedent of coupling the MSRV of leaf binding crates to the core library. `is_none_or` only requires 1.82, so 1.88 is even higher than needed for that change alone.
- Fix: Consider one of:
  1. Override `rust-version` only in `crates/mds-napi/Cargo.toml` (remove `rust-version.workspace = true`, set `rust-version = "1.88"` locally) and keep the workspace MSRV at 1.82 (the minimum needed for `is_none_or`).
  2. If napi-rs v3 truly requires 1.88, document this in the crate's README or Cargo.toml comments so future maintainers understand the constraint.

**Duplicated options parsing logic between mds-napi and mds-wasm could diverge** - `crates/mds-napi/src/lib.rs:270-395`
**Confidence**: 82%
- Problem: `mds-napi` duplicates the pattern of parsing `serde_json::Map` fields (`vars`, `basePath`, unknown-key rejection) from scratch. `mds-wasm` has its own parallel implementation of the same logic (`parse_vars`, `parse_filename`, `parse_modules`). Both binding crates independently deserialize JS options into `serde_json::Value`, then extract the same conceptual fields (`vars` -> `HashMap<String, Value>`) with custom validation. This is not a DRY violation in the traditional sense -- the two crates have genuinely different option sets (WASM has `filename`/`modules`; NAPI has `basePath`). However, the shared core of `vars` parsing and `Value::from_json` mapping is identical logic duplicated across both. As the project grows, divergent validation behaviors between WASM and NAPI would confuse users.
- Fix: Consider extracting a small shared options-parsing module in `mds-core` (e.g. `mds::options::parse_vars_from_json(map) -> Result<HashMap<String, Value>, MdsError>`) that both binding crates call. This keeps the binding-specific fields (basePath, filename, modules) in their respective crates while sharing the common `vars` extraction. This is not blocking -- it is an architectural suggestion for maintainability as the binding surface grows.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**parser.rs change couples core crate MSRV to napi binding requirements** - `crates/mds-core/src/parser.rs:628`
**Confidence**: 85%
- Problem: The change from `.map_or(true, |p| dot_pos < p)` to `.is_none_or(|p| dot_pos < p)` in `mds-core` is semantically equivalent and more idiomatic, but `is_none_or` was stabilized in Rust 1.82. This change was bundled with the napi PR, making the core parser depend on the MSRV bump. Since `mds-core` is consumed independently by `mds-cli` and `mds-wasm`, a core-only change that raises MSRV should ideally be a separate commit/PR with its own justification.
- Fix: This is already committed and the change is correct. Going forward, MSRV-bumping changes to `mds-core` should be separated from binding-layer PRs to keep the blast radius clear.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Profile override asymmetry: mds-napi uses opt-level=3 while mds-wasm uses opt-level="z"** - `Cargo.toml:49-51` (Confidence: 70%) -- The different optimization strategies are defensible (native addon favors speed; WASM favors size), but a brief comment in workspace `Cargo.toml` explaining the rationale would help future maintainers understand the intentional divergence.

- **Raw N-API unsafe surface area could use safety documentation** - `crates/mds-napi/src/lib.rs:85-134` (Confidence: 65%) -- The three `unsafe fn` helpers (`raw_create_error`, `raw_set_string_prop`, `raw_set_uint32_prop`) are necessary for structured error creation via the raw N-API. The functions themselves look correct, but they lack `// SAFETY:` comments explaining the preconditions (valid `napi_env`, non-null pointers). Adding these annotations would align with Rust community convention for unsafe code review.

- **`#[allow(clippy::needless_pass_by_value)]` is module-wide** - `crates/mds-napi/src/lib.rs:33` (Confidence: 62%) -- The crate-level allow suppresses a useful lint for the entire module. Consider narrowing it to just the napi-exported functions where `String` by-value is required by the napi macro signature, rather than blanket-allowing it for all functions including internal helpers.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Positive Architectural Observations

1. **Correct dependency direction**: `mds-napi` depends on `mds-core` (via the `mds` alias) with no reverse dependency. Core is fully unaware of the binding layer. This is textbook Clean Architecture -- dependencies point inward.

2. **Thin binding layer**: The napi crate contains zero business logic. All compilation, parsing, and validation is delegated to `mds::compile_str_with_deps`, `mds::compile_with_deps`, `mds::check_str_collecting_warnings`, and `mds::check_collecting_warnings`. The binding layer handles only: JS/Rust type marshaling, structured error conversion, panic catching, and input size validation. This is the correct separation of concerns.

3. **Consistent workspace organization**: The new crate follows the established pattern (`crates/mds-{name}/`) with workspace-inherited package metadata, matching how `mds-wasm` and `mds-cli` are structured.

4. **Panic boundary**: The `run_catching` pattern correctly wraps all core calls in `catch_unwind` with `AssertUnwindSafe`, preventing panics from unwinding into the Node.js runtime. This mirrors the `catch_panic` pattern in `mds-wasm`.

5. **Resource limit enforcement**: `MAX_SOURCE_SIZE` re-enforces the same limit as `mds-core` at the binding boundary, correctly noting that string-based entry bypasses the file layer's size check.

6. **Profile override**: The `[profile.release.package.mds-napi]` with `opt-level = 3` is appropriate -- native Node.js addons benefit from speed optimization rather than the size optimization (`opt-level = "z"`) used for WASM.

### Conditions for Approval

1. Consider whether the workspace MSRV bump to 1.88 is intentional for all crates or should be scoped to `mds-napi` only.
2. Document the rationale for 1.88 specifically (vs. 1.82 which is the minimum for `is_none_or`).

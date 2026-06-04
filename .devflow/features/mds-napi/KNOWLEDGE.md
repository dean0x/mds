---
feature: mds-napi
name: MDS Native Node.js Bindings (napi-rs)
description: "Use when adding new exports to the native Node.js addon, changing error codes or error shape, working with N-API raw system calls, updating options parsing, or investigating the panic-safety boundary. Keywords: napi-rs, native addon, node, N-API, napi_create_error, catch_unwind, cdylib, mds-napi, bindings."
category: component-patterns
directories: [crates/mds-napi/]
referencedFiles:
  - crates/mds-napi/src/lib.rs
  - crates/mds-napi/Cargo.toml
  - crates/mds-napi/build.rs
  - crates/mds-napi/package.json
  - crates/mds-napi/__test__/index.spec.mjs
  - crates/mds-napi/__test__/fixtures/simple.mds
  - crates/mds-napi/__test__/fixtures/import_consumer.mds
  - crates/mds-napi/__test__/fixtures/import_provider.mds
  - crates/mds-core/src/lib.rs
  - crates/mds-core/src/options.rs
  - Cargo.toml
created: 2026-05-20
updated: 2026-06-02
---

# MDS Native Node.js Bindings (napi-rs)

## Overview

`mds-napi` is the native Node.js addon for the MDS compiler, compiled as a `cdylib` using napi-rs. It bridges the Rust compiler in `mds-core` into the Node.js runtime by exposing four synchronous functions — `compile`, `compileFile`, `check`, and `checkFile` — with a structured error contract that the JavaScript side can discriminate by error code.

The crate sits at the boundary between Rust and JavaScript. Its primary concerns are: options parsing (converting a JS object to Rust types), error translation (converting `MdsError` into structured JS exceptions with a `code` property), panic safety (`catch_unwind` at every public entry point), and resource limits (re-enforcing `MAX_FILE_SIZE` for string inputs that bypass the file resolver).

## Core Responsibilities

- **Expose** exactly four `#[napi]`-decorated public functions: `compile`, `compileFile`, `check`, `checkFile`.
- **Parse** JS options objects using direct N-API property access (not full serde deserialization of the top-level object), then delegate vars parsing to the shared `mds::parse_json_vars` in `mds-core`.
- **Translate** `mds::MdsError` into a JS `Error` whose `.code` property is set via raw `napi_create_error`, with optional `.help` and `.span` extra properties.
- **Catch panics** using `catch_unwind` wrapped in `run_catching`, converting panics to `mds::internal` coded errors.
- **Enforce** the 10 MiB source size limit on string inputs; file-path inputs inherit the limit from `mds-core`'s resolver.
- **NOT** accept a virtual filesystem — this crate always compiles against the real OS filesystem.

## Standard Structure

All four exported functions follow an identical three-step pattern:

1. **Guard** — check source size (string variants only) or validate that `basePath` is absent (file variants).
2. **Parse options** — validate the JS options object via `parse_compile_opts` or `parse_file_opts`, which use direct N-API property access helpers.
3. **Run catching** — call the corresponding `mds-core` function inside `run_catching`, which wraps the call in `catch_unwind` and maps both `MdsError` and panics to structured JS exceptions.

The two options parsers enforce different allowed key sets:

- `parse_compile_opts` (for `compile` and `check`): accepts `basePath` and `vars`.
- `parse_file_opts` (for `compileFile` and `checkFile`): accepts `vars` only; explicitly rejects `basePath` with a helpful message.

Both parsers call `reject_unknown_napi_keys` to catch unknown keys early, surfacing misspelled option names as `mds::invalid_options` errors rather than silently ignoring them.

## Options Parsing Architecture

Options parsing uses a two-layer approach that avoids deserializing the entire options object through serde:

**Layer 1 — NAPI direct access** (in `mds-napi/src/lib.rs`): Helpers that work directly with the N-API `Object` type to enumerate keys and read individual properties.

- `napi_type_name(vt: ValueType)` — maps a `napi::ValueType` to a human-readable string for error messages.
- `reject_unknown_napi_keys(env, obj, known)` — enumerates all property names via `get_property_names()`, deserializes that Array as JSON, filters out known keys, and reports all unknown keys at once using `format_unknown_keys_error` from `mds-core`.
- `extract_base_path_direct(env, obj)` — reads `basePath` as a typed `Unknown` value, checks its `ValueType`, and returns `None` for absent/null/undefined, `Some(PathBuf)` for valid non-empty strings, or an error for wrong types.
- `extract_vars_direct(env, obj)` — reads `vars` as a typed `Unknown` value; for `ValueType::Object`, deserializes only that sub-value to `serde_json::Value` and delegates to `mds::parse_json_vars`; for other types, errors with `napi_type_name`.

**Layer 2 — Shared JSON utilities** (in `mds-core/src/options.rs`, re-exported from `mds-core`):

- `mds::json_type_name(v: &serde_json::Value)` — type name for JSON values, used in diagnostic messages.
- `mds::parse_json_vars(vars_value: serde_json::Value)` — validates that the value is a JSON object (not array, string, etc.) and converts entries to `HashMap<String, mds::Value>`. Returns `VarsError::InvalidType` for wrong types or `VarsError::Conversion` for values that can't be converted.
- `mds::format_unknown_keys_error(unknowns: &[&str], known: &[&str])` — builds the singular/plural "unknown option key(s)" message. Used by both `reject_unknown_napi_keys` (napi layer) and `reject_unknown_json_keys` (shared JSON layer).
- `mds::reject_unknown_json_keys(map, known)` — validates a `serde_json::Map` against allowed keys. Used by mds-wasm, not by mds-napi (napi uses `reject_unknown_napi_keys` instead).
- `mds::VarsError` — error type returned by `parse_json_vars`, with variants `InvalidType(String)` and `Conversion(MdsError)`.

The key design insight: `extract_vars_direct` only serializes the `vars` sub-value, not the entire options object. This keeps the serde boundary narrow and lets the NAPI layer use typed `ValueType` checks for the top-level keys.

## Dependency Patterns

```toml
# crates/mds-napi/Cargo.toml

[lib]
crate-type = ["cdylib"]    # required for a native .node file

[dependencies]
mds = { package = "mds-core", path = "../mds-core" }
napi = { workspace = true }        # napi3 + serde-json features
napi-derive = { workspace = true } # #[napi] procedural macro
serde_json = { workspace = true }  # used for vars sub-value deserialization

[build-dependencies]
napi-build = { workspace = true }  # generates the module registration boilerplate

[features]
debug-panics = []   # exposes raw panic payload on mds::internal — NEVER enable in production
```

Key points: `napi` is declared at workspace level with `features = ["napi3", "serde-json"]`. The `serde-json` feature enables `env.from_js_value(val)` to deserialize a single JS value to `serde_json::Value`. The workspace-level `[profile.release]` forces `panic = "unwind"` because `catch_unwind` requires the unwind ABI.

The shared options utilities (`parse_json_vars`, `format_unknown_keys_error`, `VarsError`, `json_type_name`, `reject_unknown_json_keys`) are imported from `mds` (the re-export alias for `mds-core`) — not duplicated in this crate.

## Error Handling

### The Error Code Contract

Every JS error thrown by this crate carries a `code` string. The codes defined by `mds-core` (e.g. `mds::syntax`, `mds::undefined_var`, `mds::file_not_found`) pass through unchanged. Three additional codes are synthesised only at the napi boundary:

| Code | Origin | Meaning |
|---|---|---|
| `mds::internal` | napi boundary | Rust panic caught by `catch_unwind` |
| `mds::invalid_options` | napi boundary | Malformed or type-incorrect JS options object |
| `mds::resource_limit` | napi boundary | Source string exceeds 10 MiB |

One additional code originates in `mds-core` and passes through the napi boundary unchanged:

| Code | Origin | Meaning |
|---|---|---|
| `mds::builtin_type_error` | mds-core builtins module | A built-in function was called with an argument of the wrong type |

`MdsError` is `#[non_exhaustive]`, so new error variants can be added to `mds-core` without a breaking change to this crate. The `throw_mds_error` helper maps all `MdsError` variants by code string, so new codes from `mds-core` flow through to JS automatically.

### ArityMismatch Error Message Format

`MdsError::ArityMismatch` now reports argument count requirements as a range when a function accepts a variable number of arguments. The `.message` on the thrown JS error will use the format `"expected 1-3 arguments, got 5"` rather than the previous single-value format `"expected 2 arguments, got 5"`. Code that matches on the error message string (rather than the `.code` property) may need to be updated. Always discriminate on `.code`, not on the message text.

### Why Raw N-API for Error Creation

napi-rs's high-level `napi::Error` type does not support setting the `.code` property on the underlying JS `Error` object. To attach a machine-readable `code`, `help`, and `span`, the crate bypasses napi-rs and calls `napi_create_error` directly via `napi::sys`. The return convention is to call `napi_throw(env, err_obj)` and then return `napi::Error::new(Status::PendingException, "")` — the `PendingException` sentinel tells napi-rs that a JS exception is already pending and it must not create a second one.

The helper functions are structured as follows: `raw_create_error` creates the `Error` with code; `raw_set_string_prop` and `raw_set_uint32_prop` attach extra properties; `throw_mds_error` orchestrates both for `MdsError`; `throw_coded_error` handles the boundary-only codes.

All raw N-API calls are `unsafe`. The invariants are: `env` is valid for the duration of the call (guaranteed by napi-rs), and values are used before any allocating re-entrant call can invalidate them.

### VarsError Mapping

`extract_vars_direct` maps `VarsError` variants to distinct napi errors:

- `VarsError::InvalidType(msg)` → `throw_options_error(env, &msg)` (code `mds::invalid_options`)
- `VarsError::Conversion(mds_err)` → `throw_mds_error(env, mds_err)` (uses the error code from `mds-core`)

### Span Shape

When an `MdsError` carries source location information, `throw_mds_error` creates a `span` JS object with `{ offset: u32, length: u32, line?: u32, column?: u32 }`. Both `line` and `column` are optional (they are `Option<usize>` in the serialized form) and are omitted from the span object when absent. Tests validate the shape in test group `E-8`.

## Integration Guidelines

### Adding a New Exported Function

1. Define the Rust function with `#[napi]` (or `#[napi(js_name = "camelCaseName")]` for non-snake-case names).
2. Accept `env: Env` as the first argument — required for error construction helpers.
3. Apply `run_catching` around any call into `mds-core` to maintain panic safety.
4. For string-source variants, call `check_source_size` before parsing options.
5. Define which option keys are valid. If the set differs from existing parsers, add a new `parse_*_opts` function that calls `reject_unknown_napi_keys`, `extract_base_path_direct`, and/or `extract_vars_direct`.
6. Return types exposed to JS must use `#[napi(object)]`. Struct fields must be `pub`.

### Calling mds-core Functions

The napi layer calls the `*_with_deps` / `*_collecting_warnings` family of functions exclusively (e.g. `mds::compile_str_with_deps`, `mds::compile_with_deps`, `mds::check_str_collecting_warnings`, `mds::check_collecting_warnings`). These return warnings as a `Vec<String>` in the return value rather than printing to stderr, so the addon can surface them in the JS return value. Never call the `emit_warnings` variants from the addon — they write to stderr and the warnings would disappear from the JS caller's perspective.

The public signatures of these functions have NOT changed: string-source variants still accept `Option<&Path>` for `base_dir`, and file-path variants still accept `impl AsRef<Path>`. Internal changes to `resolve_base_dir` (now returns `String`) and `ModuleCache::resolve_path`/`resolve_source` (now take `&str` instead of `&Path`) are transparent to the napi layer — they are private implementation details inside `mds-core/src/lib.rs`.

### Extending Options Parsing

When adding a new option key:

1. Add the key name to the `known` slice passed to `reject_unknown_napi_keys`.
2. Add a new `extract_*_direct` helper following the `extract_base_path_direct` / `extract_vars_direct` pattern: read via `get_named_property_unchecked`, check `get_type()`, handle `Undefined`/`Null` as absent, and error with `napi_type_name(other)` for unexpected types.
3. Do NOT deserialize the full options object through serde — keep the boundary narrow to the specific sub-value being extracted.

## Anti-Patterns

- **Returning `napi::Error::new(Status::GenericFailure, ...)` directly** — this creates a plain JS `Error` without a `.code` property. Always go through `throw_mds_error` or `throw_coded_error` so that consumers can discriminate errors by code.
- **Calling `env.throw_error(msg, code)` on the happy path** — the `env.throw_error` fallback inside `raw_create_error` is only for the rare case where the raw N-API call itself fails (null pointer returned). Use `throw_coded_error` as the primary path.
- **Using the non-`_collecting_warnings` mds-core functions** — those emit warnings to stderr and return `Result<String, MdsError>`, not `(output, warnings)`. Warnings would never reach the JS `result.warnings` array.
- **Enabling the `debug-panics` feature outside local dev** — the raw panic payload leaks absolute filesystem paths from the build machine, which is a security/privacy issue in shipped binaries.
- **Passing `basePath` to `compileFile`/`checkFile`** — the file variants derive their base directory from the file path itself; accepting `basePath` would create ambiguity. The parser explicitly rejects it with a descriptive error message.
- **Forgetting `AssertUnwindSafe`** — closures passed to `run_catching` / `catch_unwind` must be `UnwindSafe`. Closures that capture `String` or `PathBuf` need `AssertUnwindSafe(move || {...})` because those types are not `UnwindSafe` by default.
- **Deserializing the full options object with serde** — the old approach serialized the entire `Object` to `serde_json::Value` then removed known keys. The current approach reads individual properties directly, keeping serde deserialization limited to the `vars` sub-value only.
- **Duplicating `parse_json_vars`, `json_type_name`, or `format_unknown_keys_error` in this crate** — these now live in `mds-core/src/options.rs` and are shared with `mds-wasm`. If you need to change the error message format or vars validation logic, change it there.
- **Matching on error message text instead of `.code`** — message formats are not part of the public contract and can change (e.g. `ArityMismatch` now uses a range format). Always branch on `.code`.

## Gotchas

- **`panic = "unwind"` is workspace-global.** The workspace `Cargo.toml` sets `panic = "unwind"` in both `[profile.dev]` and `[profile.release]` because Cargo does not support per-crate panic strategies within a workspace. This affects every crate in the workspace, not just mds-napi.
- **`null` and `undefined` options are both valid.** The `opts: Option<Object>` napi-rs parameter maps both JS `null` and JS `undefined` to `None`. The test suite explicitly covers both cases (F-C2, F-C3). Do not add special-case handling for `null` — napi-rs handles the coercion.
- **`basePath: null` is silently treated as absent.** Inside `extract_base_path_direct`, `ValueType::Null` is mapped to `None` (same as omitting the key), rather than raising an error. This matches the JS convention where `null` means "not provided".
- **`ValueType::Object` includes JS arrays.** When `extract_vars_direct` sees `ValueType::Object`, it cannot distinguish a plain object from an array at the N-API type level — both report as `Object`. The distinction is made inside `parse_json_vars`: serde deserializes a JS array as `serde_json::Value::Array`, and the `let Value::Object(map) else` guard in `parse_json_vars` rejects it with `VarsError::InvalidType`.
- **Source size limit is re-enforced at the napi boundary.** The `mds-core` resolver enforces `MAX_FILE_SIZE` for file reads. When a caller passes source as a string via `compile` or `check`, the file resolver is bypassed. `check_source_size` re-applies the same limit using `mds::MAX_FILE_SIZE` as the single source of truth, so the limit stays synchronized when `mds-core` changes it.
- **`resolve_base_dir` now returns `String`, not `PathBuf`.** As of the unified backend refactor, the private `resolve_base_dir` helper in `mds-core/src/lib.rs` converts `Option<&Path>` directly to a UTF-8 `String` (failing explicitly on non-UTF-8 paths). `ModuleCache::resolve_path` and `ModuleCache::resolve_source` correspondingly take `&str` for path arguments instead of `&Path`. This is transparent to the napi layer because it calls the stable public wrappers (`compile_with_deps`, etc.), but matters if you read resolver internals.
- **Test runner requires Node.js 22+.** Tests use the built-in `node:test` runner. Running them with Node 18 or 20 will fail with import errors or missing test API features.
- **The built `.node` binary must exist before running tests.** Tests load `../mds-napi.node` directly via `require`. The file is produced by `cargo build --release` plus `napi-rs CLI`. Tests cannot be run from source alone.
- **Dependency paths in `CompileResult` are absolute when using `compileFile`.** The `dependencies` field contains paths as returned by `mds-core`'s module cache, which normalizes them to absolute paths. For `compile` (source string variant), dependencies are also absolute if the provider files are resolved from an absolute `basePath`.
- **`MdsError` is `#[non_exhaustive]`.** New variants (e.g. `BuiltinError` added in the builtins module) do not break the napi layer's `throw_mds_error` — it maps errors by their code string. However, exhaustive match arms on `MdsError` in any future helper code will fail to compile when new variants are added.

## Key Files

- `crates/mds-napi/src/lib.rs` — entire implementation: all four exports, error helpers, options parsers, `run_catching`, size guard.
- `crates/mds-core/src/options.rs` — shared options utilities: `json_type_name`, `parse_json_vars`, `format_unknown_keys_error`, `reject_unknown_json_keys`, `VarsError`. Re-exported from `mds-core` for use by both `mds-napi` and `mds-wasm`.
- `crates/mds-napi/Cargo.toml` — crate manifest; declares `cdylib` type, `debug-panics` feature, and workspace dependency pins.
- `crates/mds-napi/build.rs` — single call to `napi_build::setup()`, generates module registration.
- `crates/mds-napi/package.json` — npm package metadata used by `@napi-rs/cli` for binary distribution.
- `crates/mds-napi/__test__/index.spec.mjs` — integration test suite (Node.js 22+, `node:test`), covers all four functions plus error shape and resource limits.
- `crates/mds-core/src/lib.rs` — public API surface that napi bridges; `compile_with_deps`, `compile_str_with_deps`, `check_collecting_warnings`, `check_str_collecting_warnings` are the four functions called by the addon. Defines `MdsError` (`#[non_exhaustive]`); new variants like `BuiltinError` pass through the napi error boundary automatically via code-string dispatch. Internally, `resolve_base_dir` now returns `String` and `ModuleCache::resolve_path`/`resolve_source` now take `&str` — these are private to `mds-core` and transparent to this crate.
- `Cargo.toml` (workspace) — defines `panic = "unwind"` profiles and workspace-level napi dependency versions.

## Related

- `crates/mds-core/src/options.rs` — defines the shared options utilities imported by this crate. Changes to `parse_json_vars` or `format_unknown_keys_error` affect both mds-napi and mds-wasm.
- `crates/mds-core/src/lib.rs` — defines `MdsError`, `CompileOutput`, `VarsError`, and the `*_collecting_warnings` functions that the addon calls. Internal path representation (private `resolve_base_dir` returning `String`; `ModuleCache` methods taking `&str`) changed in the unified backend refactor but does not affect this crate's call sites.
- `crates/mds-wasm/` — parallel WASM binding for the same compiler; uses `wasm-bindgen` instead of napi-rs but shares the same `mds-core::options` utilities and applies the same `catch_unwind` pattern at the boundary. Compare when making changes that affect both targets.

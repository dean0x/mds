# Security Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`AssertUnwindSafe` wraps closures capturing user-controlled data** - `crates/mds-napi/src/lib.rs:427,461,493,520` (Confidence: 65%) -- The closures passed to `catch_unwind` are wrapped with `AssertUnwindSafe` because they capture owned `String`/`PathBuf`/`HashMap` values which are `UnwindSafe` in practice (no `&mut` aliases, no shared interior mutability). This is the accepted pattern for FFI panic boundaries and is safe here, but each new closure added to `run_catching` should be audited to ensure it does not capture `&mut` references or `Rc`/`Arc<RefCell<_>>` types that could observe inconsistent state after a panic.

- **`raw_create_error` ignores N-API return codes for string creation** - `crates/mds-napi/src/lib.rs:94-106` (Confidence: 70%) -- The return values from `napi_create_string_utf8` and `napi_create_error` are discarded with `let _ = ...`. If any of these calls fail (e.g., out of memory), `err_val` remains null, and the fallback path at line 176 handles it. The fallback is correct, but logging the failure reason (even in debug builds) would improve diagnosability. Not a security vulnerability, but a defense-in-depth observation.

- **`debug-panics` feature gate could leak filesystem paths** - `crates/mds-napi/Cargo.toml:14` (Confidence: 60%) -- The `debug-panics` feature exposes raw panic payloads which may contain absolute filesystem paths. The Cargo.toml comment correctly warns "NEVER enable in production builds." The workspace `Cargo.toml` does not enable it by default, and the release profile strips symbols. However, there is no CI gate or build-time assertion preventing accidental activation. This is documentation-only risk and the existing comment is adequate for the current pre-release state.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### Input Validation

The napi boundary implements thorough input validation:

- **Source size guard** (`check_source_size`, line 243): Enforces `MAX_SOURCE_SIZE` (10 MiB, mirroring `mds::MAX_FILE_SIZE`) before any allocation-heavy compilation. Applied to both `compile` and `check`.
- **Options parsing** (`parse_compile_opts`, `parse_file_opts`): Strict schema validation with unknown-key rejection, type checking for each field, and context-appropriate restrictions (`basePath` rejected on file variants).
- **vars validation** (`parse_vars_field`): Type-checks that `vars` is an object, delegates value conversion to `Value::from_json` which returns `Result`.

### Path Traversal Protection

Path security is delegated to `mds-core`'s `NativeFs`, which provides:

- **Symlink rejection** (`NativeFs::check_symlink`): Canonicalizes parent and full path separately; rejects if they differ (symlink in final component).
- **Project root containment** (`NativeFs::check_path_traversal`): All resolved paths must start with the established project root directory.
- **Null byte rejection**: `VirtualFs::normalize` rejects `\0` in paths; `NativeFs` relies on OS-level rejection via `canonicalize`.
- **Segment count limits**: `MAX_PATH_SEGMENTS = 256` prevents unbounded allocation from adversarial import paths.

The `basePath` parameter sets the resolution root for imports but does not itself allow reading arbitrary files -- it establishes the boundary within which imports are constrained.

### Panic Safety

The `run_catching` function (line 211) wraps all compilation in `catch_unwind`, converting panics to structured `mds::internal` JS errors. Key properties:

- Release builds suppress panic payloads (line 232-233), preventing information leakage.
- `panic = "unwind"` is correctly set in both dev and release profiles (workspace `Cargo.toml` lines 37-42).
- The `debug-panics` feature is opt-in and not enabled by default.

### Raw FFI Safety

The `unsafe` blocks in `raw_create_error`, `raw_set_string_prop`, and `raw_set_uint32_prop` are minimal and correct:

- `napi_create_string_utf8` receives explicit byte lengths from `&str::len()`, so null-termination is not required.
- `CString::new` is used for property name keys passed to `napi_set_named_property` (which requires null-terminated C strings). Failure is handled gracefully with early return.
- All `napi_value` outputs are initialized to `null_mut()` and checked before use.
- The `throw_mds_error` function has a null-check fallback (line 175-178) that uses the safe `env.throw_error` API if raw error creation fails.

### Secrets and Credentials

No hardcoded secrets, API keys, tokens, or credentials found in any changed files. The `.gitignore` additions (`.node`, `node_modules/`, generated `index.js`/`index.d.ts`) are appropriate.

### Resource Limits / DoS

- Source size is capped at 10 MiB at the napi boundary.
- The core library independently enforces file size limits, import depth limits (64), path segment limits (256), and traversal depth limits (256).
- No unbounded loops or allocations introduced in the napi layer.

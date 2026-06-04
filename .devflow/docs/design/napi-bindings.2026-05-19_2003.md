---
title: "Phase 5: napi-rs Native Node.js Bindings"
created: 2026-05-19
status: draft
issue: "#16"
parent: mds-js-ecosystem.2026-05-17.md
scope: single-phase
decisions:
  - AD-N1: Design doc API naming (compile/compileFile/check/checkFile)
  - AD-N2: basePath only for string compilation (no virtual modules)
  - AD-N3: check returns { warnings } matching WASM
  - AD-N4: Crate at crates/mds-napi/ (follows workspace convention)
  - AD-N5: Explicit catch_unwind, not napi-rs default panic handling
  - AD-N6: Custom JS Error via env.throw() + PendingException
---

# Phase 5: napi-rs Native Node.js Bindings

## 1. Goal

New crate `crates/mds-napi/` — thin napi-rs wrapper over mds-core exposing four synchronous functions for Node.js: `compile`, `compileFile`, `check`, `checkFile`.

Scoped to host platform only. Cross-platform CI builds and npm publishing deferred.

## 2. Architecture Decisions

### AD-N1: Design doc API naming

Use `compile(source, opts?)` + `compileFile(path, opts?)` + `check(source, opts?)` + `checkFile(path, opts?)`.

Rationale: Matches WASM binding convention and design doc TypeScript types. Issue #16 used different naming (`compile(path)` + `compileStr(source)`) but WASM already shipped with string-first `compile()`.

### AD-N2: basePath only for string compilation

`compile(source, opts?)` accepts `basePath` option for `@import` resolution via NativeFs. No virtual module maps.

Rationale: napi has real filesystem access. Virtual modules are a WASM-only concept driven by browser constraints. Simpler API surface.

### AD-N3: check returns { warnings }

`check(source, opts?)` and `checkFile(path, opts?)` return `{ warnings: string[] }`.

Rationale: Matches WASM pattern. Uses `check_collecting_warnings` / `check_str_collecting_warnings` from core API. Callers get structured feedback without stderr.

### AD-N4: Crate at crates/mds-napi/

Follows established workspace convention (`crates/mds-core`, `crates/mds-cli`, `crates/mds-wasm`).

Rationale: Design doc specified `bindings/napi/` but WASM landed at `crates/mds-wasm/`. Consistency wins.

### AD-N5: Explicit catch_unwind

Wrap all #[napi] exports in `std::panic::catch_unwind()` to produce structured `mds::internal` errors.

Rationale: napi-rs default panic handling produces generic errors without `code`/`help`/`span` properties. The explicit wrapper mirrors the WASM `catch_panic` pattern and ensures consistent error shapes.

### AD-N6: Custom JS Error via env.throw()

Use `env.throw()` + `Status::PendingException` to throw JS Error objects with `code`, `help`, `span` properties.

Rationale: `napi::Error` is `(Status, String)` — too flat for structured errors. The `env.throw()` pattern constructs a full JS Error object via `env.create_error()`, attaches properties via `set_named_property()`, and throws it.

## 3. API Surface

### TypeScript

```typescript
export interface CompileResult {
  output: string;
  warnings: string[];
  dependencies: string[];
}

export interface CheckResult {
  warnings: string[];
}

export function compile(source: string, opts?: {
  basePath?: string;
  vars?: Record<string, unknown>;
}): CompileResult;

export function compileFile(path: string, opts?: {
  vars?: Record<string, unknown>;
}): CompileResult;

export function check(source: string, opts?: {
  basePath?: string;
  vars?: Record<string, unknown>;
}): CheckResult;

export function checkFile(path: string, opts?: {
  vars?: Record<string, unknown>;
}): CheckResult;
```

### Error Shape

```typescript
interface MdsError extends Error {
  code: string;        // "mds::syntax", "mds::undefined_var", etc.
  help?: string;       // diagnostic help text
  span?: {
    offset: number;    // byte offset into source
    length: number;    // span length in bytes
    line?: number;     // 1-indexed
    column?: number;   // 1-indexed
  };
}
```

### Rust Exports

```rust
#[napi]
pub fn compile(env: Env, source: String, opts: Option<Object>) -> Result<CompileResult>;

#[napi(js_name = "compileFile")]
pub fn compile_file(env: Env, path: String, opts: Option<Object>) -> Result<CompileResult>;

#[napi]
pub fn check(env: Env, source: String, opts: Option<Object>) -> Result<CheckResult>;

#[napi(js_name = "checkFile")]
pub fn check_file(env: Env, path: String, opts: Option<Object>) -> Result<CheckResult>;
```

### Core API Mapping

| napi export | mds-core function | Notes |
|-------------|-------------------|-------|
| `compile(source, opts?)` | `compile_str_with_deps(source, base_dir, vars)` | basePath → base_dir |
| `compileFile(path, opts?)` | `compile_with_deps(path, vars)` | Direct path pass-through |
| `check(source, opts?)` | `check_str_collecting_warnings(source, base_dir, vars)` | Returns ((), warnings) |
| `checkFile(path, opts?)` | `check_collecting_warnings(path, vars)` | Returns ((), warnings) |

## 4. File Changes

### Create

| File | Purpose |
|------|---------|
| `crates/mds-napi/Cargo.toml` | Package manifest with napi deps |
| `crates/mds-napi/build.rs` | napi-build setup |
| `crates/mds-napi/src/lib.rs` | All binding code (~300 lines) |
| `crates/mds-napi/npm/package.json` | npm package metadata |
| `crates/mds-napi/__test__/index.spec.ts` | Node.js integration tests |

### Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add workspace member, napi workspace deps, release profile |
| `.gitignore` | Add napi build artifacts |

## 5. Implementation Order

1. Workspace setup (Cargo.toml, crate scaffold, build.rs)
2. Error helpers (throw_mds_error, options_error, panic handler)
3. Options parsing (parse_compile_opts, parse_file_opts, parse_vars)
4. compileFile — simplest export (path + vars → compile_with_deps)
5. compile — string source + basePath + vars → compile_str_with_deps
6. checkFile / check — validation variants
7. Rust unit tests
8. Build verification — napi build, load in Node.js 18+
9. Node.js integration tests — full API contract

## 6. Error Handling Strategy

### MdsError conversion

```rust
fn throw_mds_error(env: &Env, err: mds::MdsError) -> napi::Error {
    let serialized = err.serialize();
    // Create JS Error, set code/help/span properties, throw via env.throw()
    // Return Status::PendingException
    // Fallback to napi::Error::from_reason() if object creation fails
}
```

### Panic handling

```rust
fn catch_panic<F, T>(f: F) -> Result<T, PanicPayload>
where F: FnOnce() -> Result<T, mds::MdsError> + std::panic::UnwindSafe
{
    // catch_unwind returns Result<Result<T, MdsError>, Box<dyn Any>>
    // Outer: panic → mds::internal error
    // Inner: MdsError → structured error with code/help/span
}
```

Key: `env` must NOT be captured inside `catch_unwind` (not UnwindSafe). Return `Result<T, MdsError>` from closure, convert to JS error outside.

### Resource limits

`check_source_size(source)` for string-based functions. File-based functions rely on NativeFs MAX_FILE_SIZE.

### Options validation

Reject unknown keys with `mds::invalid_options` error. Validate vars types (JSON-compatible only).

## 7. Test Strategy

### Rust unit tests (no Node.js required)

- Options parsing: defaults, basePath, unknown keys, vars type validation
- Resource limits: oversized source rejected
- Error serialization: code, help, span fields

### Node.js integration tests

- Compile: basic source, with vars, with frontmatter, with basePath
- CompileFile: real file, with vars, nonexistent file error
- Check/CheckFile: valid source, undefined var error
- Error shape: code/help/span properties present
- Edge cases: no options, empty options, unknown options
- Thread safety: concurrent worker thread compilation

### Parity

Use existing fixtures from `crates/mds-cli/tests/fixtures/` for output comparison.

## 8. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| napi error object complexity | Medium | env.throw() + PendingException — well-documented pattern |
| Env lifetime in catch_unwind | High | Return Result from closure, convert outside |
| napi v3 MSRV compatibility | Low | Verify against Rust 1.80 requirement |
| Cross-platform CI | Out of scope | Phase 5 = host platform only |

## 9. Gap Analysis Summary

### Resolved (blocking)

- API naming → design doc convention
- Crate location → crates/mds-napi/
- Check return → { warnings }
- String API → basePath only

### Incorporated (should-address)

- Panic handling → explicit catch_unwind
- Error conversion → custom JS Error via env.throw()
- Resource limits → MAX_SOURCE_SIZE at boundary
- Workspace deps → napi/napi-derive in workspace
- Release profile → opt-level=3 for native speed
- Vars type validation → JSON-compatible only
- debug-panics feature flag → mirrors WASM

### Design review notes

- No anti-patterns detected (N+1, god function, parallelism, caching, decomposition)
- One refinement: error conversion fallback on OOM → degrade to napi::Error::from_reason()

## 10. PR Description Guidance

**Problem Being Solved:** MDS has WASM bindings for browsers but no native Node.js bindings. Node.js build tools (Vite, Webpack) and the DevFlow CLI need high-performance native compilation without the overhead of WASM instantiation or CLI subprocess spawning.

**Key Changes to Highlight:**
- Four-function API matching WASM naming convention
- Structured JS errors with code/help/span matching WASM error shape
- Panic handling, resource limits, strict options validation

**Breaking Changes:** None. New crate, no existing API changes.

**Reviewer Focus Areas:**
- Error conversion pattern (env.throw + PendingException) — correctness
- catch_unwind/Env lifetime interaction — safety
- Options parsing strictness — parity with WASM

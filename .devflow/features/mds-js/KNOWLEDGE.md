---
feature: mds-js
name: MDS JavaScript Package (@mds/mds)
description: "Use when working on the @mds/mds JavaScript package. Keywords: MdsBaseBackend, MdsNodeBackend, MdsBackend, initWasmNode, initWasmBrowser, createWasmBackend, createNativeBackend, browser entry, node entry, module-scanner, buildModulesMap, findProjectRoot, normalizeVirtualKey, wrapWithFileOps, O_NOFOLLOW, TOCTOU, WasmModule, validateWasmShape, init, compileFile, checkFile, circuit breaker, promise dedup, MDS_BACKEND, forceBackend, ensureBackend, LazyInit, loadNativeBackend, loadWasmNodeBackend, varsOpt, fileOpts, compileOpts, cross-directory imports, project root discovery, .git marker, .mdsroot marker, entryFilename, relative path key, DEFAULT_MAX_MODULES, DEFAULT_MAX_AGGREGATE_SIZE, MAX_IMPORT_DEPTH, MAX_TRAVERSAL_DEPTH."
category: component-patterns
directories: [packages/mds/]
referencedFiles:
  - packages/mds/src/types.ts
  - packages/mds/src/index.ts
  - packages/mds/src/node.ts
  - packages/mds/src/browser.ts
  - packages/mds/src/backend/wasm.ts
  - packages/mds/src/backend/native.ts
  - packages/mds/src/util/module-scanner.ts
  - packages/mds/src/util/options.ts
  - packages/mds/package.json
  - packages/mds/__test__/scanner.spec.mjs
created: 2026-05-27
updated: 2026-06-01
---

# MDS JavaScript Package (@mds/mds)

## Overview

`@mds/mds` is the universal JavaScript package for the MDS compiler. It provides two entry points:
- **`dist/node.js`** — Node.js environments; tries the native (napi-rs) backend first, falls back to WASM
- **`dist/browser.js`** — browser/edge environments; WASM-only, no file operations

The package conditionally selects the backend using Node.js `package.json` exports conditions (`"node"` / `"default"`).

## Type Hierarchy

All types are defined in `packages/mds/src/types.ts`:

| Type | Description |
|---|---|
| `MdsBaseBackend` | Browser-safe interface: `compile`, `check`, `getBackend` |
| `MdsNodeBackend` | Extends `MdsBaseBackend` with `compileFile`, `checkFile` |
| `MdsBackend` | Deprecated alias for `MdsNodeBackend` |
| `CompileResult` | `{ output: string; warnings: string[]; dependencies: string[] }` |
| `CheckResult` | `{ warnings: string[] }` |
| `CompileOptions` | `{ vars?: Record<string, unknown> }` |
| `FileOptions` | `{ vars?: Record<string, unknown> }` |
| `InitOptions` | `{ wasmUrl?: string | URL | Response | BufferSource }` |
| `BackendType` | `'native' | 'wasm'` |
| `MdsError` | Extends `Error` with `code: string`, `help?: string`, `span?: MdsErrorSpan` |
| `MdsErrorSpan` | `{ offset, length, line?, column? }` — byte-based, 1-indexed line/column |
| `WasmModule` | WASM module shape: `compile`, `check`, `scanImports`, optional `default` |

**`isMdsError(err: unknown): err is MdsError`** — type guard. Returns `true` when `err` is an `Error` with a `.code` string starting with `'mds::'`.

## Node.js Entry (`packages/mds/src/node.ts`)

### Backend Selection

`MDS_BACKEND` env var controls backend selection:
- `'native'` — force native (napi-rs) backend; error if unavailable
- `'wasm'` — force WASM backend; skip native probe
- Any other value — warning emitted, treated as unset
- Unset (default) — try native first, fall back to WASM

`ensureBackend(options?)` is the single source of truth for backend initialization. It deduplicates concurrent `init()` calls by caching the in-flight `Promise<void>`.

### init() Contract

`init(options?: InitOptions): Promise<void>` — must be called and awaited before any other function. Idempotent: subsequent calls resolve immediately once the backend is set. Concurrent calls share one promise (no double-initialization race).

### File Operations

`wrapWithFileOps(base: MdsBaseBackend, wasmModule: WasmModule): MdsNodeBackend` — wraps a browser-safe backend with `compileFile`/`checkFile` that:
1. Call `buildModulesMap(path, wasmModule.scanImports)` to resolve all transitive imports into a flat `Record<string, string>`.
2. Extract the entry source from the modules map (keyed by `entryFilename`).
3. Call `wasm.compile({ filename: entryFilename, modules: remainingModules, vars? })` or `wasm.check(...)`.

The native backend's `compileFile`/`checkFile` are synchronous wrappers directly over the napi addon — no modules map needed.

### Backend Loaders

- `loadNativeBackend()` — dynamic `require('mds-napi')` wrapped in try/catch; returns `{ backend, error: null }` on success or `{ backend: null, error }` on failure. Never throws.
- `loadWasmNodeBackend(options?)` — calls `initWasmNode(options)` then `createWasmBackend(module)` then `wrapWithFileOps`. Always returns a `MdsNodeBackend`. Throws if WASM module cannot be loaded.

### Test Utilities

- `_resetForTesting()` — clears `backend` and `initPromise`. FOR TESTING ONLY.

## Browser Entry (`packages/mds/src/browser.ts`)

Browser entry exposes only `compile`, `check`, `getBackend`, and `init`. No file operations.

`init(options?: InitOptions)` calls `initWasmBrowser(options)`, caches in `resolvedBackend`. Uses a separate `initVoidPromise` for concurrent-call deduplication. On rejection, `initVoidPromise` is reset to `null` (cleared) so the next call retries; `resolvedBackend` is never cleared once set.

### Test Utilities (browser)

- `_resetForTesting()` — clears both `resolvedBackend` and `initVoidPromise`.
- `_initWithModuleForTesting(mod: WasmModule)` — injects a pre-loaded module, bypassing `initWasmBrowser()`. Allows Node.js test suites to exercise the browser API surface.

## WASM Backend (`packages/mds/src/backend/wasm.ts`)

### WasmModule Shape

```typescript
interface WasmModule {
  compile(source: string, options?: { filename?: string; modules?: Record<string, string>; vars?: Record<string, unknown> }): CompileResult;
  check(source: string, options?: { filename?: string; modules?: Record<string, string>; vars?: Record<string, unknown> }): CheckResult;
  scanImports(source: string): string[];
  default?: (input?: unknown) => Promise<void>;
}
```

### Circuit Breaker Pattern

Both `initWasmNode` and `initWasmBrowser` implement a circuit breaker:
- `MAX_INIT_RETRIES = 3` (Node.js) / `MAX_BROWSER_RETRIES = 3` (browser)
- On failure: increment failure counter, clear cached promise (so next call retries)
- After exhaustion: every subsequent call throws immediately without re-attempting

`nodeFailures` and `browserFailures` are module-level counters. `_resetForTesting(failures?, browserFailuresCount?)` pre-seeds them for exhaustion path testing.

### Node.js WASM Initialization

`initWasmNode(options?)` deduplicates via `cachedNodePromise`. On first call:
1. Defers `require('node:module')` import to the async function body (browser-safe).
2. Calls `_initNode(options)` which iterates candidate paths via `tryLoadCandidate`.
3. On success, validates shape via `validateWasmShape`.

`tryLoadCandidate(candidate, require, wasmUrl)`:
- Returns `null` for `MODULE_NOT_FOUND` errors.
- Throws for shape validation failures or unexpected errors.
- Re-throws non-not-found errors so the caller can surface them.

`isModuleNotFound(err)` — detects `MODULE_NOT_FOUND` / `ERR_MODULE_NOT_FOUND` error codes.

`validateWasmShape(mod: unknown): asserts mod is WasmModule` — exported; checks `compile`, `check`, `scanImports` are all present as functions. Throws a descriptive error naming the first missing member.

### Browser WASM Initialization

`initWasmBrowser(options?)` — no candidate list; calls `_initBrowser(options)` which dynamically imports the WASM module and calls its `default` initializer with `wasmUrl`. Simpler than Node.js — exhaustion means the `wasmUrl` itself is wrong.

### Options Builders

- `compileOpts(options?)` — merges `filename: 'input.mds'`, frozen empty `modules`, and optional `vars`. Returns a frozen object to prevent WASM FFI mutation of shared state.
- `fileOpts(entryFilename, modules, options?)` — for file operations; uses the real `entryFilename` and the resolved `modules` map, plus optional `vars`. Exported for use in `node.ts`.
- `DEFAULT_COMPILE_OPTS` — deep-frozen default object for the no-vars path; both outer object and `modules` are frozen.

### Factory

`createWasmBackend(wasmModule: WasmModule): MdsBaseBackend` — synchronous factory; mirrors `createNativeBackend(addon)` pattern. Returns `compile`, `check`, `getBackend` (always `'wasm'`) without file operations. File operations are added by `wrapWithFileOps` in `node.ts`.

### Test Utilities (wasm)

`_resetForTesting(failures?, browserFailuresCount?)` — full reset including both counter pre-seeding slots. Exported.

## Native Backend (`packages/mds/src/backend/native.ts`)

`createNativeBackend(addon: NapiAddon): MdsNodeBackend` — synchronous factory. The addon is injected (not imported directly) for testability. Returns `compile`, `check`, `compileFile` (sync), `checkFile` (sync), `getBackend` (always `'native'`).

`NapiAddon` interface documents the napi surface:
- `compile(source, opts?)` / `check(source, opts?)` — accept `{ basePath?, vars? }`
- `compileFile(path, opts?)` / `checkFile(path, opts?)` — accept `{ vars? }` only

`varsOpt(options?)` from `../util/options.ts` builds `{ vars }` only when `options.vars` is defined and non-null. Used by both native and WASM backends to keep the options shape minimal.

## Module Scanner (`packages/mds/src/util/module-scanner.ts`)

### Project Root Discovery

**`findProjectRoot(start: string): string`** — exported function. Walks up from `start` directory looking for `.git` or `.mdsroot` markers (same as Rust's `NativeFs::find_project_root`). Falls back to `start` if no marker found within `MAX_TRAVERSAL_DEPTH = 256` directories.

This determines the boundary for path traversal security and the base for computing virtual module keys (relative paths from project root).

### Key Design: entryFilename is now a relative path

As of the project root discovery update, `buildModulesMap` computes:
- `projectRoot = findProjectRoot(dirname(absoluteEntry))` — discovered via `.git`/`.mdsroot` markers
- `entryFilename = relative(projectRoot, absoluteEntry)` — path from project root, not just `basename`

**Consequence**: `entryFilename` is now a path like `packages/mds/__test__/fixtures/imports/entry.mds`, not just `entry.mds`. Tests must use `endsWith` checks rather than equality when verifying `entryFilename`.

**Why this matters**: Cross-directory imports (e.g. `../lib/helpers.mds` from `app/entry.mds` to a sibling directory `lib/`) require `projectRoot` to be the common ancestor, not just `dirname(entry)`. Without project root discovery, the sibling directory import would be rejected as escaping the project root.

### `normalizeVirtualKey(base: string, relative: string): string`

Exported function. Mirrors Rust's `VirtualFs::normalize()` exactly. Converts a relative import path to a canonical slash-separated virtual key:
- `base = ''` (root entry): uses `relative` as-is (no parent resolution)
- `base != ''`: resolves `relative` against the directory of `base`
- `..` segments are allowed up to the project root; escaping throws `'import path escapes project directory'`
- Empty path throws `'import path is empty'`
- Null byte throws `'import path contains null byte'`
- Path with >256 segments throws segment-count error

**MUST exactly mirror the Rust implementation** — any divergence causes import resolution mismatches between the TypeScript scanner and the Rust WASM module.

### `buildModulesMap(entryPath, scanImports, options?): Promise<BuildModulesMapResult>`

Recursively resolves an MDS entry file and all its transitive imports into a flat modules map.

**Returns**: `{ entryFilename: string; modules: Record<string, string> }`

The `modules` map includes the entry file keyed by `entryFilename`. Callers that pass `modules` to WASM `compile`/`check` **MUST** extract and remove the entry source before the call — leaving the entry key present causes `mds::filename_collision` because WASM also inserts the entry source under `filename`.

**Security checks** (in order):
1. `validateImportPath(importPath, absoluteDir)` — rejects null bytes, empty paths, and paths escaping `projectRoot`
2. `openAndValidateModule(absolutePath)` — security perimeter:
   - `openNoFollow` — O_NOFOLLOW | O_RDONLY; `ELOOP`/`ENOTDIR` → security error about symlink
   - `handle.stat().isFile()` — rejects non-regular files (directories, devices, etc.)
   - `realpath` check — on platforms where O_NOFOLLOW=0 (Windows), compares resolved path; mismatch → security error
3. Aggregate size checked **before** reading content (via `fstat` size from the opened handle) to prevent forced allocation of content that will be rejected

**Resource limits**:
- `maxModules` (default: `DEFAULT_MAX_MODULES = 256`) — checked immediately after `visited.add()`
- `maxAggregateSize` (default: `DEFAULT_MAX_AGGREGATE_SIZE = 10 MiB`) — checked before `readFile`
- `MAX_IMPORT_DEPTH = 64` — explicit depth parameter on `scan()`, enforced before recursing

**Parallelism**: Child imports at each level are resolved with `Promise.all(importPaths.map(...))`. Aggregate size increments are safe because JS is single-threaded.

**`scan(absolutePath, virtualKey, depth)` internal function**: Marks visited before recursing (prevents duplicate reads), opens via `openAndValidateModule`, checks limits, reads content, calls `scanImports(content)` to get import paths, resolves children in parallel.

### Security Architecture

| Threat | Defense |
|---|---|
| Symlink traversal | `O_NOFOLLOW` (Linux/macOS) or post-open `realpath` comparison (Windows) |
| Path traversal above project root | `projectRoot` prefix check in `validateImportPath` and `openAndValidateModule` |
| Circular imports | `visited: Set<string>` of absolute paths |
| Import chain depth | `depth` parameter with `MAX_IMPORT_DEPTH = 64` guard |
| Excessive modules | `maxModules` guard after `visited.add()` |
| Excessive memory | `maxAggregateSize` checked before `readFile` via fstat |
| Non-regular files | `handle.stat().isFile()` check |
| Filesystem root as project root | Explicit rejection: `projectRoot === '/'` throws |
| Non-UTF-8 paths | Node.js handles UTF-8 natively; `node:path` functions work on string representations |

## Options Utility (`packages/mds/src/util/options.ts`)

`varsOpt(options?)` — returns `{ vars: Record<string, unknown> }` when `options.vars` is defined and non-null, `undefined` otherwise. Used by both `native.ts` and (indirectly) `wasm.ts` to avoid creating unnecessary option objects.

## Package Configuration

`packages/mds/package.json`:
- `"type": "module"` — ESM-only
- `"engines": { "node": ">=22.0.0" }` — requires Node 22+ (uses `node:test` runner and modern ESM)
- Exports: `"."` → `"node"` condition → `dist/node.js`; `"default"` → `dist/browser.js`
- `"optionalDependencies": { "mds-napi": "file:../../crates/mds-napi" }` — native addon is optional (WASM fallback if missing)
- Scripts: `test` (all tests), `test:native` (native backend only), `test:perf` (benchmarks)
- Build: `tsc -p tsconfig.json` → `dist/`

## Test Suite (`packages/mds/__test__/`)

Tests use Node.js built-in `node:test` runner. All tests require the built `dist/` output.

| File | Tests | Scope |
|---|---|---|
| `compile.spec.mjs` | U-C1–U-C9 | `compile()` behavior |
| `check.spec.mjs` | U-K1–U-KF3 | `check()` and `checkFile()` |
| `compileFile.spec.mjs` | U-CF1–U-CF9 | `compileFile()` behavior |
| `wasm-compileFile.spec.mjs` | U-WCF1–U-WCF11 | WASM backend file ops (subprocess isolation) |
| `error.spec.mjs` | U-E1–U-E9 | Error shape: `code`, `help`, `span`, `isMdsError` |
| `backend.spec.mjs` | U-B1–U-B11 | Backend selection, MDS_BACKEND env, getBackend |
| `browser.spec.mjs` | U-BR1–U-BR13 | Browser entry pre/post-init, promise dedup, retry reset |
| `wasm-backend.spec.mjs` | U-WB1–U-WB20 | Circuit breaker, browser circuit breaker, shape validation |
| `native-backend.spec.mjs` | U-N1–U-N6 | Native backend isolation via `createNativeBackend` |
| `scanner.spec.mjs` | U-S1–U-S10, U-SM1–U-SM8 | `normalizeVirtualKey` and `buildModulesMap` |
| `perf.spec.mjs` | U-PF1–U-PF5 | Performance (no strict timing assertions) |

`helpers.mjs` exports shared fixture paths: `FIXTURES`, `SIMPLE_MDS`, `IMPORT_PROVIDER_MDS`, `IMPORT_CONSUMER_MDS`, `ENTRY_MDS`, `EMPTY_MDS`, `FRONTMATTER_ONLY_MDS`, `MD_EXTENSION`.

`wasm-compileFile.spec.mjs` uses subprocess isolation via `execFile` to prevent cross-test contamination of the module-level backend singleton. `wasmEnv()` / `nativeEnv()` build environment overrides. The `runScript(script, env)` helper spawns an inline ESM script and parses its JSON stdout.

### Scanner Test Fixtures

- `fixtures/imports/` — multi-file import chain: `entry.mds` → `lib.mds` → `deep.mds` (3+ modules)
- `fixtures/simple.mds` — single file with no imports
- `fixtures/cross-dir/app/entry.mds` — imports `../lib/helpers.mds` (sibling directory cross-dir test, U-SM8)
- `fixtures/cross-dir/lib/helpers.mds` — sibling directory module
- `fixtures/edge/` — edge cases: `empty.mds`, `frontmatter_only.mds`, `md_extension.md`

## Integration Guidelines

### Adding a New Public Function to Both Node and Browser

1. Add the function signature to `types.ts` (`MdsBaseBackend` for browser-safe, `MdsNodeBackend` for node-only).
2. Add the implementation to `browser.ts` and `node.ts`.
3. Add the call through `assertReady()` (which returns the backend) on both entry points.
4. Export from `index.ts` if it should be re-exported.
5. Add `createNativeBackend` / `createWasmBackend` support in `native.ts` / `wasm.ts`.

### Using `findProjectRoot` for Path Computation

Any code that computes a virtual module key relative to a project boundary should:
1. Call `findProjectRoot(dirname(absolutePath))` to discover the boundary.
2. Use `relative(projectRoot, absolutePath)` to compute the virtual key.
3. Reject `projectRoot === '/'` as a filesystem root guard.

### Extending `buildModulesMap` Resource Limits

Add new limit constants at module scope (named `MAX_*`). Check limits at the earliest possible point — before I/O for size limits, after `visited.add()` for count limits. Document the limit in `ModuleScannerOptions` interface.

## Anti-Patterns

- **Calling `compile`/`check`/`compileFile`/`checkFile` before `init()`** — throws `'MDS backend not initialized'`. Always `await init()` first.
- **Passing `modules` that still contains the entry source to WASM `compile`** — causes `mds::filename_collision`. Extract and remove `modules[entryFilename]` before passing `modules` to the WASM call.
- **Hardcoding `entryFilename === basename(path)`** — `entryFilename` is now `relative(projectRoot, absolutePath)`, which may include subdirectory segments. Use `endsWith` checks in tests and callers.
- **Using `createNativeBackend` with a direct `require('mds-napi')`** — the addon is injected for testability; direct require creates coupling that breaks test isolation.
- **Importing `buildModulesMap` in browser-side code** — it uses `node:fs/promises` and `node:path`; safe only in Node.js environments.
- **Comparing `nodeFailures >= MAX_INIT_RETRIES` in tests using a literal** — mirror `MAX_INIT_RETRIES` as a constant in the test file so that drift surfaces as a test failure rather than a silently wrong threshold.
- **Mutating the `DEFAULT_COMPILE_OPTS` object** — it's deep-frozen; attempting mutation in strict mode throws. Build a new options object via `compileOpts(options)` instead.
- **Using `file:` links in production code paths** — `mds-napi` is `file:` linked in `optionalDependencies` for local dev/CI; production consumers install the published npm package.

## Gotchas

- **`entryFilename` includes directory segments** — since project root discovery was added, `entryFilename` is `relative(projectRoot, absoluteEntry)` not `basename(absoluteEntry)`. A file at `packages/mds/__test__/fixtures/imports/entry.mds` in a repo with `.git` at `/Users/dean/Sandbox/mdl/` will have `entryFilename = 'packages/mds/__test__/fixtures/imports/entry.mds'`.
- **`findProjectRoot` falls back to the starting directory** — if no `.git` or `.mdsroot` marker is found within `MAX_TRAVERSAL_DEPTH` parents, `findProjectRoot` returns `start`. This means files outside any recognized project boundary use their own directory as root, same as the previous behavior.
- **WASM `modules` map MUST NOT include the entry** — the WASM `compile`/`check` functions take `{ filename, modules, vars? }` where `filename` is already the entry. If `modules[filename]` also exists, the WASM backend raises `mds::filename_collision`. Remove the entry key before passing `modules`.
- **Circuit breaker is per-process** — `nodeFailures` and `browserFailures` are module-level singletons. Multiple tests in the same process share failure state. Use `_resetForTesting()` between tests.
- **`initWasmNode` defers `node:module` import** — the `import('node:module')` is inside the async function body, not at module scope. This keeps `wasm.ts` importable in browser environments (where `node:module` is unavailable).
- **`O_NOFOLLOW = 0` on Windows** — the symlink guard falls back to a post-open `realpath` comparison. This is a race-condition window (the symlink could be created between `open` and `realpath`), but it's the best available on Windows.
- **`aggregate size check uses fstat, not file content length`** — `aggregateSize += fileSize` uses `stats.size` from `handle.stat()` before `readFile`. The actual UTF-8 decoded content may differ slightly from the byte count on some systems, but this is conservative and acceptable.
- **Subprocess isolation for WASM compileFile tests** — `wasm-compileFile.spec.mjs` spawns subprocesses to prevent the module-level `backend` singleton from being contaminated across test cases. Each subprocess gets a fresh module instance.
- **Node 22+ required** — tests use `node:test` built-in runner; `node:fs/promises` features require Node 22+. Running with Node 18 or 20 will fail.
- **`dist/` must be built before tests** — tests import from `../dist/`. Run `npm run build` in `packages/mds/` before running tests.

## Related

- `crates/mds-napi/` — the native Node.js addon that `loadNativeBackend()` dynamically loads. Changes to the napi error shape (`.code`, `.span`) or export signatures affect this package.
- `crates/mds-wasm/` — the WASM module that `initWasmNode`/`initWasmBrowser` loads. Changes to the `WasmModule` shape (especially `scanImports`) affect the module scanner and WASM backend.
- `crates/mds-core/src/fs.rs` — `VirtualFs::normalize()` must stay in sync with `normalizeVirtualKey()` in `module-scanner.ts`. Any change to how the Rust resolver normalizes import paths must be mirrored here.
- `packages/bundler-utils/` — exports `LazyInit<T>` used by Vite/Webpack/Rollup plugins for single-init guarantee and concurrent-call deduplication. Shares the same pattern as `ensureBackend`.

---
title: MDS JavaScript/TypeScript Ecosystem
created: 2026-05-17
status: accepted
scope: multi-phase
phases: 7
tracks: [core-abstraction, wasm-bindings, napi-bindings, js-packages, bundler-plugins]
decisions:
  - AD-1: Box<dyn FileSystem> over generics (avoid API infection)
  - AD-2: String keys for ModuleCache (not PathBuf)
  - AD-3: Security as property of FileSystem implementation
  - AD-4: Pre-bundled module map for WASM @import
  - AD-5: Separate crates for wasm and napi bindings
  - AD-6: Universal JS package with runtime backend detection
  - AD-7: YAML accepted in source, vars as JSON only in WASM
  - AD-8: Lockstep versioning across all @mds/ packages
---

# MDS JavaScript/TypeScript Ecosystem

## 1. Goal and Scope

Transform the MDS Rust compiler from a single-crate CLI tool into a multi-target platform supporting:
- WebAssembly (browser, edge, universal)
- Native Node.js bindings (napi-rs, high performance)
- JavaScript bundler plugins (Vite, Webpack, Rollup)
- Integration with the DevFlow TypeScript CLI

**In scope:**
- Cargo workspace restructuring (mds-core library + mds-cli binary)
- FileSystem trait abstraction (native + virtual/WASM)
- WASM bindings via wasm-bindgen
- napi-rs bindings for native Node.js
- Universal JS wrapper (@mds/mds) auto-selecting backend
- Shared bundler utilities (@mds/bundler-utils)
- Vite, Webpack, and Rollup plugins
- Error serialization for JS consumption
- Dependency graph exposure for HMR

**Out of scope (deferred to v0.2+):**
- Source maps
- Language server protocol (LSP)
- DevFlow CLI direct integration (wired after npm packages publish)

## 2. Architecture

```
+-----------------------------------------------------+
|  @mds/vite-plugin  @mds/webpack-loader  @mds/rollup |  <- Bundler plugins
+-----------------------------------------------------+
|              @mds/bundler-utils                      |  <- Shared transform logic
+-----------------------------------------------------+
|                    @mds/mds                          |  <- Universal JS API
+------------------------+----------------------------+
|   WASM binding         |       napi-rs binding      |  <- Rust->JS bridge
+------------------------+----------------------------+
|                   mds-core                          |  <- Rust compiler library
|          +-------------------------+                |
|          |  trait FileSystem       |                |
|          |  +-- NativeFs (real OS) |                |
|          |  +-- VirtualFs (HashMap)|                |
|          +-------------------------+                |
+-----------------------------------------------------+
|                   mds-cli                           |  <- CLI binary (unchanged)
+-----------------------------------------------------+
```

### Target Directory Structure

```
mdl/
├── Cargo.toml              (workspace root)
├── crates/
│   ├── mds-core/           (compiler library)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── fs.rs       (FileSystem trait + NativeFs + VirtualFs)
│   │       ├── resolver.rs (refactored: uses dyn FileSystem)
│   │       ├── error.rs    (+ SerializedError)
│   │       └── ...
│   └── mds-cli/            (binary)
│       ├── Cargo.toml
│       └── src/main.rs
├── bindings/
│   ├── wasm/               (wasm-bindgen, crate-type cdylib)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── napi/               (napi-rs native addon)
│       ├── Cargo.toml
│       ├── package.json
│       └── src/lib.rs
├── packages/
│   ├── mds/                (@mds/mds — universal JS API)
│   │   ├── package.json
│   │   ├── src/index.ts
│   │   ├── src/types.ts
│   │   ├── src/wasm.ts
│   │   └── src/native.ts
│   ├── bundler-utils/      (@mds/bundler-utils)
│   │   ├── package.json
│   │   └── src/index.ts
│   ├── vite-plugin/        (@mds/vite-plugin)
│   │   ├── package.json
│   │   └── src/index.ts
│   ├── webpack-loader/     (@mds/webpack-loader)
│   │   ├── package.json
│   │   └── src/index.ts
│   └── rollup-plugin/      (@mds/rollup-plugin)
│       ├── package.json
│       └── src/index.ts
├── package.json            (npm workspace root)
├── tsconfig.base.json
└── tests/
    └── corpus/             (shared test fixtures)
```

## 3. Architecture Decisions

### AD-1: Box<dyn FileSystem> over generics

Use `Box<dyn FileSystem>` as a member of `ModuleCache` rather than `ModuleCache<F: FileSystem>`.

Rationale: Generics would infect the entire public API surface. With dyn, the WASM binding injects VirtualFs and native injects NativeFs without type parameter propagation. Vtable cost is negligible vs compilation work.

### AD-2: String keys for ModuleCache

`HashMap<String, Arc<ResolvedModule>>` and `IndexSet<String>` for cycle detection.

Rationale: PathBuf is meaningless in WASM. Strings serve as abstract module identifiers. NativeFs::normalize() returns canonical path as String. VirtualFs returns normalized virtual path key.

### AD-3: Security as property of FileSystem implementation

Symlink checks, path traversal prevention, and project root discovery live inside `NativeFs::normalize()`. `VirtualFs::normalize()` is pure string manipulation — security comes from the closed key-space.

Rationale: Core compiler stays security-agnostic. Native FS handles OS-level security. Virtual FS is inherently contained (can only access pre-loaded keys).

### AD-4: Pre-bundled module map for WASM @import

WASM users pass `Record<string, string>` (path -> source) at compile time. VirtualFs is populated from this map.

Rationale: WASM cannot do filesystem I/O. Bundler plugins already know which files exist. Avoids async callbacks from Rust to JS.

### AD-5: Separate binding crates

`bindings/wasm/` and `bindings/napi/` are independent Cargo crates depending on mds-core.

Rationale: wasm-pack and napi-rs have incompatible build targets. Each binding is a thin adapter.

### AD-6: Universal JS package with runtime detection

`@mds/mds` detects environment and exports uniform API. Node.js with native -> napi. Browser/edge/fallback -> WASM.

Rationale: Consumers don't care about binding mechanism. One import, one API.

### AD-7: YAML in source, JSON vars in WASM

WASM accepts YAML frontmatter in .mds source (it's part of the format) but `vars` option accepts only JSON to avoid bundling serde_yml twice.

Rationale: Frontmatter parsing is part of the compiler core (unavoidable). Runtime vars are caller-provided data — JSON is sufficient.

### AD-8: Lockstep versioning

All @mds/ packages share the same semver version as the Rust crate.

Rationale: Tightly coupled (bindings built from same source). Simplifies compatibility matrix in early versions.

## 4. FileSystem Trait Design

```rust
pub trait FileSystem: Send + Sync {
    /// Normalize a module specifier relative to a base key.
    /// NativeFs: canonicalize + symlink/traversal checks, returns canonical path as String.
    /// VirtualFs: pure string path join with ../ handling, /-separated.
    fn normalize(&self, base: &str, relative: &str) -> Result<String, MdsError>;

    /// Read the content of a normalized module key.
    /// NativeFs: std::fs::read + size limit check.
    /// VirtualFs: HashMap lookup, returns error if key not found.
    fn read(&self, normalized: &str) -> Result<String, MdsError>;

    /// Determine if a normalized key represents a .md file (affects frontmatter handling).
    fn is_markdown(&self, normalized: &str) -> bool;
}
```

### VirtualFs Path Resolution Rules

- Keys use `/` as separator (regardless of host OS)
- `normalize("components/header.mds", "./footer.mds")` -> `"components/footer.mds"`
- `normalize("components/header.mds", "../shared.mds")` -> `"shared.mds"`
- `normalize("a/b/c.mds", "../../d.mds")` -> `"d.mds"`
- Traversal above root produces error (e.g., `normalize("a.mds", "../../x.mds")` -> Error)
- No duplicate slashes, no trailing slashes, `.` segments removed

### VirtualFs "Module Not Found" Error

When `read()` is called with a key not in the HashMap:
- Error code: `"mds::module_not_found"`
- Message: `"module '{key}' not found in provided module map"`
- Span: from the @import directive in the importing file

## 5. Error Serialization

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct SerializedError {
    pub code: String,          // "mds::syntax", "mds::undefined_var", "mds::module_not_found"
    pub message: String,       // User-facing message
    pub help: Option<String>,  // Diagnostic help text
    pub span: Option<SerializedSpan>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SerializedSpan {
    pub offset: usize,         // Byte offset into source
    pub length: usize,         // Span length in bytes
    pub line: Option<usize>,   // 1-indexed line number
    pub column: Option<usize>, // 1-indexed column number
}

impl MdsError {
    pub fn serialize(&self) -> SerializedError { ... }
}
```

## 6. CompileOutput (Dependency Graph)

```rust
pub struct CompileOutput {
    pub output: String,
    pub warnings: Vec<String>,
    pub dependencies: Vec<String>,  // Normalized keys of all resolved modules
}

pub fn compile_with_deps(
    path: impl AsRef<Path>,
    runtime_vars: Option<HashMap<String, Value>>,
) -> Result<CompileOutput, MdsError>;

pub fn compile_str_with_deps(
    source: &str,
    base_dir: Option<&Path>,
    runtime_vars: Option<HashMap<String, Value>>,
    fs: Option<Box<dyn FileSystem>>,
) -> Result<CompileOutput, MdsError>;
```

## 7. TypeScript Types (@mds/mds)

```typescript
export interface CompileOptions {
    modules?: Record<string, string>;
    vars?: Record<string, unknown>;
}

export interface CompileResult {
    output: string;
    warnings: string[];
    dependencies: string[];
}

export interface MdsError {
    code: string;
    message: string;
    help?: string;
    span?: {
        offset: number;
        length: number;
        line?: number;
        column?: number;
    };
}

export function compile(source: string, options?: CompileOptions): CompileResult;
export function compileFile(path: string, options?: Omit<CompileOptions, 'modules'>): CompileResult;
export function check(source: string, options?: CompileOptions): void;
export function checkFile(path: string, options?: Omit<CompileOptions, 'modules'>): void;
```

## 8. Implementation Phases

### Phase 1: Cargo Workspace Split

**Goal:** Split single crate into workspace with mds-core (library) + mds-cli (binary). Zero behavioral changes.

**Create:**
- `crates/mds-core/Cargo.toml` — library manifest
- `crates/mds-core/src/*` — all current src/ files except main.rs
- `crates/mds-cli/Cargo.toml` — binary manifest, depends on mds-core
- `crates/mds-cli/src/main.rs` — current main.rs

**Modify:**
- `Cargo.toml` — convert to workspace root

**Delete:**
- `src/` — all files moved to crates/

**Visibility changes needed:**
- `MAX_TRAVERSAL_DEPTH` — pub(crate) -> pub
- `MAX_FILE_SIZE` — already pub via re-export
- `ModuleCache` — already pub

**Test:** `cargo test --workspace` passes all 286 tests. Binary produces identical output.

---

### Phase 2: FileSystem Trait + ModuleCache Refactor

**Goal:** Introduce trait, convert resolver from PathBuf to String keys, make filesystem-agnostic.

**Create:**
- `crates/mds-core/src/fs.rs` — FileSystem trait, NativeFs, VirtualFs

**Modify:**
- `crates/mds-core/src/resolver.rs` — major refactor:
  - `ModuleCache { fs: Box<dyn FileSystem>, modules: HashMap<String, ...>, resolving: IndexSet<String> }`
  - Remove check_symlink, check_path_traversal, canonicalize_and_check, read_validated_file (move to NativeFs)
  - `resolve()` uses `self.fs.normalize()` and `self.fs.read()`
  - `find_project_root` moves into NativeFs
- `crates/mds-core/src/lib.rs` — add `pub mod fs;`, new constructors:
  - `ModuleCache::native()` — uses NativeFs (backward compat)
  - `ModuleCache::virtual_fs(modules: HashMap<String, String>)` — uses VirtualFs

**Test (TDD):**
1. VirtualFs::normalize() — relative paths, ../ handling, boundary errors
2. VirtualFs::read() — found/not-found, ModuleNotFound error code
3. NativeFs — existing security behavior preserved (symlink, traversal)
4. All 286 integration tests pass (using NativeFs)
5. New: compile with VirtualFs + multi-file module map

---

### Phase 3: Error Serialization + Dependency Graph

**Goal:** SerializedError for JS, CompileOutput with dependencies.

**Modify:**
- `crates/mds-core/src/error.rs` — add SerializedError, SerializedSpan, MdsError::serialize()
- `crates/mds-core/src/resolver.rs` — track resolved keys during resolution
- `crates/mds-core/src/lib.rs` — add CompileOutput struct, compile_with_deps(), compile_str_with_deps()

**Test (TDD):**
1. serialize() for each MdsError variant — correct code, message, span
2. Line/column computation from byte offset
3. Dependencies for multi-file compile — correct keys, topological order
4. Dependencies for single-file compile — empty vec

---

### Phase 4: WASM Bindings (parallel with Phase 5)

**Goal:** wasm-bindgen crate, <500KB uncompressed.

**Create:**
- `bindings/wasm/Cargo.toml` — crate-type cdylib, wasm-bindgen, serde-wasm-bindgen
- `bindings/wasm/src/lib.rs` — #[wasm_bindgen] exports: compile(), check()
- `bindings/wasm/tests/web.rs` — wasm-bindgen-test

**Modify:**
- `Cargo.toml` — add "bindings/wasm" to workspace members
- `crates/mds-core/Cargo.toml` — feature-gate miette display behind `fancy-errors` (default on for CLI, off for WASM)

**WASM API:**
```rust
#[wasm_bindgen]
pub fn compile(source: &str, options: JsValue) -> Result<JsValue, JsValue>;
// options: { modules?: Record<string, string>, vars?: Record<string, any> }
// returns: { output: string, warnings: string[], dependencies: string[] }
// error:   { code: string, message: string, help?: string, span?: {...} }
```

**Test:** wasm-pack test --node, size budget check, parity with native output.

---

### Phase 5: napi-rs Bindings (parallel with Phase 4)

**Goal:** Native Node.js addon for high-performance compilation.

**Create:**
- `bindings/napi/Cargo.toml` — napi-rs deps
- `bindings/napi/src/lib.rs` — #[napi] exports
- `bindings/napi/package.json` — napi platform config
- `bindings/napi/__test__/index.spec.ts`

**Modify:**
- `Cargo.toml` — add "bindings/napi" to workspace members

**Platforms (initial):**
- `darwin-arm64`
- `darwin-x64`
- `linux-x64-gnu`

**Test:** Native test on dev machine, parity with CLI output.

---

### Phase 6: @mds/mds Universal JS Package

**Goal:** Single TypeScript API, auto-selects WASM or native backend.

**Create:**
- `packages/mds/package.json` — @mds/mds, ESM, conditional exports
- `packages/mds/src/index.ts` — entry point with backend detection
- `packages/mds/src/types.ts` — TypeScript interfaces
- `packages/mds/src/wasm.ts` — WASM backend (singleton module instance)
- `packages/mds/src/native.ts` — napi backend
- `packages/mds/tests/compile.test.ts`
- `package.json` — npm workspace root
- `tsconfig.base.json`

**Critical:** WASM module instantiation must be cached (singleton pattern). First call to compile() instantiates; subsequent calls reuse.

**Test:** Compile via JS API, assert output matches CLI, test error objects.

---

### Phase 7: Bundler Plugins

**Goal:** Vite, Webpack, Rollup plugins + shared utils.

**Create:**
- `packages/bundler-utils/` — @mds/bundler-utils (shared transform/resolve logic)
- `packages/vite-plugin/` — @mds/vite-plugin (transform hook + HMR via addWatchFile)
- `packages/webpack-loader/` — @mds/webpack-loader (loader + this.addDependency)
- `packages/rollup-plugin/` — @mds/rollup-plugin (transform hook)

**Shared logic (bundler-utils):**
```typescript
export function createMdsTransformer(mds: MdsApi): {
    transform(id: string, source: string): CompileResult & { modules: Record<string, string> };
    shouldTransform(id: string): boolean;
    collectDependencies(id: string): string[];
};
```

**HMR strategy:** All plugins use CompileResult.dependencies to register file watchers. On change, invalidate the importing module. HMR type: full reload (appropriate for template content).

**Test:** Integration tests with actual bundler builds on fixture projects.

## 9. Dependency Graph

```
Phase 1 (workspace split)
    |
    v
Phase 2 (FileSystem trait)
    |
    v
Phase 3 (error serialization + deps)
   / \
  v   v
Phase 4    Phase 5
(WASM)     (napi-rs)
  \         /
   v       v
  Phase 6 (@mds/mds)
       |
       v
  Phase 7 (bundler plugins)
```

Phases 4+5 parallelizable. Phase 7 parallelizable internally (3 plugins independent).

## 10. Test Strategy

### Parity Corpus

Shared fixtures ensuring native/WASM/napi produce identical output:
- Move `tests/fixtures/` to `tests/corpus/`
- Each fixture: `input.mds` + `expected.md` (+ optional `vars.json`, `modules/`)
- CI runs corpus through all backends, diffs output

### TDD Per Phase

Each phase writes failing tests first:
- Phase 2: VirtualFs tests before implementing normalize/read
- Phase 3: serialize() tests before implementing
- Phase 4: wasm-bindgen-test before WASM glue
- Phase 6: TypeScript tests before runtime detection

## 11. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| WASM size > 500KB | Slow load | Feature-gate serde_yml, wasm-opt -Oz, strip debug |
| serde_yml in WASM | Size bloat | Accept YAML in source only (unavoidable), vars as JSON |
| VirtualFs edge cases | Wrong resolution | Extensive path normalization test matrix |
| napi CI complexity | Missing binaries | Start darwin+linux, add platforms incrementally |
| Phase 2 PR size | Hard to review | Atomic but reviewable in sections; tight coupling justifies |
| Breaking API change | CLI breaks | Phase 1 is structural only; Phase 2 keeps backward-compat wrappers |

## 12. PR Description Guidance

**Problem Being Solved:** MDS is a Rust-only CLI with no JavaScript integration. Browser-based tools, Node.js build systems, and the DevFlow TypeScript project cannot use MDS without shelling out to a binary.

**Key Changes to Highlight:**
- FileSystem trait decouples compiler from OS (Phase 2) — the keystone enabling all bindings
- String-based module keys replace PathBuf (Phase 2) — semantic shift from "files" to "modules"
- SerializedError (Phase 3) — structured errors cross the Rust/JS boundary cleanly
- WASM singleton (Phase 6) — instantiate once, amortize startup across all compilations

**Breaking Changes:** None expected. Existing CLI behavior preserved throughout. Library API gains new functions but does not remove or change existing ones.

**Reviewer Focus Areas:**
- Phase 2 NativeFs: must preserve all existing security guarantees (symlink, traversal)
- Phase 2 VirtualFs normalize: ../ resolution correctness and boundary enforcement
- Phase 4 WASM: size budget, error propagation across boundary
- Phase 6 runtime detection: must work in Node.js, Deno, browser, edge workers

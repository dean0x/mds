# Documentation Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing package README for public npm package** - `packages/mds/`
**Confidence**: 95%
- Problem: The `@mds/mds` package is a new public-facing npm package with conditional exports (Node.js native/WASM, browser WASM), an `init()` lifecycle requirement for browsers, and environment-specific behaviors. There is no `README.md` in the package directory. Consumers discovering this package via npm or the monorepo will have no guidance on installation, usage, or the backend selection strategy.
- Fix: Add `packages/mds/README.md` covering:
  - Quick start (Node.js auto-selects native addon with WASM fallback)
  - Browser usage (must call `init()` before `compile`/`check`)
  - API reference (compile, check, compileFile, checkFile, getBackend, init, isMdsError)
  - Environment variable `MDS_BACKEND` for forcing backend selection
  - Error handling with `isMdsError`

**Missing JSDoc on public API functions in `types.ts`** - `packages/mds/src/types.ts:1-48`
**Confidence**: 90%
- Problem: `types.ts` defines the entire public API surface (CompileResult, CheckResult, CompileOptions, FileOptions, MdsError, MdsErrorSpan, InitOptions, MdsBackend) with no JSDoc on any interface or its members. These types are re-exported from `index.ts` and are what consumers will encounter in their IDE tooltips.
- Fix: Add JSDoc to each interface and its non-obvious members. For example:
  ```typescript
  /** Result of compiling an MDS template to Markdown output. */
  export interface CompileResult {
    /** The rendered Markdown string. */
    output: string;
    /** Non-fatal warnings emitted during compilation. */
    warnings: string[];
    /** File paths of resolved imports (absolute paths when using compileFile). */
    dependencies: string[];
  }

  /**
   * Options for string-based compile/check operations.
   */
  export interface CompileOptions {
    /** Runtime variable overrides merged with frontmatter variables. */
    vars?: Record<string, unknown>;
  }
  ```

**Missing JSDoc on exported functions in `node.ts`** - `packages/mds/src/node.ts:42-60`
**Confidence**: 88%
- Problem: The `compile`, `check`, `compileFile`, `checkFile`, and `getBackend` functions exported from the Node.js entry point have no JSDoc. These are the primary API consumers will call. The `init` re-export has good documentation (lines 62-73), showing the standard is understood but not applied consistently.
- Fix: Add JSDoc with brief description, `@param`, and `@returns` for each:
  ```typescript
  /**
   * Compile an MDS template string to Markdown.
   * @param source - MDS template source text
   * @param options - Optional compilation settings (vars)
   * @returns Compiled output with warnings and dependency list
   */
  export function compile(source: string, options?: CompileOptions): CompileResult {
    return backend.compile(source, options);
  }
  ```

### MEDIUM

**No CHANGELOG entry for new package** - `CHANGELOG.md`
**Confidence**: 85%
- Problem: The CHANGELOG only documents the initial 0.1.0 release. This PR introduces a significant new package (`@mds/mds` universal JS bindings) that should be documented under a new version entry or an "Unreleased" section.
- Fix: Add an `## [Unreleased]` section to `CHANGELOG.md`:
  ```markdown
  ## [Unreleased]

  ### Added

  **JavaScript package** (`@mds/mds`)
  - Universal package with native (napi) and WASM backend adapters
  - Node.js entry: auto-selects native addon, falls back to WASM
  - Browser entry: WASM-only with explicit init() lifecycle
  - `compile()`, `check()`, `compileFile()`, `checkFile()` API
  - `scanImports()` for static import resolution
  - Recursive module scanner with security guards (symlink rejection, path traversal prevention)
  ```

**Missing JSDoc on `MdsBackend` interface methods** - `packages/mds/src/types.ts:38-44`
**Confidence**: 82%
- Problem: The `MdsBackend` interface defines the contract for all backend implementations but has no doc comments on its methods. Implementors or consumers extending this interface get no IDE guidance.
- Fix: Add brief JSDoc to each method:
  ```typescript
  export interface MdsBackend {
    /** Compile an MDS source string to Markdown. */
    compile(source: string, options?: CompileOptions): CompileResult;
    /** Validate an MDS source string without rendering. */
    check(source: string, options?: CompileOptions): CheckResult;
    /** Compile an MDS file, resolving imports from disk. */
    compileFile(path: string, options?: FileOptions): Promise<CompileResult>;
    /** Validate an MDS file, resolving imports from disk. */
    checkFile(path: string, options?: FileOptions): Promise<CheckResult>;
    /** Returns which backend is active ('native' or 'wasm'). */
    getBackend(): BackendType;
  }
  ```

**Top-level README does not mention the JavaScript package** - `README.md:70-85`
**Confidence**: 83%
- Problem: The README's "Library Usage" section only shows Rust usage. The new `@mds/mds` JavaScript package is a major new consumer surface that is not referenced anywhere in the project's top-level documentation.
- Fix: Add a "JavaScript / TypeScript" subsection under "Library Usage":
  ```markdown
  ### JavaScript / TypeScript

  ```typescript
  // Node.js — auto-selects native addon with WASM fallback
  import { compile, compileFile } from '@mds/mds';

  const result = compile('Hello {name}!', { vars: { name: 'World' } });
  console.log(result.output); // "Hello World!\n"

  // Browser — requires explicit initialization
  import { init, compile } from '@mds/mds';
  await init();
  const result = compile('Hello {name}!', { vars: { name: 'World' } });
  ```
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`InitOptions.wasmUrl` type union lacks explanation** - `packages/mds/src/types.ts:35`
**Confidence**: 80%
- Problem: `wasmUrl?: string | URL | Response | BufferSource` accepts four different types but provides no guidance on when to use each. A consumer seeing this in their IDE won't know the difference between passing a URL string vs a Response vs a BufferSource.
- Fix: Add JSDoc explaining each variant:
  ```typescript
  export interface InitOptions {
    /**
     * Custom WASM module source. Accepts:
     * - `string` or `URL`: Fetched via the network
     * - `Response`: A pre-fetched Response (e.g., from a custom CDN)
     * - `BufferSource`: Pre-loaded WASM bytes (ArrayBuffer or TypedArray)
     */
    wasmUrl?: string | URL | Response | BufferSource;
  }
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing documentation issues detected in the changed files.

## Suggestions (Lower Confidence)

- **`module-scanner.ts` could document its security model in a file-level JSDoc** - `packages/mds/src/util/module-scanner.ts:1` (Confidence: 70%) — The function-level docs are good but a top-of-file overview explaining the threat model (untrusted import paths, why symlinks are rejected, relationship to Rust's VirtualFs) would help maintainers.
- **`browser.ts` `compileFile`/`checkFile` could document why they reject** - `packages/mds/src/browser.ts:72-88` (Confidence: 65%) — The error messages are clear, but a JSDoc explaining "browser environments lack filesystem access" would appear in IDE tooltips before the user calls and gets an error.
- **`crates/mds-napi/index.js` has no file-level documentation** - `crates/mds-napi/index.js:1` (Confidence: 62%) — The native binding loader uses a platform/arch matrix with musl detection; a brief header comment explaining what this file does and how it relates to the build pipeline would help contributors.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 3 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 4/10
**Recommendation**: CHANGES_REQUESTED

The package introduces a well-structured codebase with good internal comments on complex logic (init race conditions, security checks, WASM fallback strategy). However, as a new public npm package with a non-trivial lifecycle (init requirement, backend selection, environment-specific behavior), the complete absence of a package README and the missing JSDoc on the primary public API types create a significant documentation gap. Consumers cannot discover how to use this package without reading source code.

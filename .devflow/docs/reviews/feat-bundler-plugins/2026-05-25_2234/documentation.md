# Documentation Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T22:34

## Issues in Your Changes (BLOCKING)

### HIGH

**`vars` type documented as `Record<string, string>` but actual type is `Record<string, unknown>` (6 occurrences)** -- Confidence: 95%
- `packages/bundler-utils/README.md:67`, `packages/vite-plugin/README.md:66`, `packages/rollup-plugin/README.md:64`, `packages/webpack-loader/README.md:71`, `CHANGELOG.md:21`, `README.md:85`
- Problem: All four package READMEs, the CHANGELOG, and the root README document the options interface as `vars?: Record<string, string>`, but the actual TypeScript type in `packages/bundler-utils/src/types.ts:45` is `vars?: Record<string, unknown>`. Users following the documented type will create code that is more restrictive than the actual API. More importantly, users who pass non-string values (which the API accepts) will believe they are using it incorrectly.
- Fix: Update all six locations to show `Record<string, unknown>`:
  ```ts
  interface MdsPluginOptions {
    /** Variables available for interpolation in .mds templates. */
    vars?: Record<string, unknown>;
  }
  ```
  And in CHANGELOG/README prose: `{ vars?: Record<string, unknown> }`

### MEDIUM

**JSDoc for `shouldTransform` says "500 bytes" but code uses 512** -- Confidence: 95%
- `packages/bundler-utils/src/frontmatter.ts:27,29`
- Problem: The JSDoc says "Frontmatter detection reads only the first 500 bytes" and "There is a closing `---` before byte 500", but the implementation uses `const PEEK_BYTES = 512` on line 41. This is a code-comment drift issue that could confuse contributors modifying the peek window.
- Fix: Update the JSDoc to say 512:
  ```typescript
  /**
   * ...
   * Frontmatter detection reads only the first 512 bytes and looks for:
   * 1. File starts with `---`
   * 2. There is a closing `---` before byte 512
   * 3. Between the opening and closing `---`, there is a `type: mds` key
   */
  ```

**`createMdsTransformer` missing JSDoc** -- Confidence: 85%
- `packages/bundler-utils/src/transform.ts:48`
- Problem: `createMdsTransformer` is the primary public API of `@mds/bundler-utils` -- it is the factory function that all three plugin packages depend on. Every other exported function in the package (`isMdsExtension`, `cleanId`, `shouldTransform`, `formatMdsError`) has JSDoc, but this one does not. The `@param` and `@returns` documentation is important for consumers writing custom bundler plugins.
- Fix: Add JSDoc above the export:
  ```typescript
  /**
   * Creates a transformer that compiles `.mds` files into JavaScript ES modules.
   * Lazily initializes the MDS compiler on first transform and caches the init
   * promise so subsequent calls skip initialization.
   *
   * @param mds - The MDS compiler API (typically `await import('@mds/mds')`)
   * @param options - Plugin options forwarded to the compiler (e.g. template variables)
   * @returns An object with `shouldTransform(id)` and `transform(id)` methods
   */
  export function createMdsTransformer(mds: MdsApi, options?: MdsPluginOptions): {
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`mdsLoader` default export missing JSDoc** -- Confidence: 82%
- `packages/webpack-loader/src/index.ts:40`
- Problem: The webpack loader's default export function `mdsLoader` has no JSDoc. The two test-only exports (`_resetForTesting`, `_setTransformerForTesting`) both have JSDoc, but the main export that webpack users interact with does not. This is an inconsistency since the equivalent exports in vite-plugin and rollup-plugin (`mdsPlugin`) both have JSDoc.
- Fix:
  ```typescript
  /**
   * Webpack 5 async loader that compiles `.mds` files into JavaScript ES modules.
   * Uses a module-level singleton transformer to avoid re-initializing the MDS
   * compiler for each file in a build.
   */
  export default async function mdsLoader(this: LoaderContext): Promise<void> {
  ```

## Pre-existing Issues (Not Blocking)

No pre-existing documentation issues found.

## Suggestions (Lower Confidence)

- **Unused import in bundler-utils README example** - `packages/bundler-utils/README.md:26` (Confidence: 70%) -- The code example imports `shouldTransform` as a standalone function but never uses it; the example only uses `transformer.shouldTransform()`. Consider removing it from the import or adding a standalone usage example.

- **Vite README headline says "with HMR support" but behavior is full-reload** - `packages/vite-plugin/README.md:3` (Confidence: 65%) -- The heading "Vite plugin for importing `.mds` templates as ES modules with HMR support" may set expectations for component-level hot replacement. The actual behavior is a full-page reload (documented correctly in the source code). Consider "with dev-server reload support" or simply dropping "with HMR support" to avoid ambiguity.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The documentation is comprehensive overall -- READMEs cover installation, configuration, usage, TypeScript setup, and options for all four packages. JSDoc is present on most exports and the types file is thoroughly documented. The CHANGELOG and root README are both updated. The main blocking issue is the `Record<string, string>` vs `Record<string, unknown>` type mismatch across six locations, which would actively mislead users about the accepted value types.

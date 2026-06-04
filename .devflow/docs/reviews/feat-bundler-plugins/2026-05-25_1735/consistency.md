# Consistency Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25T17:35

## Issues in Your Changes (BLOCKING)

### HIGH

**Build artifacts (dist/) committed to version control -- deviates from existing convention** - `packages/bundler-utils/dist/`, `packages/rollup-plugin/dist/`, `packages/vite-plugin/dist/`, `packages/webpack-loader/dist/`
**Confidence**: 95%
- Problem: The existing `@mds/mds` package has `packages/mds/dist/` listed in the root `.gitignore` and does not track build artifacts. All 4 new packages commit their entire `dist/` directories (32 files total: `.js`, `.d.ts`, `.js.map`, `.d.ts.map`). This contradicts the established convention of treating `dist/` as a build output.
- Fix: Add gitignore entries for the new packages. Either add per-package lines to the root `.gitignore`:
  ```
  packages/bundler-utils/dist/
  packages/rollup-plugin/dist/
  packages/vite-plugin/dist/
  packages/webpack-loader/dist/
  ```
  Or create a shared pattern like `packages/*/dist/` (which would also cover `packages/mds/dist/`, replacing the existing specific entry). Then remove the tracked dist files with `git rm -r --cached packages/bundler-utils/dist/ packages/rollup-plugin/dist/ packages/vite-plugin/dist/ packages/webpack-loader/dist/`.

**Import ordering inconsistency: webpack-loader puts value import before type import** - `packages/webpack-loader/src/index.ts:1-2`
**Confidence**: 85%
- Problem: The rollup-plugin and vite-plugin both place `import type` before value imports from the same module. The webpack-loader reverses this order:
  ```typescript
  // webpack-loader (inconsistent)
  import { createMdsTransformer, formatMdsError } from '@mds/bundler-utils';
  import type { MdsPluginOptions } from '@mds/bundler-utils';

  // rollup-plugin and vite-plugin (consistent)
  import type { MdsPluginOptions } from '@mds/bundler-utils';
  import { createMdsTransformer, formatMdsError, cleanId } from '@mds/bundler-utils';
  ```
  The bundler-utils package itself also follows type-first ordering (e.g., `transform.ts:1-2`).
- Fix: In `packages/webpack-loader/src/index.ts`, swap lines 1 and 2:
  ```typescript
  import type { MdsPluginOptions } from '@mds/bundler-utils';
  import { createMdsTransformer, formatMdsError } from '@mds/bundler-utils';
  ```

### MEDIUM

**package.json JSON formatting diverges from reference package** - `packages/bundler-utils/package.json`, `packages/rollup-plugin/package.json`, `packages/vite-plugin/package.json`, `packages/webpack-loader/package.json`
**Confidence**: 82%
- Problem: The reference `@mds/mds` package.json uses expanded multi-line formatting for all objects and arrays:
  ```json
  "engines": {
    "node": ">=22.0.0"
  },
  "files": [
    "dist/"
  ],
  ```
  All 4 new packages use compact inline formatting:
  ```json
  "engines": { "node": ">=22.0.0" },
  "files": ["dist/"],
  ```
  The new packages are internally consistent with each other, but diverge from the established format. This affects readability and diff noise when fields are added.
- Fix: Reformat to match the expanded multi-line style used in `packages/mds/package.json`, or accept the compact style as a deliberate convention shift for simpler packages. If accepting, this should be a conscious decision since it creates two formatting styles across the monorepo.

**JSDoc comments missing on all interface members in bundler-utils types** - `packages/bundler-utils/src/types.ts:1-29`
**Confidence**: 80%
- Problem: The reference `@mds/mds` package (`packages/mds/src/types.ts`) documents every interface and every member with JSDoc comments (e.g., `/** Rendered Markdown output. */` on `CompileResult.output`). The new `bundler-utils/src/types.ts` defines 5 interfaces with zero JSDoc comments on any field. This is the public API surface for all downstream plugin packages.
- Fix: Add JSDoc comments to at least the exported interfaces and their members, matching the style in `packages/mds/src/types.ts`. For example:
  ```typescript
  /** Interface for interacting with the @mds/mds compiler. */
  export interface MdsApi {
    /** Compile an MDS file at the given path. */
    compileFile(path: string, options?: { vars?: Record<string, unknown> }): Promise<CompileResult>;
    /** Initialize the compiler backend. Must be called before compile operations. */
    init(): Promise<void>;
    /** Type guard for MDS compiler errors. */
    isMdsError(err: unknown): boolean;
  }
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Test name convention divergence** - `packages/bundler-utils/__test__/*.spec.mjs`, `packages/rollup-plugin/__test__/plugin.spec.mjs`, `packages/vite-plugin/__test__/plugin.spec.mjs`, `packages/webpack-loader/__test__/loader.spec.mjs` (Confidence: 65%) -- The existing @mds/mds tests use ID-prefixed names (e.g., `U-C1: compile plain text`) while the new tests use plain descriptive names. This is defensible for utility packages but creates two naming conventions in the monorepo.

- **MdsErrorLike type partially duplicates MdsError/MdsErrorSpan** - `packages/bundler-utils/src/errors.ts:3-8` (Confidence: 70%) -- The `MdsErrorLike` interface and `isMdsErrorLike` type guard duplicate the `MdsError`/`isMdsError` pattern from `@mds/mds/src/types.ts` with a reduced `span` type (missing `offset`/`length`). This is likely intentional to avoid a hard dependency on internal types, but could diverge over time.

- **PluginContext interface duplicated across rollup-plugin and vite-plugin** - `packages/rollup-plugin/src/index.ts:4-8`, `packages/vite-plugin/src/index.ts:3-7` (Confidence: 60%) -- Both plugins define local `PluginContext`/`PluginTransformContext` interfaces rather than sharing them through bundler-utils. The interfaces differ appropriately (Rollup has `error()`, Vite does not), so this may be intentional, but a shared base type in bundler-utils could reduce duplication.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 2 | 2 | - |
| Should Fix | - | - | - | - |
| Pre-existing | - | - | - | - |

**Consistency Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The 4 new packages are internally very consistent with each other -- identical tsconfig.json files, matching package.json structure, parallel test organization, and uniform use of `@mds/bundler-utils` as the shared utility layer. However, two issues require attention before merge:

1. **dist/ committed to git** (HIGH) -- 32 build artifacts tracked in version control, breaking the established convention where `packages/mds/dist/` is gitignored. This is the most impactful finding.
2. **Import ordering flip** (HIGH) -- webpack-loader reverses the type-first import convention used by the other 3 packages.

The medium-severity formatting and documentation items are worth addressing to maintain long-term monorepo consistency but are not blockers.

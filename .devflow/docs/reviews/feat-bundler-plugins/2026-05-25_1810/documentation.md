# Documentation Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25T18:10

## Issues in Your Changes (BLOCKING)

### HIGH

**Four new npm packages have no README files** - `packages/bundler-utils/`, `packages/vite-plugin/`, `packages/rollup-plugin/`, `packages/webpack-loader/`
**Confidence**: 95%
- Problem: All four new packages (@mds/bundler-utils, @mds/vite-plugin, @mds/rollup-plugin, @mds/webpack-loader) ship without README.md files. These are user-facing npm packages with `"files"` entries in package.json, meaning consumers who install them will have no usage documentation. The existing `@mds/mds` package has a README, making this an inconsistency as well. Bundler plugins are integration-heavy and users need to know: (a) what the plugin does, (b) how to configure it in their bundler, (c) what peer dependencies are required, and (d) how `.mds` and `.md` (with `type: mds` frontmatter) files are handled.
- Fix: Add a README.md to each package with at minimum: a one-line description, install command, configuration example for the target bundler, peer dependency requirements, and a note about `*.mds` module declaration (`@mds/bundler-utils/mds`). For example, for vite-plugin:
```markdown
# @mds/vite-plugin

Vite plugin for MDS — import `.mds` templates as ES modules.

## Install

npm install @mds/vite-plugin @mds/mds

## Usage

```ts
// vite.config.ts
import mds from '@mds/vite-plugin';
export default { plugins: [mds()] };
```

## TypeScript

Add to your `tsconfig.json` compilerOptions.types or use a triple-slash reference:
/// <reference types="@mds/bundler-utils/mds" />

## Options

- `vars` — Runtime variables for template interpolation
```

### MEDIUM

**All four package.json files missing `description` field** - `packages/bundler-utils/package.json`, `packages/vite-plugin/package.json`, `packages/rollup-plugin/package.json`, `packages/webpack-loader/package.json`
**Confidence**: 92%
- Problem: The `description` field is absent from all four package.json files. This field is displayed on npm search results, npm package pages, and by tools like `npm ls`. Without it, users searching for MDS bundler integrations will see empty descriptions.
- Fix: Add a `description` field to each package.json:
```json
// bundler-utils
"description": "Shared utilities for MDS bundler plugins (Vite, Rollup, Webpack)"

// vite-plugin
"description": "Vite plugin for importing MDS templates as ES modules"

// rollup-plugin
"description": "Rollup plugin for importing MDS templates as ES modules"

// webpack-loader
"description": "Webpack loader for importing MDS templates as ES modules"
```

**CHANGELOG not updated for four new packages** - `CHANGELOG.md:8`
**Confidence**: 90%
- Problem: The top-level `CHANGELOG.md` has an `[Unreleased]` section that documents changes to `@mds/mds` but does not mention the four new bundler integration packages being added in this PR. This is a significant feature addition (the PR title says "Adds bundler integration") that should be documented for consumers tracking project changes.
- Fix: Add entries under `[Unreleased] > Added`:
```markdown
### Added

- **Bundler integration packages** — import `.mds` templates natively in bundler projects
  - `@mds/bundler-utils` — shared transformer, frontmatter detection, and error formatting
  - `@mds/vite-plugin` — Vite plugin with HMR full-reload support
  - `@mds/rollup-plugin` — Rollup plugin with watch file tracking
  - `@mds/webpack-loader` — Webpack loader with dependency tracking
  - TypeScript module declarations for `*.mds` imports (`@mds/bundler-utils/mds`)
```

**Top-level README does not mention bundler/JS usage** - `README.md`
**Confidence**: 85%
- Problem: The top-level README covers only the CLI and Rust library usage. This PR introduces JavaScript/TypeScript bundler integration, which is a primary use case for the `@mds/mds` npm package and the new bundler plugins. Users arriving at the repository have no way to discover that bundler integration exists. The README's "Library Usage" section shows only Rust examples.
- Fix: Add a "Bundler Integration" or "JavaScript Usage" section to the top-level README after "Library Usage":
```markdown
## Bundler Integration

Import `.mds` templates directly in your JavaScript/TypeScript project:

```ts
import greeting from './hello.mds';
console.log(greeting); // rendered Markdown string
```

| Bundler  | Package              |
|----------|----------------------|
| Vite     | `@mds/vite-plugin`   |
| Rollup   | `@mds/rollup-plugin` |
| Webpack  | `@mds/webpack-loader` |

See each package's README for setup instructions.
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Exported functions in frontmatter.ts missing JSDoc (3 occurrences)** - `packages/bundler-utils/src/frontmatter.ts:3`, `packages/bundler-utils/src/frontmatter.ts:7`, `packages/bundler-utils/src/frontmatter.ts:10`
**Confidence**: 85%
- Problem: `isMdsExtension()` (line 3) and `cleanId()` (line 7) are public API exports (re-exported from index.ts and used by all three plugin packages) but have no JSDoc. `isMdsErrorLike()` in errors.ts (line 10) is internal but `formatMdsError()` (line 16) is also exported without JSDoc. Meanwhile `shouldTransform()` (line 30) already has excellent JSDoc. The types.ts interfaces all received JSDoc in this PR (Cycle 1 resolution), creating a documentation inconsistency within the same package.
- Fix: Add JSDoc to the exported utility functions:
```typescript
/** Returns true if the file path ends with `.mds`. */
export function isMdsExtension(id: string): boolean {

/**
 * Strips query parameters (`?...`) and fragment identifiers (`#...`) from a module id.
 * Bundlers like Vite append query strings (e.g., `?t=123`) for cache busting.
 */
export function cleanId(id: string): string {

/**
 * Formats an MDS compiler error into the shape expected by bundler error reporting.
 * Extracts line/column from MDS error spans and appends help text when available.
 */
export function formatMdsError(err: unknown, id: string): FormattedError {
```

**Plugin factory functions missing JSDoc (3 occurrences)** - `packages/vite-plugin/src/index.ts:24`, `packages/rollup-plugin/src/index.ts:20`, `packages/webpack-loader/src/index.ts:40`
**Confidence**: 82%
- Problem: The default-exported factory functions in all three plugin packages have no JSDoc documentation. These are the primary public API entry points that users interact with directly. The Vite and Rollup plugins export `mdsPlugin()` and the webpack loader exports `mdsLoader()` — all without documenting their purpose, parameters, or behavior.
- Fix: Add JSDoc to each factory function:
```typescript
// vite-plugin/src/index.ts
/**
 * Creates a Vite plugin that compiles `.mds` and `.md` (type: mds) files
 * into ES modules with a default string export and metadata.
 * Triggers full-reload HMR when `.mds` files change.
 */
export default function mdsPlugin(options?: MdsPluginOptions): VitePlugin {

// rollup-plugin/src/index.ts
/**
 * Creates a Rollup plugin that compiles `.mds` and `.md` (type: mds) files
 * into ES modules. Registers dependencies as watch files for rebuild.
 */
export default function mdsPlugin(options?: MdsPluginOptions): RollupPlugin {

// webpack-loader/src/index.ts
/**
 * Webpack loader that compiles `.mds` files into ES modules.
 * Uses a module-level singleton transformer; options are captured on first invocation.
 */
export default async function mdsLoader(this: LoaderContext): Promise<void> {
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`mds.d.ts` type declaration lacks JSDoc for the `metadata` export** - `packages/bundler-utils/mds.d.ts:4`
**Confidence**: 80%
- Problem: The ambient module declaration for `*.mds` files exposes `metadata` with type `{ warnings: string[]; dependencies: string[] }` but no JSDoc explaining what it contains or when a user might use it. This is the TypeScript contract users see in their IDE.
- Fix:
```typescript
declare module '*.mds' {
  /** The rendered Markdown output of the compiled MDS template. */
  const content: string;
  export default content;
  /** Compiler metadata: non-fatal warnings and transitive file dependencies. */
  export const metadata: { warnings: string[]; dependencies: string[] };
}
```

## Suggestions (Lower Confidence)

- **`keywords` missing in package.json files** - all four packages (Confidence: 70%) -- Adding npm keywords like `"mds"`, `"markdown"`, `"template"`, `"vite-plugin"` / `"rollup-plugin"` / `"webpack-loader"` would improve discoverability on npm search.

- **`repository` field missing in package.json files** - all four packages (Confidence: 65%) -- npm packages typically include a `repository` field pointing to the monorepo with a `directory` subpath, helping users navigate from npm to source code.

- **`ensureTransformer` in webpack-loader lacks JSDoc** - `packages/webpack-loader/src/index.ts:15` (Confidence: 62%) -- This module-internal function has a detailed inline comment about options capture but no JSDoc summarizing its singleton-with-retry behavior.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 1 | 3 | - |
| Should Fix | - | - | 2 | - |
| Pre-existing | - | - | 1 | - |

**Documentation Score**: 4/10
**Recommendation**: CHANGES_REQUESTED

The types.ts interfaces are well-documented after Cycle 1 fixes, and inline code comments (escapeForJs rationale, poisoned-promise reset, webpack singleton options capture, Vite error overlay pattern) are excellent. However, four brand-new npm packages shipping without READMEs, without package descriptions, without CHANGELOG entries, and without top-level README mention represents a significant documentation gap for a user-facing feature addition. The code-level JSDoc is partially there (types.ts, shouldTransform) but missing on several other public API exports.

# TypeScript Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25T17:35

## Issues in Your Changes (BLOCKING)

### HIGH

**Failed init promise permanently poisons subsequent calls (2 occurrences)** -- Confidence: 92%
- `packages/bundler-utils/src/transform.ts:31-38`, `packages/webpack-loader/src/index.ts:15-27`
- Problem: Both `ensureInit()` in `transform.ts` and `ensureTransformer()` in `webpack-loader/src/index.ts` cache the init promise but never reset it on rejection. If `mds.init()` (or `import('@mds/mds')`) fails transiently (e.g., WASM download timeout), `initPromise` remains a rejected promise forever. Every subsequent call returns the same rejected promise with no recovery path. The upstream `@mds/mds` `init()` correctly resets `initPromise = null` on catch (node.ts:183-186), but these consumers do not follow the same pattern.
- Fix: Reset `initPromise` on failure so the next call can retry:
  ```typescript
  // transform.ts ensureInit
  async function ensureInit(): Promise<void> {
    if (initialized) return;
    if (initPromise === null) {
      initPromise = mds.init().then(() => {
        initialized = true;
      }).catch((err: unknown) => {
        initPromise = null;
        throw err;
      });
    }
    return initPromise;
  }
  ```
  Apply the same pattern to `ensureTransformer` in `webpack-loader/src/index.ts`.

### MEDIUM

**`MdsApi.isMdsError` is required by the interface but never consumed** -- Confidence: 85%
- `packages/bundler-utils/src/types.ts:4`
- Problem: The `MdsApi` interface declares `isMdsError(err: unknown): boolean` as a required member. However, `formatMdsError` in `errors.ts` defines its own local `isMdsErrorLike` type guard instead of using `mds.isMdsError()`. This means:
  1. Every consumer of `MdsApi` must provide `isMdsError`, but the bundler-utils code never calls it
  2. The local `isMdsErrorLike` duplicates the same `instanceof Error && code.startsWith('mds::')` check but with a weaker type (`MdsErrorLike` vs the upstream `MdsError` which includes `offset`/`length` in `span`)
- Fix: Either remove `isMdsError` from the `MdsApi` interface (since it is unused), or refactor `formatMdsError` to accept the `MdsApi` and delegate to `mds.isMdsError()` for the discriminant check. Removing it is simpler and more honest about what the API actually requires:
  ```typescript
  export interface MdsApi {
    compileFile(path: string, options?: { vars?: Record<string, unknown> }): Promise<CompileResult>;
    init(): Promise<void>;
  }
  ```

**Committed `dist/` build artifacts for 4 new packages** -- Confidence: 90%
- `packages/bundler-utils/dist/`, `packages/rollup-plugin/dist/`, `packages/vite-plugin/dist/`, `packages/webpack-loader/dist/`
- Problem: All 4 new packages have their `dist/` directories checked into version control. The root `.gitignore` only ignores `packages/mds/dist/`. Committing build output leads to merge conflicts, stale artifacts, and inflated diffs. Each package already has a `"build": "tsc -p tsconfig.json"` script that produces these files.
- Fix: Add the new dist directories to `.gitignore`:
  ```
  packages/bundler-utils/dist/
  packages/rollup-plugin/dist/
  packages/vite-plugin/dist/
  packages/webpack-loader/dist/
  ```
  Then remove them from tracking: `git rm -r --cached packages/bundler-utils/dist packages/rollup-plugin/dist packages/vite-plugin/dist packages/webpack-loader/dist`

**`escapeForJs` uses non-idiomatic `switch(true)` pattern** -- Confidence: 80%
- `packages/bundler-utils/src/transform.ts:11-19`
- Problem: The `switch(true)` with boolean `case` expressions is a non-standard pattern that defeats exhaustiveness checking and is harder for TypeScript tooling to reason about. Each case compares different things (char equality vs charCode equality) making the mixed comparison non-obvious. This is functionally correct but violates TypeScript idiom expectations.
- Fix: Refactor to a straightforward if/else chain or a character lookup map:
  ```typescript
  function escapeForJs(str: string): string {
    let result = '';
    for (let i = 0; i < str.length; i++) {
      const ch = str[i]!;
      const code = ch.charCodeAt(0);
      if (ch === '\\') { result += '\\\\'; }
      else if (ch === '"') { result += '\\"'; }
      else if (ch === '\n') { result += '\\n'; }
      else if (ch === '\r') { result += '\\r'; }
      else if (code === 0x2028) { result += '\\u2028'; }
      else if (code === 0x2029) { result += '\\u2029'; }
      else { result += ch; }
    }
    return result;
  }
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Inline plugin interfaces duplicated across 3 packages** -- Confidence: 82%
- `packages/rollup-plugin/src/index.ts:4-18`, `packages/vite-plugin/src/index.ts:4-22`, `packages/webpack-loader/src/index.ts:4-10`
- Problem: Each plugin package defines its own inline interface for the bundler's plugin/loader context (`PluginContext`, `PluginTransformContext`, `LoaderContext`). While this is an intentional choice to avoid bundler type dependencies (mentioned in PR description), the Rollup and Vite interfaces overlap significantly (`warn`, `addWatchFile`). If these shared members ever drift, they could silently diverge. Consider whether the shared subset could live in `bundler-utils` as a minimal `PluginContextBase`.
- Note: This is a design tradeoff acknowledged in the PR description. Keeping inline interfaces is defensible for avoiding peer dependency on bundler types. Flagging for awareness, not as a hard requirement.

## Pre-existing Issues (Not Blocking)

_No pre-existing CRITICAL issues found in surrounding code._

## Suggestions (Lower Confidence)

- **Missing `noUncheckedIndexedAccess` verification** - `packages/bundler-utils/tsconfig.json` (Confidence: 65%) -- The tsconfigs extend `../../tsconfig.base.json` which does not exist at the repo root. Unable to verify whether `strict` and `noUncheckedIndexedAccess` are enabled. The code uses `str[i]!` (non-null assertion on indexed access) in `escapeForJs`, suggesting the author may already be aware of this, but the base config should be verified.

- **`escapeForJs` does not handle NUL bytes or other control characters** - `packages/bundler-utils/src/transform.ts:6-22` (Confidence: 62%) -- Only `\n`, `\r`, `\\`, `"`, U+2028, and U+2029 are escaped. Other control characters (e.g., `\0`, `\b`, `\f`, `\t`) pass through unescaped. For compiler output this is likely fine, but a `\0` in output would produce an invalid JS string literal.

- **Webpack loader options race on first call** - `packages/webpack-loader/src/index.ts:15-27` (Confidence: 68%) -- The `ensureTransformer` function accepts `options` from `this.getOptions()` but only uses the options from the _first_ caller. If two webpack loader invocations race and the first provides different options than the second, the second caller's options are silently ignored. This is inherent to the singleton pattern and likely acceptable for webpack loaders where options are typically uniform, but worth documenting.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 3 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The type design is solid overall -- good use of `import type`, structural interfaces, discriminated patterns in `FormattedError`, and proper `unknown` typing for error parameters. The main concerns are: (1) the poisoned-init-promise bug which can cause permanent failure after a transient error, (2) committed dist artifacts inflating the repo, and (3) the unused `isMdsError` member on the `MdsApi` interface which adds a dead requirement to all consumers.

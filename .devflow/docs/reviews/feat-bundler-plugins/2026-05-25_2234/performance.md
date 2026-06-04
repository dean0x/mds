# Performance Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25
**Prior Resolutions**: PERF-1 (redundant cleanId removed) -- confirmed fixed. PERF-2 (full-reload HMR) -- confirmed documented with rationale comment.

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

- **shouldTransform I/O for .md files could be cached in watch mode** - `packages/bundler-utils/src/frontmatter.ts:42` (Confidence: 65%) -- Every `.md` file the bundler resolves triggers a 512-byte file read to check for `type: mds` frontmatter. In projects with many `.md` files (e.g., docs-heavy monorepos), this adds an `open()/read()/close()` syscall triple per file per build. A per-build Map cache keyed by cleaned id would eliminate redundant reads in watch mode. Not actionable now since the current 512-byte peek is already quite efficient and caching introduces staleness concerns.

- **Full-reload HMR on .mds change** - `packages/vite-plugin/src/index.ts:103` (Confidence: 70%) -- Full page reload on any `.mds` file change is heavier than targeted module invalidation. The comment at line 96-100 documents this as intentional for v0.1.0 with a future optimization path. Previously raised as PERF-2 and acknowledged as documented. No action needed this cycle.

- **await on synchronous shouldTransform return** - `packages/vite-plugin/src/index.ts:69`, `packages/rollup-plugin/src/index.ts:62` (Confidence: 60%) -- `shouldTransform()` returns `true` synchronously for `.mds` files but the caller always `await`s. For the common `.mds` case, `await true` creates a microtask. The cost is negligible in practice (V8 optimizes trivial awaits), and the alternative (type-narrowing the return) would add complexity for no meaningful gain.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

The bundler plugin implementation demonstrates strong performance practices:

1. **Regex compilation at module scope** -- Both `JS_ESCAPE_RE` and `SAFE_JSON_RE` are compiled once at module load, not per-transform call. This avoids the overhead of repeated regex construction.

2. **Init promise caching** -- The `ensureInit()` pattern in `createMdsTransformer` uses a cached promise with a fast-path boolean (`initialized`) to skip promise machinery after first init. Concurrent calls share the same promise. Failed init resets the promise for retry. This is the correct pattern.

3. **512-byte peek for frontmatter** -- `shouldTransform` reads only the first 512 bytes of `.md` files rather than loading the entire file. This is a deliberate and effective optimization for large `.md` files.

4. **Redundant cleanId removed (PERF-1)** -- The prior cycle's PERF-1 finding is confirmed resolved: `transform()` in `bundler-utils/src/transform.ts` no longer calls `cleanId()` internally. Callers (vite-plugin, rollup-plugin) call `cleanId()` once and pass the cleaned id. Webpack's loader uses `resourcePath` directly (already clean). No double-cleaning.

5. **PERF-2 documented** -- The full-reload HMR strategy is now documented with clear rationale in a code comment at `vite-plugin/src/index.ts:96-100`, confirming it as an intentional v0.1.0 choice with a path to future optimization.

6. **Module-level singleton in webpack** -- `ensureTransformer()` in the webpack loader uses a module-level singleton with promise-based init and an invariant assertion, ensuring the compiler is initialized exactly once across all loader invocations.

7. **safeJsonForJs uses native regex replace** -- Despite the PR description mentioning "char-by-char loop", the implementation correctly uses `String.prototype.replace()` with a pre-compiled regex, which delegates scanning to the native regex engine. This is the idiomatic and performant approach.

No blocking or should-fix performance issues found. The implementation follows performance best practices throughout.

# Reliability Review Report

**Branch**: feat-17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25

## Issues in Your Changes (BLOCKING)

No CRITICAL or HIGH reliability issues found.

### MEDIUM

**Silent empty-source fallback masks missing entry file in modules map** - `packages/mds/src/node.ts:73`
**Confidence**: 82%
- Problem: The expression `modules[entryFilename] ?? ''` silently falls back to an empty string if the entry filename is not found in the modules map. While `buildModulesMap` should always populate this key, the fallback hides a potential invariant violation: if the entry source is missing, the WASM compiler receives an empty string instead of failing fast. This contradicts the assertion density principle -- a broken invariant should surface immediately rather than produce incorrect empty output.
- Fix: Replace the silent fallback with an explicit assertion:
  ```typescript
  const source = modules[entryFilename];
  if (source === undefined) {
    throw new Error(
      `@mds/mds: entry file "${entryFilename}" not found in modules map — this is a bug in buildModulesMap`,
    );
  }
  delete modules[entryFilename];
  ```

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

No CRITICAL pre-existing reliability issues found in the reviewed files. The existing codebase demonstrates strong reliability patterns:

- `buildModulesMap` has bounded recursion (MAX_IMPORT_DEPTH = 64), bounded module count (DEFAULT_MAX_MODULES = 256), and bounded aggregate size (DEFAULT_MAX_AGGREGATE_SIZE = 10 MiB).
- WASM init has bounded retries (MAX_INIT_RETRIES = 3, MAX_BROWSER_RETRIES = 3) with exhaustion tracking.
- File handles use try/finally for cleanup.
- Subprocess tests have a 30-second timeout.

## Suggestions (Lower Confidence)

- **No stderr capture in test helper** - `packages/mds/__test__/wasm-compileFile.spec.mjs:29` (Confidence: 65%) -- When `runScript` spawns a subprocess that crashes or prints warnings to stderr, the error message from the rejected promise includes stderr, but `JSON.parse(stdout)` on line 35 could throw with a confusing `SyntaxError` if the subprocess exits 0 with empty stdout. Consider adding a guard: `if (!stdout.trim()) throw new Error('subprocess produced no output');` before `JSON.parse`.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The core bug fix (removing the entry file from the modules map to prevent `mds::filename_collision`) is correct and well-documented with comments. The `prepareFileArgs` refactoring properly deduplicates the fix logic between `compileFile` and `checkFile`. All existing reliability bounds (recursion depth, module count, aggregate size, retry limits, subprocess timeouts) are preserved. The single condition is to replace the silent `?? ''` fallback with an explicit assertion, which converts a latent silent-corruption risk into a fail-fast invariant check.

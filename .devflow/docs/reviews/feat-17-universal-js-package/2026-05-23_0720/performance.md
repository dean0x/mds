# Performance Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23T07:20
**Cycle**: 3 (incremental — prior cycles resolved 18/21 issues)

## Cross-Cycle Awareness

Prior resolutions relevant to this cycle:
- **Sequential-to-parallel I/O in module-scanner**: VERIFIED FIXED. `lstat` and `realpath` are now called via `Promise.all` (module-scanner.ts:158-161), and child imports are parallelized via `Promise.all` (module-scanner.ts:203-212).
- **Frozen default options to avoid object spread on every compile/check**: VERIFIED FIXED. `DEFAULT_COMPILE_OPTS` is `Object.freeze`d at module level (wasm.ts:106) and reused without spread on the no-vars path (wasm.ts:117, 123).

No regressions detected from prior fixes.

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Frozen `modules` object shared across compile/check calls may accumulate stale references** - `wasm.ts:106`
**Confidence**: 65%
- Moved to Suggestions (below threshold).

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**WASM `_init` tries candidates sequentially with synchronous `require()`** - `wasm.ts:75-89`
**Confidence**: 82%
- Problem: The WASM init loop tries `require(candidate)` for each candidate path sequentially. The first `require()` call that fails triggers a filesystem search (CJS module resolution) before moving to the next candidate. In cold-start scenarios (serverless, worker init), this adds latency from failed `require()` resolution on the first candidate path before falling back.
- Impact: Adds ~5-50ms to cold-start time depending on filesystem speed. Not an issue for long-running processes where init happens once.
- Fix: This is a pre-existing architectural choice (CJS require for WASM loading). The two-candidate approach is reasonable for the current use case. Consider lazy-loading or parallel candidate resolution if cold-start performance becomes measurable. No action needed now.

## Suggestions (Lower Confidence)

- **`DEFAULT_COMPILE_OPTS.modules` is a frozen empty object shared by reference** - `wasm.ts:106` (Confidence: 65%) — The frozen `modules: {}` in `DEFAULT_COMPILE_OPTS` is passed to WASM `compile`/`check` on every no-vars call. If the WASM FFI layer mutates this object (adding entries during compilation), it would fail silently or throw on a frozen object. This is actually a safety benefit of `Object.freeze`, but worth verifying that the WASM layer does not attempt to write to the modules object. If it does, each call would need its own copy.

- **`varsOpt` creates a new `{ vars }` wrapper object on every call with vars** - `options.ts:11` (Confidence: 65%) — Each `compile(source, { vars })` call where vars is non-null creates an intermediate `{ vars: options.vars }` object, which is then spread into `{ ...DEFAULT_COMPILE_OPTS, ...vars }` at wasm.ts:117, creating a second object. This is two allocations per call. For a library that may be called in tight loops (batch compilation), this could add up. However, the WASM FFI boundary itself is likely orders of magnitude more expensive, making these allocations negligible in practice.

- **`compileFile`/`checkFile` duplicate the `buildModulesMap` + `wasm.compile` pattern** - `wasm.ts:126-143` (Confidence: 62%) — Both `compileFile` and `checkFile` independently call `buildModulesMap` with `(src) => wasm.scanImports(src)` creating a new closure each time. The closure itself is trivial (no captured state beyond `wasm`), but if a user calls `compileFile` then `checkFile` on the same file, the entire module tree is scanned and read from disk twice. A shared cache of resolved module maps keyed by entry path would eliminate this redundancy. However, this is an uncommon usage pattern and the current approach is correct for the single-call case.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED

### Rationale

This PR demonstrates strong performance awareness. The key improvements from prior review cycles are well-implemented:

1. **Parallel I/O** in module-scanner (`Promise.all` for `lstat`/`realpath`, parallel child scanning) eliminates the sequential bottleneck that was the primary performance concern.
2. **Frozen default options** (`DEFAULT_COMPILE_OPTS`) avoids per-call object allocation on the hot path (compile/check with no vars).
3. **Browser init deduplication** — concurrent `init()` calls share the same promise, preventing redundant WASM initialization.
4. **`init()` changed from `async function` to synchronous function returning `Promise`** — eliminates the async state machine overhead when the backend is already initialized (`resolvedBackend !== undefined` returns `Promise.resolve()` immediately).
5. **Depth limit** on module scanning (MAX_IMPORT_DEPTH=64) prevents stack overflow from deep import chains without adding per-call overhead.

No blocking or should-fix performance issues were found in the changed code. The suggestions are all below the 80% confidence threshold and relate to micro-optimizations that would not produce measurable improvement given the WASM FFI boundary dominates call latency.

# Architecture Review Report

**Branch**: refactor-27-28-unified-backend-architecture -> main
**Date**: 2026-05-24T12:06

## Issues in Your Changes (BLOCKING)

### HIGH

**wrapWithFileOps bypasses base backend methods for compile/check in file operations** - `packages/mds/src/node.ts:67,72`
**Confidence**: 85%
- Problem: `wrapWithFileOps()` spreads `...base` to inherit `compile` and `check` from the `MdsBaseBackend`, but `compileFile` and `checkFile` call `wasmModule.compile()` and `wasmModule.check()` directly (lines 67, 72) instead of delegating to `base.compile()` / `base.check()`. This means any options normalization performed by `compileOpts()` inside the base backend's `compile`/`check` methods (e.g., the frozen `DEFAULT_COMPILE_OPTS` singleton) is bypassed for file operations. The `fileOpts()` helper constructs its own options object, so the behavior happens to be correct today, but the architectural intent of the wrapper pattern is violated: the base backend's `compile`/`check` are dead code in the file-ops path, and a future change to `compileOpts()` normalization would silently diverge from `fileOpts()`.
- Fix: Have `compileFile`/`checkFile` delegate to `base.compile()` / `base.check()` after building the modules map, passing file-level options through the base backend's normalization. If this is intentional because the WASM module needs raw options for file operations, document the divergence with a comment.

### MEDIUM

**initWasmBrowser has no retry exhaustion like initWasmNode** - `packages/mds/src/backend/wasm.ts:206-215`
**Confidence**: 85%
- Problem: `initWasmNode()` implements a circuit breaker pattern with `MAX_INIT_RETRIES` (3 retries before permanent failure). `initWasmBrowser()` has no such limit — on each failure the cached promise is cleared and the next call retries indefinitely. The JSDoc on line 203-204 acknowledges this ("simpler than Node.js") but the asymmetry is an architectural inconsistency. In a browser environment, an infinite retry loop against a bad `wasmUrl` could cause repeated network requests and resource waste, whereas the Node.js path fails fast.
- Fix: Add a `browserFailures` counter and `MAX_INIT_RETRIES` check to `initWasmBrowser()` to match the Node.js circuit breaker pattern. Alternatively, explicitly document the design decision that browser retries are unbounded and why this is acceptable.

**FileOptions import unused in wasm.ts after file ops moved to node.ts** - `packages/mds/src/backend/wasm.ts:6`
**Confidence**: 90%
- Problem: `FileOptions` is imported in `wasm.ts` (line 6) but is only used in the `fileOpts()` helper (line 298), which is now exported for consumption by `node.ts`. While not a runtime issue, `wasm.ts` still imports and re-exports file-operation plumbing (`fileOpts`) that conceptually belongs to the Node-only layer. This partially undermines the stated goal of keeping `wasm.ts` browser-safe — the type import is harmless but the `fileOpts` function is dead code when used from the browser entry.
- Fix: Move `fileOpts()` to `node.ts` (where it is consumed) or to a shared `util/options.ts` module. This keeps `wasm.ts` focused on WASM module lifecycle and backend creation only.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**browser.ts getBackend() bypasses assertInitialized() — inconsistent with contract** - `packages/mds/src/browser.ts:89-91`
**Confidence**: 82%
- Problem: `browser.ts:getBackend()` returns `'wasm'` as a hardcoded constant without calling `assertInitialized()`. While functionally correct (browser is always WASM), it violates the pattern established by `node.ts` where `getBackend()` calls `assertReady()` (line 200). A consumer who checks `getBackend()` before `init()` would get a valid response in the browser entry but an error in the Node entry — inconsistent behavior across entry points for the same API shape.
- Fix: Either (a) call `assertInitialized()` in `browser.ts:getBackend()` for consistency with the Node entry's pre-init guard, or (b) document the intentional divergence (browser backend is always known statically).

**Asymmetric `_resetForTesting` — node.ts does not reset WASM-layer state** - `packages/mds/src/node.ts:41-44` and `packages/mds/src/backend/wasm.ts:56-60`
**Confidence**: 80%
- Problem: `node.ts:_resetForTesting()` clears `backend` and `initPromise` but does not call `wasm._resetForTesting()`. If a test resets the node layer and re-initializes with WASM, the wasm-layer singleton (`cachedNodePromise`) may still hold a stale promise from a previous init. The test file `backend.spec.mjs` works around this by calling `init()` again (which shares the existing wasm promise), but a test that needs to exercise a fresh WASM init after a node reset would silently reuse cached state.
- Fix: Have `node.ts:_resetForTesting()` also call `wasm._resetForTesting(0)` to ensure full state isolation, or document that node-layer reset intentionally preserves wasm-layer caching.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**createNativeBackend wraps sync operations in async for compileFile/checkFile** - `packages/mds/src/backend/native.ts:39-45`
**Confidence**: 80%
- Problem: `createNativeBackend` wraps `addon.compileFile()` and `addon.checkFile()` in `async` functions that simply return the synchronous napi result. The native addon does its own file I/O synchronously within the napi call, but the `MdsNodeBackend` interface requires `Promise<CompileResult>`. This means the native backend's file operations block the event loop — a concern for server-side usage with large files. Not introduced by this PR, but the interface split makes this pattern more visible.

## Suggestions (Lower Confidence)

- **index.ts barrel re-exports WASM internals at the package level** - `packages/mds/src/index.ts:15-16` (Confidence: 70%) — `initWasmNode`, `initWasmBrowser`, `createWasmBackend`, and `WasmModule` are exported from the barrel `index.ts`. These are implementation details of the backend layer. Consumers who use the `node.ts` or `browser.ts` entry points never need these. If `index.ts` is intended as a "power-user" barrel for advanced integrations, this is acceptable; if it is the primary public API, these exports leak internal architecture.

- **MdsBackend deprecated alias provides no migration path enforcement** - `packages/mds/src/types.ts:84-88` (Confidence: 65%) — The `@deprecated` tag on `MdsBackend` type alias is informational only. Since this is a pre-release project (zero users per MEMORY.md), the deprecated alias could be removed entirely rather than maintained as dead weight.

- **module-scanner.ts `realpath` call inside `openAndValidateModule` re-opens TOCTOU on Windows** - `packages/mds/src/util/module-scanner.ts:174` (Confidence: 65%) — On Windows where `O_NOFOLLOW=0`, the `realpath()` call on line 174 operates on the path string, not the file descriptor. Between `open()` and `realpath()`, the path could be swapped. The comment on lines 185-187 acknowledges this but the mitigation is weaker than the Linux path.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The interface split (MdsBaseBackend / MdsNodeBackend) is a well-executed application of Interface Segregation Principle (ISP). The separation of browser-safe and Node-specific concerns into distinct interfaces with a clean inheritance hierarchy is architecturally sound. The removal of top-level await in favor of explicit `init()` + `assertReady()` is a good DIP improvement — consumers control initialization timing rather than being forced into TLA semantics.

The sync factory pattern (`createWasmBackend(wasmModule)` mirroring `createNativeBackend(addon)`) correctly applies constructor injection. The split of `initWasmNode` / `initWasmBrowser` properly separates platform-specific initialization while sharing the factory.

Conditions for approval: Address the HIGH-severity `wrapWithFileOps` bypass finding (clarify or fix the delegation path). The MEDIUM items are worth addressing but are not blocking.

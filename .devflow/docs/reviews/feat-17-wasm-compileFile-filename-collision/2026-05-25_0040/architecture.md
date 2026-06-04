# Architecture Review Report

**Branch**: feat-17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25
**PR**: #30

## Issues in Your Changes (BLOCKING)

### MEDIUM

**buildModulesMap return type leaks implementation responsibility to callers** - `packages/mds/src/node.ts:68-76`
**Confidence**: 82%
- Problem: `buildModulesMap()` returns a `modules` map that includes the entry file under its own filename key. Callers must know to extract and delete the entry before passing to WASM. This is a leaky abstraction (ISP/SRP concern) -- the fix in `prepareFileArgs` works correctly, but the real contract ambiguity lives in `buildModulesMap`'s return type. The `BuildModulesMapResult` interface (`{ entryFilename, modules }`) does not document whether `modules` includes or excludes the entry, forcing every consumer to handle the collision defensively.
- Fix: Add a JSDoc comment to `BuildModulesMapResult.modules` documenting that it *includes* the entry file source keyed by `entryFilename`. Alternatively, consider having `buildModulesMap` return `{ entryFilename, entrySource, imports }` where `imports` excludes the entry -- making the contract unambiguous and eliminating the need for the `delete` workaround. This is a non-blocking suggestion because the current `prepareFileArgs` approach is correct and well-documented with inline comments.

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Asymmetric backend architectures for native vs WASM file operations** - `packages/mds/src/backend/native.ts:39-45` vs `packages/mds/src/node.ts:58-91`
**Confidence**: 83%
- Problem: The native backend delegates `compileFile`/`checkFile` directly to the napi addon (which handles file I/O and import resolution internally in Rust). The WASM backend requires a JS-side `wrapWithFileOps` layer that does file I/O, import resolution via `buildModulesMap`, and entry extraction before calling the WASM `compile`/`check` functions. This architectural asymmetry means the two backends have fundamentally different trust boundaries and error surface areas for file operations. The native backend's file operations are atomic (Rust handles everything), while the WASM backend's are split across JS and Rust with a mutable intermediate state (`delete modules[entryFilename]`).
- This is an inherent consequence of WASM not having filesystem access and is well-documented in the codebase comments. No action needed, but worth noting for future maintainers that changes to `buildModulesMap` or the WASM `build_modules()` function must be coordinated.

## Suggestions (Lower Confidence)

- **`prepareFileArgs` is a closure capturing `wasmModule` -- consider making it a standalone function** - `packages/mds/src/node.ts:68` (Confidence: 62%) -- `prepareFileArgs` is defined inside `wrapWithFileOps` to capture `wasmModule` via closure. This is a reasonable pattern for a small helper, but if the function grows, extracting it to module scope with an explicit `wasmModule` parameter would improve testability.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED

### Rationale

The fix is architecturally sound. The core bug (WASM `build_modules()` colliding with the entry key in the modules map) is addressed correctly by extracting the entry source and removing it from the map before passing to WASM. The solution follows good patterns:

1. **DRY via `prepareFileArgs`** -- The duplicated extract-and-delete logic is consolidated into a single helper, keeping `compileFile` and `checkFile` thin delegators.
2. **Well-documented invariant** -- The JSDoc on `prepareFileArgs` clearly explains *why* the deletion is necessary, linking it to the WASM `build_modules()` behavior.
3. **Safe mutation** -- The comment in the commit message notes that `modules` is a fresh allocation per `buildModulesMap()` call, so the `delete` does not affect shared state.
4. **Separation of concerns preserved** -- File I/O stays in `node.ts` (Node-only), WASM module stays browser-safe, and the types layer remains clean.
5. **Good test architecture** -- Subprocess isolation for backend tests prevents singleton contamination, and parity tests (U-WCF7, U-WCF8) validate that WASM and native backends produce identical results.

The one medium-severity blocking note is about `buildModulesMap`'s underdocumented return contract rather than a defect in this PR's code. The `package.json` cleanup (removing `wasm/` from `files`) is a correct housekeeping change.

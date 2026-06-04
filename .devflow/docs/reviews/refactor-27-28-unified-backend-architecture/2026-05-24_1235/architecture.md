# Architecture Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24
**Scope**: Incremental review (4 commits: c57685c...HEAD)

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Redundant shape check after `validateWasmShape` in `_initBrowser`** - `packages/mds/src/backend/wasm.ts:272`
**Confidence**: 85%
- Problem: After `validateWasmShape(imported)` succeeds on line 269 (which asserts `mod is WasmModule`), the code immediately checks `typeof wasmMod.default !== 'function'` on line 272. However, `WasmModule.default` is typed as optional (`default?: (input?: unknown) => Promise<void>`), so the check is correct at the type level. The concern is subtle: `validateWasmShape` validates `compile`, `check`, and `scanImports` but does NOT validate `default`. This means the `WasmModule` type assertion from the `asserts` function grants trust over `default` without verifying it. The check on line 272 does catch this, but the architecture would be cleaner if `validateWasmShape` handled the browser-specific `default` requirement as an optional second parameter or a separate browser shape validator, since `_initBrowser` always requires `default` while `tryLoadCandidate` (Node.js) treats it as optional.
- Fix: This is an architectural observation rather than a bug -- the runtime check on line 272 does protect against the gap. However, consider either: (a) adding a `requireDefault` parameter to `validateWasmShape`, or (b) creating a `validateBrowserWasmShape` that extends the base check with the `default` requirement. This would centralize all shape validation in one place rather than splitting it between `validateWasmShape` and `_initBrowser`.

## Issues in Code You Touched (Should Fix)

_No issues found._

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Module-level mutable singletons in `wasm.ts`** - `packages/mds/src/backend/wasm.ts:32-42`
**Confidence**: 82%
- Problem: Four module-level mutable variables (`cachedNodePromise`, `nodeFailures`, `cachedBrowserPromise`, `browserFailures`) manage state as loose module globals. While this is a common pattern in JS/TS for singletons, it creates tight coupling between the init functions and the shared state, making the module harder to test (requiring `_resetForTesting` to exist) and impossible to run two independent WASM instances simultaneously. This is a mild DIP violation: the init functions directly depend on the concrete module-level state rather than receiving state through injection.
- Note: The new `browserFailures` counter added in this PR follows the established pattern for `nodeFailures`, which is the correct consistency choice. This is a pre-existing architectural concern, not a new issue.

## Suggestions (Lower Confidence)

- **`openAndValidateModule` leaks handle responsibility to caller** - `packages/mds/src/util/module-scanner.ts:172` (Confidence: 72%) -- The refactored `openAndValidateModule` now returns an open file handle that the caller (`scan`) must close in two different code paths (aggregate size failure and normal read). This split-responsibility pattern increases the surface area for resource leaks compared to the original self-contained function. The current code handles it correctly with explicit `handle.close()` calls, but the design would be more robust with a callback pattern or returning a disposable.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED

## Rationale

The changes in this incremental review are architecturally well-structured:

1. **DRY improvement** (validateWasmShape extraction): The inline shape check in `tryLoadCandidate` and the separate browser shape validation are unified into a single `validateWasmShape` function with an `asserts` return type. This is a clean application of the Extract Function refactoring that improves both Node.js and browser code paths. The function uses TypeScript's type assertion signature correctly.

2. **Consistent guard naming** (assertInitialized -> assertReady): Renaming `assertInitialized()` to `assertReady()` in `browser.ts` to match `node.ts` is a pure consistency improvement with zero behavioral change. Both entry points now use the same convention.

3. **Clean separation of concerns** (openNoFollow extraction): Moving the O_NOFOLLOW open + ELOOP translation into a module-level `openNoFollow` helper reduces nesting in `openAndValidateModule` and isolates a single responsibility. The original function was doing too many things; now each piece has a clear purpose.

4. **Security-first resource control** (aggregate size before readFile): Splitting `openAndValidateModule` into open+validate and read phases so the aggregate size check runs before `readFile` is an architecturally sound decision. It bounds worst-case memory allocation by checking metadata before loading content, following the "validate at boundaries" principle.

5. **Browser/Node symmetry** (browser circuit breaker): The `initWasmBrowser` circuit breaker mirrors the existing `initWasmNode` pattern exactly. Consistent patterns across the Node/browser split reduce cognitive load and make the module's retry semantics predictable.

6. **Test state isolation** (try/finally in U-B6): Wrapping the test's assertion in try/finally to guarantee `init()` re-runs regardless of assertion outcome is a reliability improvement that prevents test state leakage.

No SOLID violations, no circular dependencies, and dependency direction is correct throughout (browser.ts -> backend/wasm.ts -> types.ts). The interface hierarchy (MdsBaseBackend / MdsNodeBackend) is properly respected with no LSP or ISP violations.

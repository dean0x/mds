# Code Review Summary

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23_1153
**Cycle**: 4 (incremental from cycle 3 — 19/21 fixed, 1 FP aggregateSize, 1 deferred LSP tension)

## Merge Recommendation: CHANGES_REQUESTED

This PR requires fixes to two blocking issues before merge. Both are regressions introduced by cycle-3 refactoring:

1. **tryLoadCandidate error swallowing** — Silent catch of all exceptions contradicts JSDoc, hides real WASM init failures
2. **browser.ts retry bypass** — Permanently caches rejected promise, making wasm.ts retry logic unreachable

All 78 tests pass. The refactoring is well-structured and introduces no new exports or signature changes.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| **Blocking** | 0 | 5 | 3 | 0 |
| **Should Fix** | 0 | 0 | 6 | 0 |
| **Pre-existing** | 0 | 0 | 3 | 2 |
| **Total** | **0** | **5** | **12** | **2** |

---

## Blocking Issues (MUST FIX)

### HIGH — tryLoadCandidate swallows all errors (DEDUPED: 6 reviewers)

**Location**: `packages/mds/src/backend/wasm.ts:75-96`
**Confidence**: 90% (flagged by: architecture, consistency, documentation, regression, reliability, security, typescript)

**Problem**: The extracted `tryLoadCandidate()` function catches *all* exceptions and returns `null`, but the JSDoc claims "Re-throws unexpected errors so the caller can surface them." This is a documentation-to-implementation mismatch. More critically, when the WASM module is found but fails to initialize (corrupted binary, OOM, invalid wasmUrl, supply-chain tampering), the error is silently discarded. The caller then moves to the next candidate or throws the generic "failed to load WASM module. Build it first..." message, which is misleading when the module exists but cannot initialize.

**Impact**:
- Debugging becomes significantly harder — users with correctly-located but broken WASM binaries get "build it first" instead of the real error
- Security issue: tampered or corrupted modules are silently treated as "not found"
- Breaks JSDoc contract, misleading future maintainers about re-throw semantics

**Fix**: Distinguish "module not found" (MODULE_NOT_FOUND error code) from unexpected errors. Only the former should return null:

```typescript
async function tryLoadCandidate(
  candidate: string,
  require: NodeRequire,
  wasmUrl: InitOptions['wasmUrl'],
): Promise<WasmModule | null> {
  try {
    const mod = require(candidate) as WasmModule;
    if (typeof mod.default === 'function') {
      await mod.default(wasmUrl);
    }
    return mod;
  } catch (err: unknown) {
    // MODULE_NOT_FOUND means this candidate path doesn't exist — try the next.
    if (
      err instanceof Error &&
      'code' in err &&
      (err as NodeJS.ErrnoException).code === 'MODULE_NOT_FOUND'
    ) {
      return null;
    }
    // All other errors (WASM init failure, OOM, etc.) are unexpected — re-throw.
    throw err;
  }
}
```

Also restore error context in the `_init` error message to help users debug when all candidates fail:

```typescript
// Track last error for better diagnostics
let lastError: unknown;
for (const candidate of candidates) {
  try {
    const mod = await tryLoadCandidate(candidate, require, options?.wasmUrl);
    if (mod !== null) { wasmModule = mod; return; }
  } catch (e) { lastError = e; }
}
throw new Error(
  `@mds/mds: failed to load WASM module. Build it first with: wasm-pack build crates/mds-wasm --target nodejs --out-dir pkg.${lastError ? ' ' + String(lastError) : ''}`,
);
```

---

### HIGH — browser.ts init() permanently caches rejected promise (2 reviewers: reliability, architecture)

**Location**: `packages/mds/src/browser.ts:43-47`
**Confidence**: 95%

**Problem**: Cycle 3 removed the `.catch()` handler that reset `initVoidPromise = null` on failure, intending to delegate retry logic to wasm.ts. However, browser.ts caches `initVoidPromise` at the entry point (line 44), while wasm.ts's retry reset happens inside `init()` (wasm.ts:69). Result: after a rejection, `browser.init()` always hits `if (initVoidPromise !== null) return initVoidPromise` and returns the stale rejected promise forever. wasm.ts's retry counter is never consulted from browser environments.

**Impact**: A single transient WASM load failure (network timeout, CDN hiccup) permanently breaks the browser SDK for the page lifetime. User must reload the page to recover, even though up to 3 retries were designed.

**Fix**: Re-introduce the `.catch()` handler that clears the cache, allowing wasm.ts's retry logic to execute:

```typescript
export function init(options?: InitOptions): Promise<void> {
  if (resolvedBackend !== undefined) return Promise.resolve();
  if (initVoidPromise !== null) return initVoidPromise;
  initVoidPromise = createWasmBackend(options)
    .then((b) => {
      resolvedBackend = b;
    })
    .catch((err) => {
      // Clear so subsequent calls re-enter wasm.ts's retry logic.
      // wasm.ts's MAX_INIT_RETRIES enforces the permanent failure bound.
      initVoidPromise = null;
      throw err;
    });
  return initVoidPromise;
}
```

---

### MEDIUM — Inconsistent options-building pattern (3 reviewers: architecture, consistency, performance)

**Location**: `packages/mds/src/backend/wasm.ts:155-182`
**Confidence**: 85%

**Problem**: `compile()` and `check()` use the new `compileOpts()` helper, but `compileFile()` and `checkFile()` inline the options-building with `...varsOpt(options)` spread. This creates two divergent code paths for "build WASM options." The `compileOpts()` function is named generically but only covers string-source paths; it was not extended to cover file-based paths.

**Impact**: Violation of Single Responsibility Principle. Any future changes to option construction must be replicated in two places. Performance: the file methods still create intermediate wrapper objects.

**Fix**: Either rename `compileOpts` narrowly to signal its limited scope, or extract a parallel helper for file-based paths:

```typescript
function buildFileOpts(
  entryFilename: string,
  modules: Record<string, string>,
  options?: FileOptions,
): { filename: string; modules: Record<string, string>; vars?: Record<string, unknown> } {
  const vars = varsOpt(options);
  return vars !== undefined 
    ? { filename: entryFilename, modules, vars: vars.vars } 
    : { filename: entryFilename, modules };
}
```

Then use consistently in `compileFile()` and `checkFile()`.

---

### MEDIUM — Double object allocation in compileOpts with vars (performance)

**Location**: `packages/mds/src/backend/wasm.ts:144-147`
**Confidence**: 82%

**Problem**: When vars are provided, `compileOpts` spreads both `DEFAULT_COMPILE_OPTS` (frozen) and the `{ vars }` wrapper from `varsOpt()`, creating two allocations per call:
```typescript
return vars !== undefined ? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS;
```

**Impact**: For high-throughput compilation (batch template processing), allocation overhead accumulates.

**Fix**: Construct a single object directly without spreading frozen defaults:

```typescript
function compileOpts(options?: CompileOptions) {
  const vars = options?.vars;
  if (vars != null) {
    return { filename: 'input.mds', modules: DEFAULT_COMPILE_OPTS.modules, vars };
  }
  return DEFAULT_COMPILE_OPTS;
}
```

---

### MEDIUM — Undeclared `mds-wasm` runtime dependency (dependencies)

**Location**: `packages/mds/src/backend/wasm.ts:108` + `package.json`
**Confidence**: 85%

**Problem**: The WASM backend lists `'mds-wasm'` as a fallback candidate in `_init()`, meaning `require('mds-wasm')` will be attempted at runtime. However, `mds-wasm` is not declared in `package.json` under any dependency field (`dependencies`, `optionalDependencies`, or `peerDependencies`). Unlike `mds-napi` which is properly declared as an `optionalDependency`, this creates a "phantom dependency."

**Impact**: In a production npm install scenario (not workspace), this fallback will silently fail since the package is never installed. The code handles failure gracefully (returns null and continues), but the lack of declaration creates a hidden contract.

**Fix**: Either declare `mds-wasm` as an `optionalDependency`:

```json
{
  "optionalDependencies": {
    "mds-napi": "file:../../crates/mds-napi",
    "mds-wasm": "^0.1.0"
  }
}
```

Or add a clear forward-looking comment in the source:

```typescript
// Future: when mds-wasm is published as a standalone npm package
'mds-wasm',
```

---

## Should-Fix Issues (SHOULD ADDRESS)

### MEDIUM — tryLoadCandidate JSDoc contradicts implementation (2 reviewers: consistency, documentation)
**Location**: `packages/mds/src/backend/wasm.ts:78-79`
**Confidence**: 88%
(Captured in blocking HIGH issue above — requires same fix)

---

### MEDIUM — JSDoc missing for _init function (documentation)
**Location**: `packages/mds/src/backend/wasm.ts:98`
**Confidence**: 85%

Add JSDoc explaining its role:
```typescript
/**
 * Internal initialization: locate and load the WASM module from known candidate paths.
 *
 * Called by `init()` — callers should not invoke this directly.
 * Throws if no candidate can be loaded.
 */
async function _init(options?: InitOptions): Promise<void> {
```

---

### MEDIUM — JSDoc missing for assertInitialized function (documentation)
**Location**: `packages/mds/src/backend/wasm.ts:124`
**Confidence**: 80%

```typescript
/** Return the initialized WASM module, or throw if init() has not completed. */
function assertInitialized(): WasmModule {
```

---

### MEDIUM — Error context lost in _init error message (documentation, regression)
**Location**: `packages/mds/src/backend/wasm.ts:119-121`
**Confidence**: 82%

The previous error message included `${String(loadError)}` for diagnostics. The new version drops this context entirely. Fix captured in blocking HIGH fix above.

---

### MEDIUM — compileOpts return type manually spelled (typescript)
**Location**: `packages/mds/src/backend/wasm.ts:144`
**Confidence**: 82%

The return type duplicates structure already defined in the WasmModule interface. Remove the explicit return type and let TypeScript infer it:

```typescript
function compileOpts(options?: CompileOptions) {
  const vars = varsOpt(options);
  return vars !== undefined ? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS;
}
```

---

### MEDIUM — as WasmModule type assertion bypasses safety (typescript)
**Location**: `packages/mds/src/backend/wasm.ts:87`
**Confidence**: 80%

Add runtime shape check after loading to validate the module conforms to WasmModule interface:

```typescript
const mod = require(candidate) as Record<string, unknown>;
if (typeof mod.compile !== 'function' || typeof mod.check !== 'function' || typeof mod.scanImports !== 'function') {
  return null; // Not a valid WasmModule — try next candidate
}
```

---

## Testing Gaps (Require Test Fixes)

### HIGH — U-WB1 test name misleading (testing)
**Location**: `packages/mds/__test__/wasm-backend.spec.mjs:21-26`
**Confidence**: 85%

The test pre-seeds failures to 2 via `_resetForTesting(MAX_INIT_RETRIES - 1)` then calls `init()`. The test passes because the WASM module is actually loadable in the test environment, not because the circuit-breaker logic is correct. If WASM were unavailable, the test would fail even though the circuit breaker is working.

**Fix**: Update the test name to reflect its integration nature:

```javascript
test('U-WB1: init() attempts loading when failures are below the limit (requires WASM build)', async () => {
  _resetForTesting(MAX_INIT_RETRIES - 1);
  // Circuit breaker allows the attempt; success depends on WASM being built.
  await assert.doesNotReject(init());
});
```

---

### MEDIUM — Hardcoded MAX_INIT_RETRIES duplicates source constant (testing)
**Location**: `packages/mds/__test__/wasm-backend.spec.mjs:12`
**Confidence**: 85%

The test hardcodes `const MAX_INIT_RETRIES = 3` which duplicates `wasm.ts:28`. If source constant changes, the test silently tests the wrong threshold.

**Fix**: Add a comment acknowledging the duplication and cross-reference:

```javascript
// Mirror of MAX_INIT_RETRIES from wasm.ts — if this value drifts, U-WB2
// will fail to trigger the exhaustion path, surfacing the mismatch.
const MAX_INIT_RETRIES = 3;
```

---

### MEDIUM — Missing test for browser.ts permanent failure path (testing)
**Location**: `packages/mds/src/browser.ts:41-48`
**Confidence**: 85%

No browser test validates that init() caches a rejected promise permanently. U-BR6 tests concurrent success, but the failure path is untested. Mitigated by wasm.ts circuit-breaker tests, but browser.ts caching layer needs validation.

**Fix**: Add test for permanent-rejection semantics, or use subprocess test like U-B5.

---

### MEDIUM — U-WB afterEach singleton reset impacts global state (testing)
**Location**: `packages/mds/__test__/wasm-backend.spec.mjs:15-19`
**Confidence**: 82%

The `afterEach` hook resets the WASM module singleton globally. If `wasm-backend.spec.mjs` runs before other test files sharing the same ESM module singleton, the reset could leave wasm uninitialized for subsequent tests.

**Fix**: Verify test runner uses process-level isolation (`--experimental-test-isolation=process`), or add a comment documenting the dependency.

---

## Pre-existing Issues (NOT BLOCKING)

### MEDIUM — `scanImports` helper comment is outdated (documentation)
**Location**: `packages/mds/__test__/scanner.spec.mjs:22-26`
**Confidence**: 82%

The comment reads like unfinished draft ("Actually we need scan_imports" then switches). Not introduced in this PR.

---

### MEDIUM — TOCTOU window in statAndValidateModule (security, reliability)
**Location**: `packages/mds/src/util/module-scanner.ts:138-208`
**Confidence**: 82%

Gap between lstat/realpath validation and readFile is inherent to Node.js filesystem API limitations. Current approach (symlink check + realpath) is practical maximum. Not introduced in this PR.

---

### MEDIUM — `as object` and type assertions in node.ts (typescript)
**Location**: `packages/mds/src/node.ts:27-29`
**Confidence**: 85%

Similar to tryLoadCandidate assertion issue — native addon loaded without runtime validation. Pre-existing pattern.

---

### LOW — `mds-napi` uses `file:` protocol link (dependencies)
**Location**: `packages/mds/package.json:31`
**Confidence**: 82%

Workspace reference will break when published to npm. Pre-release project, will be blocking at publish time.

---

### LOW — Test file header comment range incomplete (documentation)
**Location**: `packages/mds/__test__/scanner.spec.mjs:2`
**Confidence**: 70%

Header says "U-S1 through U-S10" but omits U-SM1-U-SM5 (buildModulesMap tests). Minor consistency issue.

---

## Convergence Status

**High Confidence Issues** (80%+):
- tryLoadCandidate error swallowing: **6 reviewers converge** (architecture, consistency, documentation, regression, reliability, security, typescript)
- browser.ts retry bypass: **2 reviewers converge** (reliability, architecture)  
- Missing JSDoc (_init, assertInitialized): **2 reviewers** (documentation)
- compileOpts options duplication: **3 reviewers converge** (architecture, consistency, performance)

**Pattern Identified**: The cycle-3 refactoring (extracting tryLoadCandidate, removing browser catch) introduced both the error-swallowing and retry-bypass issues. Multiple review disciplines (security, reliability, regression, consistency) independently identified the same core problems.

**False Positives from Cycle 3**: 1 (aggregateSize atomicity — correctly dismissed as JS single-threaded)
**Deferred Items**: 1 (node.ts/browser.ts LSP tension — now surfaced as retry bypass issue)

---

## Positive Observations

1. **All 78 tests pass** — no test failures introduced
2. **No breaking exports** — backward compatible with prior release
3. **Well-structured refactoring** — deep-freeze, helper extraction, function decomposition improve maintainability
4. **Strong test coverage** — circuit-breaker tests (U-WB1, U-WB2), multi-candidate fallback tests, filesystem validation
5. **Consistent naming** — Test IDs follow U-prefix convention throughout
6. **Documentation mostly complete** — README, CHANGELOG, JSDoc generally thorough (gaps identified are targeted)

---

## Summary Statistics

- **Reviewers**: 11 independent disciplines
- **Total Issues Identified**: 20
- **Blocking (HIGH/CRITICAL)**: 5 HIGH (must fix before merge)
- **Should Fix**: 6 MEDIUM (recommended improvements)
- **Pre-existing**: 5 (informational, lower priority)
- **FP Rate from Cycle 3**: 4.8% (1 of 21)
- **Deferred Issues**: 1 (LSP tension — now clearly a retry bypass issue)

---

## Action Plan

**Before Merge:**
1. Fix tryLoadCandidate error discrimination (HIGH, security-blocking)
2. Fix browser.ts permanent promise cache (HIGH, reliability-blocking)
3. Declare or document mds-wasm dependency (MEDIUM, blocking)
4. Add _init and assertInitialized JSDoc (MEDIUM, documentation consistency)
5. Update U-WB1 test name for clarity (MEDIUM, test quality)

**Nice-to-Have (Post-Merge):**
- Extract buildFileOpts helper for consistency
- Remove explicit compileOpts return type
- Add runtime shape validation for tryLoadCandidate
- Document test runner isolation assumptions

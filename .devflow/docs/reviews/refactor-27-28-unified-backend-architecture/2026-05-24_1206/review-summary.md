# Code Review Summary

**Branch**: refactor-27-28-unified-backend-architecture -> main
**Date**: 2026-05-24T12:06
**Cycle**: 1

## Merge Recommendation: CHANGES_REQUESTED

This refactoring successfully removes top-level await, splits the backend interface hierarchy (MdsBaseBackend / MdsNodeBackend), and improves security with TOCTOU fixes in module-scanner. The architecture is well-structured and follows the stated design goals. However, **5 blocking issues** must be resolved before merge:

1. **Browser WASM init missing shape validation** (HIGH, security impact)
2. **Browser init retry exhaustion missing** (HIGH, reliability impact)
3. **wrapWithFileOps bypasses base backend methods** (HIGH, architectural consistency)
4. **Aggregate size check occurs after content is read** (HIGH, performance/reliability impact)
5. **Complexity: buildModulesMap at 169 lines** (HIGH, maintainability)

Additionally, **3 should-fix medium-severity issues** require attention, and **multiple documentation/consistency gaps** should be addressed.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| **Blocking** | 0 | 5 | 2 | 0 |
| **Should Fix** | 0 | 0 | 5 | 0 |
| **Pre-existing** | 0 | 0 | 1 | 1 |

**Subtotal Blocking Issues**: 7 (5 HIGH, 2 MEDIUM)
**Subtotal Should-Fix Issues**: 5 (all MEDIUM)
**Total Issues**: 14 (9 introduced, 3 pre-existing suggestions)

---

## Blocking Issues (Must Fix Before Merge)

### CRITICAL-TIER

None.

### HIGH-TIER

**1. Browser WASM init missing shape validation** - `packages/mds/src/backend/wasm.ts:232`
- **Reporters**: security, typescript (90% confidence each)
- **Problem**: `_initBrowser()` casts the dynamically imported WASM module directly to `WasmModule` without verifying that `compile`, `check`, and `scanImports` are present. The Node.js path (`tryLoadCandidate`) validates all three exports, but the browser path trusts the cast. A corrupted or mismatched bundled module would only fail at runtime deep in user code, not at initialization.
- **Impact**: Security boundary violation; FEATURE_KNOWLEDGE notes "validates WASM module shape at boundary"
- **Fix**: Add shape validation matching `tryLoadCandidate`:
  ```typescript
  if (
    typeof wasmMod.compile !== 'function' ||
    typeof wasmMod.check !== 'function' ||
    typeof wasmMod.scanImports !== 'function'
  ) {
    throw new Error(
      '@mds/mds: WASM module missing required exports (compile, check, scanImports). ' +
      'Ensure the correct version of mds-wasm is bundled.',
    );
  }
  ```

**2. Browser WASM init lacks retry exhaustion circuit breaker** - `packages/mds/src/backend/wasm.ts:206-215`
- **Reporters**: reliability (92% confidence), architecture, security, typescript
- **Problem**: Unlike `initWasmNode()` which has `nodeFailures` counter with `MAX_INIT_RETRIES=3`, `initWasmBrowser()` clears `cachedBrowserPromise` on every failure and allows unlimited retries. While each call terminates, the system never "gives up" on a permanently broken wasmUrl, allowing unbounded network requests and resource waste.
- **Impact**: Potential denial-of-self in browser environments; inconsistent with Node.js reliability pattern
- **Fix**: Add matching circuit breaker:
  ```typescript
  let browserFailures = 0;
  const MAX_BROWSER_RETRIES = 3;
  
  export async function initWasmBrowser(options?: InitOptions): Promise<WasmModule> {
    if (cachedBrowserPromise !== null) {
      return cachedBrowserPromise;
    }
    if (browserFailures >= MAX_BROWSER_RETRIES) {
      throw new Error(
        `@mds/mds: WASM browser backend failed to initialize after ${MAX_BROWSER_RETRIES} attempts. ` +
        `Check that wasmUrl is correct and accessible.`,
      );
    }
    cachedBrowserPromise = _initBrowser(options).catch((err) => {
      browserFailures += 1;
      cachedBrowserPromise = null;
      throw err;
    });
    return cachedBrowserPromise;
  }
  ```

**3. wrapWithFileOps bypasses base backend methods for compile/check** - `packages/mds/src/node.ts:67,72`
- **Reporters**: architecture (85% confidence)
- **Problem**: `compileFile` and `checkFile` call `wasmModule.compile()` and `wasmModule.check()` directly instead of delegating to `base.compile()` / `base.check()`. This bypasses any options normalization in the base backend (e.g., `DEFAULT_COMPILE_OPTS` singleton freeze). The architectural intent of the wrapper pattern is violated: base backend's `compile`/`check` are dead code for the file-ops path, and future normalization changes would silently diverge.
- **Impact**: Architectural inconsistency; potential for subtle bugs if base backend normalization changes
- **Fix**: Delegate through base backend after building modules map:
  ```typescript
  async compileFile(absolutePath: string, options?: FileOptions): Promise<CompileResult> {
    const { modules } = await buildModulesMap(absolutePath, (path) => base.scanImports(readFileSync(path, 'utf-8')));
    return base.compile(primaryContent, { ...options, modules });
  }
  ```

**4. Aggregate size check occurs after content is read into memory** - `packages/mds/src/util/module-scanner.ts:224-232`
- **Reporters**: performance (85% confidence), reliability (85% confidence)
- **Problem**: The TOCTOU fix changed `statAndValidateModule` (stat-only) + separate `readFile` to `openAndValidateModule` (open+stat+read combined). The aggregateSize check now happens *after* the file content is loaded into memory. For files at the edge of the aggregate limit, this wastes I/O and memory. Worst case: 256 modules * 10 MiB each = ~2.5 GB of memory allocation before aggregate check rejects.
- **Impact**: Resource exhaustion on pathological inputs; violated precondition from lines 226-230 comment ("pre-reserve file size ... before reading content")
- **Fix**: Split into validation (open+stat+realpath) and reading (readFile) phases, with aggregate check in between:
  ```typescript
  async function openAndValidateModule(absolutePath: string): Promise<{ handle: FileHandle; size: number }> {
    // ... open, stat, realpath checks ...
    return { handle, size: stats.size };
  }
  
  // In scan():
  const { handle, size: fileSize } = await openAndValidateModule(absolutePath);
  aggregateSize += fileSize;
  if (aggregateSize > maxAggregateSize) {
    await handle.close();
    throw new Error(`resource limit: aggregate module size exceeds ${maxAggregateSize} bytes`);
  }
  const content = await handle.readFile({ encoding: 'utf-8' });
  await handle.close();
  ```

**5. buildModulesMap spans 169 lines with 3 nested closures sharing mutable state** - `packages/mds/src/util/module-scanner.ts:91-259`
- **Reporters**: complexity (82% confidence)
- **Problem**: The outer function contains three nested closures (`validateImportPath`, `openAndValidateModule`, `scan`) capturing shared mutable state (`projectRoot`, `modules`, `visited`, `aggregateSize`). This PR added `openAndValidateModule` (50 lines, replacing the prior 30-line `statAndValidateModule`), growing the enclosing function by 20 lines. The nesting depth reaches 4 levels; readers must hold the entire 169-line scope to understand state mutations. While each closure is well-scoped individually, the aggregate complexity exceeds maintainability thresholds.
- **Impact**: Cognitive overload; difficult to test security checks in isolation; difficult to reason about state flow
- **Fix**: Extract `openAndValidateModule` into a helper returning the handle, reducing `openAndValidateModule` from 50 to ~35 lines:
  ```typescript
  async function openNoFollow(absolutePath: string): Promise<FileHandle> {
    try {
      return await open(absolutePath, constants.O_RDONLY | O_NOFOLLOW);
    } catch (err) {
      const code = (err as NodeJS.ErrnoException).code;
      if (code === 'ELOOP' || code === 'ENOTDIR') {
        throw new Error(`security: symlink detected at ${absolutePath}`);
      }
      throw err;
    }
  }
  ```

### MEDIUM-TIER (Blocking)

**6. Unused `lstat` import + stale JSDoc in module-scanner.ts** - `packages/mds/src/util/module-scanner.ts:1,6,86`
- **Reporters**: consistency (95% confidence), performance, typescript
- **Problem**: `lstat` is imported but never called (dead code). The refactor replaced `lstat`-based validation with `open() + handle.stat()`. JSDoc still says "Rejects symlinks (lstat check)" but uses `O_NOFOLLOW` instead.
- **Fix**: Remove `lstat` from import and update JSDoc:
  ```typescript
  import { open, realpath } from 'node:fs/promises';
  // JSDoc: "Rejects symlinks (O_NOFOLLOW / realpath check)"
  ```

**7. Test issues: U-PF0 measures subprocess overhead, not import time** - `packages/mds/__test__/perf.spec.mjs:21-35`
- **Reporters**: testing (85% confidence)
- **Problem**: Test name says "< 100ms" (import-time goal) but assertion checks 5000ms (subprocess overhead including V8 warmup, ESM resolution). The test is 50x more generous than the stated goal. TLA regression could happen at 100ms-5000ms range without failing the test.
- **Impact**: False confidence; doesn't catch real TLA regressions; misleading contract
- **Fix**: Either rename to accurately describe what it measures (`"module import + subprocess overhead completes within 5s"`) or measure import time inside subprocess:
  ```javascript
  const output = execFileSync(process.execPath, ['--input-type=module'], {
    input: `const { performance } = await import('node:perf_hooks');
  const t0 = performance.now();
  await import('../dist/node.js');
  console.log((performance.now() - t0).toFixed(2));`,
  });
  const importMs = parseFloat(output.trim());
  assert.ok(importMs < 100, `import took ${importMs}ms`);
  ```

---

## Should-Fix Issues (HIGH/MEDIUM)

**S1. Assertion guard function naming inconsistency** - `node.ts:169`, `browser.ts:71`
- **Severity**: HIGH | **Reporters**: consistency (92% confidence)
- **Problem**: `node.ts` uses `assertReady()` while `browser.ts` uses `assertInitialized()`. Identical purpose, parallel entry points.
- **Fix**: Standardize on one name (recommend `assertReady`).

**S2. JSDoc phrasing inconsistency for init() requirement** - `node.ts:178-198`, `browser.ts:78-83`
- **Severity**: HIGH | **Reporters**: consistency (90% confidence)
- **Problem**: Different JSDoc phrasings: "Requires await init() first." vs "Requires init() to have been called and awaited first."
- **Fix**: Use consistent wording across both entries.

**S3. Missing shape validation in browser init** - `packages/mds/src/backend/wasm.ts:228-233`
- **Severity**: MEDIUM | **Reporters**: consistency (85% confidence)
- **Problem**: `_initBrowser()` does not validate that the loaded module has `compile`, `check`, `scanImports`. The Node.js path does.
- **Fix**: Add shape validation (same fix as blocking issue #1).

**S4. Test naming misleads on what is tested** - `packages/mds/__test__/wasm-backend.spec.mjs:157-168` (U-WB13)
- **Severity**: MEDIUM | **Reporters**: testing (88% confidence)
- **Problem**: Test titled "rejects modules missing scanImports" only tests the positive path (module *has* scanImports). Never tests rejection.
- **Fix**: Rename to `"initWasmNode() only succeeds when WasmModule includes scanImports"`.

**S5. Test state mutation creates ordering dependency** - `packages/mds/__test__/backend.spec.mjs:57-74` (U-B6)
- **Severity**: MEDIUM | **Reporters**: testing (82% confidence)
- **Problem**: Test calls `_resetForTesting()` mid-test, then `await init()` at end. If test throws before `init()`, subsequent tests fail with confusing errors. Creates ordering dependency if tests are parallelized.
- **Fix**: Use `afterEach` hook to guarantee state restoration, or isolate in subprocess like U-B7+ already do.

---

## Pre-existing Issues (Not Blocking)

- **No symlink rejection test in scanner.spec.mjs** (MEDIUM, testing) — The TOCTOU fix (O_NOFOLLOW) is a security improvement, but the symlink behavior is untested. Recommend adding a symlink-rejection test in a follow-up PR.

---

## Convergence Status

### Cross-Reviewer Agreement (HIGH Confidence)

| Issue | Reporters | Confidence | Category |
|-------|-----------|------------|----------|
| Browser shape validation missing | security, typescript | 90% | Blocking |
| Browser retry exhaustion missing | reliability, architecture, security, typescript | 82-92% | Blocking |
| Aggregate size check ordering | performance, reliability | 85% | Blocking |
| Assertion naming inconsistency | consistency | 92% | Should-Fix |
| JSDoc phrasing inconsistency | consistency | 90% | Should-Fix |
| Unused lstat import | consistency, performance, typescript | 95% | Blocking |

### Divergent Findings

- **wrapWithFileOps bypass**: Only architecture flagged this (85%). Other reviewers did not identify the architectural inconsistency of bypassing base backend normalization. **Resolution**: Trust architecture reviewer's 85% confidence; this is a valid concern that should be addressed.

- **buildModulesMap complexity**: Only complexity flagged (82%). Other reviewers did not identify this as a critical issue. **Resolution**: Complexity reviewer is correct that 169 lines exceeds maintainability thresholds. Should be refactored.

- **browser.getBackend() asymmetry**: Flagged by architecture (82%), consistency (80%), typescript (82%), but not by other reviewers. All three agree this is intentional per test U-BR5 but creates asymmetric developer experience. **Resolution**: Document the design choice with JSDoc rather than enforcing consistency.

---

## Summary by Domain

### Architecture (8/10)
Interface split (MdsBaseBackend / MdsNodeBackend) is well-executed ISP. Sync factory pattern and promise deduplication are clean. **Blocking**: wrapWithFileOps bypass (fix delegation path). **Should-Fix**: document getBackend() asymmetry in JSDoc.

### Security (8/10)
Module-scanner TOCTOU fix (O_NOFOLLOW) is strong security improvement. **Blocking**: browser shape validation missing (matches Node.js validation). Recommend testing the symlink-rejection behavior in follow-up PR.

### Performance (8/10)
TLA removal goal achieved. Promise dedup prevents double-init races. **Blocking**: aggregate size check ordering reversed; files are read before size validation (fix phase split).

### Reliability (8/10)
Bounded iteration (MAX_IMPORT_DEPTH=64, DEFAULT_MAX_MODULES=256, DEFAULT_MAX_AGGREGATE_SIZE=10MiB) is solid. **Blocking**: browser retry exhaustion missing; unbounded retries on persistent failure.

### Complexity (7/10)
God-function `buildModulesMap` refactored from prior state, but PR added 20 lines by replacing 30-line helper with 50-line `openAndValidateModule`. **Blocking**: extract `openNoFollow` to reduce nesting; extract CSP detection from `_initBrowser`.

### Consistency (7/10)
Parallel entry points (node.ts / browser.ts) diverge in naming (`assertReady` vs `assertInitialized`) and JSDoc phrasing. Dead import (`lstat`) contradicts zero-dead-code principle. **Blocking**: fix naming, JSDoc, and dead import.

### Testing (7/10)
95 passing tests across 14 suites. Test suite covers breaking API change and concurrent init. **Blocking**: U-PF0 naming/assertion mismatch; U-WB13 naming mismatch; U-B6 state mutation risk. **Pre-existing**: untested symlink rejection behavior (addressed by TOCTOU fix).

### TypeScript (8/10)
Type hierarchy is correct. **Blocking**: browser shape validation missing; `node.ts` does not re-export `MdsNodeBackend`/`MdsBaseBackend` types; unused `lstat` import. **Should-Fix**: CSP detection overly broad substring match (`'fetch'`).

---

## Action Plan for Author

### Phase 1: Blocking Issues (Required for Merge)
1. Add shape validation to `_initBrowser()` — match `tryLoadCandidate` pattern
2. Add circuit breaker to `initWasmBrowser()` — add `browserFailures` counter with `MAX_BROWSER_RETRIES=3`
3. Fix `wrapWithFileOps` delegation — call `base.compile()` / `base.check()` instead of direct WASM calls
4. Split `openAndValidateModule` — separate validation (open+stat) from reading (readFile), check aggregate size in between
5. Extract helpers from `buildModulesMap` — `openNoFollow()` to reduce nesting; optionally `checkScanLimits()` helper
6. Remove unused `lstat` import and update JSDoc in module-scanner.ts
7. Fix test assertions/naming — U-PF0 (assertion vs name), U-WB13 (test name), U-B6 (afterEach guard)

### Phase 2: Should-Fix Issues (Recommended)
8. Standardize assertion guard naming (`assertReady` vs `assertInitialized`)
9. Standardize JSDoc phrasing for init() requirement
10. Add type re-exports to `node.ts` (`MdsNodeBackend`, `MdsBaseBackend`)
11. Tighten CSP detection in `_initBrowser` (avoid broad `'fetch'` match)
12. Document `browser.ts:getBackend()` intentional asymmetry in JSDoc

### Phase 3: Follow-up (Post-Merge)
- Add symlink-rejection test to scanner.spec.mjs (tests the TOCTOU fix)
- Add test coverage for `initWasmBrowser()` error paths (CSP detection, missing `default()`)
- Extract `ScanContext` interface for `buildModulesMap` closures (longer-term refactor)

---

## Scores

| Domain | Score | Risk |
|--------|-------|------|
| Architecture | 8/10 | LOW |
| Security | 8/10 | LOW |
| Performance | 8/10 | MEDIUM |
| Reliability | 8/10 | MEDIUM |
| Complexity | 7/10 | MEDIUM |
| Consistency | 7/10 | LOW |
| Testing | 7/10 | MEDIUM |
| TypeScript | 8/10 | LOW |
| **Overall** | **7.6/10** | **MEDIUM** |

---

## Notes

- **Zero users**: Pre-release project (MEMORY.md) allows breaking changes without migration burden.
- **Well-structured refactor**: Decomposition of god-function is successful; removal of TLA is clean.
- **Deliberate trade-offs**: TOCTOU fix justifies added complexity in `openAndValidateModule`; union-type backend selection is intentional (native preferred, WASM fallback).
- **95 tests passing**: Comprehensive coverage of breaking changes and new interface split.

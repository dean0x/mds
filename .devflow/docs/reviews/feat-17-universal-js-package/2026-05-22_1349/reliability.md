# Reliability Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Module count check is off-by-one — allows maxModules+1 modules** - `packages/mds/src/util/module-scanner.ts:132-138`
**Confidence**: 90%
- Problem: The module count limit check (`Object.keys(modules).length >= maxModules`) runs AFTER `visited.add()` and AFTER reading the file content, but BEFORE adding to `modules`. However, the entry itself is added to `modules` at line 138 unconditionally — meaning the check allows `maxModules` entries and then adds one more before the next iteration's check fires. With parallel children, multiple children can pass the check simultaneously before any of them add to `modules`, potentially exceeding the limit by up to N (where N is the number of parallel imports in a single file).
- Fix: Move the modules count check before reading file content, or use `visited.size` (which is incremented first) as the authoritative count:
```typescript
if (visited.size > maxModules) {
  throw new Error(
    `resource limit: module count exceeds maximum of ${maxModules}`,
  );
}
```

**Aggregate size check races under parallel scan — concurrent reads can overshoot limit** - `packages/mds/src/util/module-scanner.ts:125-130,144`
**Confidence**: 85%
- Problem: `aggregateSize` is a mutable closure variable incremented after `readFile` completes. Because `Promise.all` parallelizes child reads at each level (line 144), multiple concurrent `scan()` calls can each pass the aggregate size check before any of them have incremented `aggregateSize`. Example: 4 children each read 3 MiB concurrently. Each checks `aggregateSize` (still low), passes, reads, then they all increment — blowing past the 10 MiB limit to 12 MiB before any check fires.
- Fix: Either serialize child scans (replace `Promise.all` with sequential loop), or use a pre-check on `stats.size` before reading:
```typescript
const stats = await lstat(absolutePath);
if (stats.isSymbolicLink()) { /* ... */ }

// Pre-check size BEFORE reading content
aggregateSize += stats.size;
if (aggregateSize > maxAggregateSize) {
  throw new Error(
    `resource limit: aggregate module size exceeds maximum of ${maxAggregateSize} bytes`,
  );
}

const content = await readFile(absolutePath, 'utf-8');
```
This uses `stats.size` (available from the existing `lstat` call) as a reservation before the actual read occurs.

### MEDIUM

**WASM init() singleton has no retry bound — unbounded retry on transient failures** - `packages/mds/src/backend/wasm.ts:40-50`
**Confidence**: 82%
- Problem: When `init()` fails, `initPromise` is reset to `null`, allowing unlimited retries. If a caller has a retry loop around init (common in browser environments with network-loaded WASM), there is no maximum retry count imposed by the module itself. Each failure resets state completely, enabling unbounded retry attempts.
- Fix: Add a failure counter with a maximum retry limit:
```typescript
const MAX_INIT_RETRIES = 3;
let initFailures = 0;

export async function init(options?: InitOptions): Promise<void> {
  if (initPromise !== null) {
    return initPromise;
  }
  if (initFailures >= MAX_INIT_RETRIES) {
    throw new Error('@mds/mds: WASM init failed permanently after max retries');
  }
  initPromise = _init(options).catch((err) => {
    initFailures++;
    initPromise = null;
    throw err;
  });
  return initPromise;
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**napi loadBinding reads /usr/bin/ldd synchronously on every module require** - `crates/mds-napi/index.js:7`
**Confidence**: 80%
- Problem: `isMusl()` reads `/usr/bin/ldd` from the filesystem synchronously during module load. This is a blocking I/O call on the main thread. While it only runs once (at require time), if the file is large or the filesystem is slow (NFS, containers), this can stall the Node.js event loop during initialization.
- Fix: This is an acceptable tradeoff for a one-shot module initialization, but consider catching more specifically or adding a size limit to the read:
```javascript
function isMusl() {
  const { readFileSync } = require('fs');
  try { 
    const content = readFileSync('/usr/bin/ldd', 'utf-8');
    return content.slice(0, 4096).includes('musl'); // Only check first 4K
  }
  catch { return false; }
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Unbounded `candidates` loop in _init without explicit bound** - `packages/mds/src/backend/wasm.ts:68` (Confidence: 65%) — The candidates array is currently hardcoded to 2 entries, so the loop terminates. If this array grows (e.g., from configuration), consider an explicit length assertion.

- **browser.ts compileFile/checkFile create unhandled rejection risk** - `packages/mds/src/browser.ts:72-87` (Confidence: 70%) — These functions return `Promise.reject(...)` immediately, which can trigger unhandled rejection warnings if the caller does not `.catch()` the result synchronously. Consider throwing synchronously instead, or documenting the async-only nature.

- **node.ts top-level await blocks module loading** - `packages/mds/src/node.ts:14-39` (Confidence: 60%) — Top-level await in the module means all importers block until backend initialization completes. This is intentional (backend must be ready), but if either native load or WASM fallback is slow, all dependent modules stall. No immediate fix needed given the design constraints.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The module-scanner has solid architectural intentions (visit tracking, size limits, module count limits, symlink rejection) but the parallel execution model undermines the resource limit enforcement. The off-by-one in module count and the race in aggregate size checking mean the limits are advisory rather than strict bounds. Under adversarial input (deeply nested imports with many parallel branches), the actual resource consumption can exceed configured maximums. The WASM singleton pattern is well-designed with proper error recovery, though adding a retry cap would make it fully bounded.

# Performance Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Dynamic import on every `compileFile`/`checkFile` call** - `packages/mds/src/backend/wasm.ts:103`
**Confidence**: 90%
- Problem: `buildFileModules` uses `await import('../util/module-scanner.js')` on every call to `compileFile` or `checkFile`. While modern bundlers and Node.js cache dynamic imports after the first load, this adds unnecessary overhead to the hot path (module resolution lookup + cache check on every file operation). More importantly, it prevents the engine from optimizing this code path since the import is treated as a potentially side-effecting dynamic expression each time.
- Fix: Hoist the import to module-level or cache the resolved module reference:
```typescript
// Option A: top-level import (preferred — static analysis, tree shaking)
import { buildModulesMap } from '../util/module-scanner.js';

async function buildFileModules(wasm: WasmModule, path: string) {
  return buildModulesMap(
    path,
    (source) => wasm.scanImports(source),
    { maxModules: WASM_MAX_MODULES, maxAggregateSize: WASM_MAX_AGGREGATE_SIZE },
  );
}
```
Note: If the dynamic import is intentional for code-splitting (browser bundle), consider a lazy-cached pattern instead.

---

**`Object.keys(modules).length` on every iteration for module count check** - `packages/mds/src/util/module-scanner.ts:132`
**Confidence**: 85%
- Problem: `Object.keys(modules).length` creates a new array of all keys on every call to `scan()`, purely to get the count. With up to 256 modules, this is O(n) allocation on each file processed (up to O(n^2) total allocations across the entire scan). The `visited` Set already tracks the count accurately.
- Fix: Use `visited.size` which is O(1):
```typescript
if (visited.size >= maxModules) {
  throw new Error(
    `resource limit: module count exceeds maximum of ${maxModules}`,
  );
}
```
Alternatively, maintain a simple counter variable alongside `modules`.

### MEDIUM

**Redundant `lstat` + `readFile` sequential I/O per module** - `packages/mds/src/util/module-scanner.ts:111-123`
**Confidence**: 82%
- Problem: Each module requires two sequential filesystem calls: `lstat` (to reject symlinks) then `readFile`. For deep import trees hitting 256 modules, this is 512 sequential I/O operations per level (though child reads are parallelized). Since `lstat` returns the file stats but the content still requires a separate `readFile`, there is no way to combine them in Node.js, but the sequential nature within a single file is a minor overhead.
- Fix: This is a reasonable security tradeoff. For marginal improvement, you could use `fs.open` with `O_NOFOLLOW` flag on Linux to reject symlinks and read in a single open, but that is platform-specific. Acceptable as-is given the security requirement, but worth noting for future optimization if module-scanner becomes a bottleneck.

---

**`content.length` measures UTF-16 code units, not bytes** - `packages/mds/src/util/module-scanner.ts:125`
**Confidence**: 80%
- Problem: `content.length` on a string in JavaScript returns UTF-16 code unit count, not byte count. The limit is documented as "10 MiB" (bytes), but the check uses character count. For ASCII-heavy MDS files this is fine, but for files with multi-byte characters (emoji, CJK), the byte size could be up to 3x the character count for 3-byte UTF-8 sequences that are a single UTF-16 code unit. This means the effective limit is inconsistent depending on content — not a strict performance issue, but an inaccurate resource limit that could allow more memory usage than intended.
- Fix: Use `Buffer.byteLength(content, 'utf-8')` for accurate byte counting, or document that the limit is character-based, not byte-based:
```typescript
aggregateSize += Buffer.byteLength(content, 'utf-8');
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`readFileSync` in napi binding loader on Linux** - `crates/mds-napi/index.js:7`
**Confidence**: 82%
- Problem: `isMusl()` calls `readFileSync('/usr/bin/ldd', 'utf-8')` synchronously at module load time. This is a blocking I/O operation during `require()`. While this only runs once (at module load) and only on Linux, it reads an entire binary file into memory as a UTF-8 string just to check for "musl". The file `/usr/bin/ldd` can be several KB; reading the entire binary as text is wasteful.
- Fix: Since this runs once at module load and is in CommonJS require() context (inherently synchronous), the sync nature is acceptable. However, reading the entire binary is unnecessary — read only the first few hundred bytes:
```javascript
function isMusl() {
  const { openSync, readSync, closeSync } = require('fs');
  try {
    const fd = openSync('/usr/bin/ldd', 'r');
    const buf = Buffer.alloc(512);
    readSync(fd, buf, 0, 512, 0);
    closeSync(fd);
    return buf.toString('utf-8').includes('musl');
  } catch { return false; }
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Scan parallelism is breadth-first but depth-sequential** - `packages/mds/src/util/module-scanner.ts:144` (Confidence: 65%) — Children at each level are parallelized with `Promise.all`, but deeper descendants must wait for their parent's scan to complete. For deep import trees (A->B->C->D), this creates a waterfall. A worklist-based approach could achieve higher parallelism, but given the 256-module cap and typical shallow trees, this is unlikely to be a real bottleneck.

- **WASM `compile` allocates empty `modules: {}` object on every call** - `packages/mds/src/backend/wasm.ts:119` (Confidence: 62%) — When calling `wasm.compile(source, { filename: 'input.mds', modules: {}, ...varsOpt(options) })`, a new empty object is allocated per call. A frozen constant `EMPTY_MODULES = Object.freeze({})` could avoid repeated allocation, though V8 likely optimizes this already.

- **`IndexSet` in Rust `scan_imports` is heavier than needed for small sets** - `crates/mds-core/src/lib.rs:750` (Confidence: 60%) — For the typical case of 1-5 imports, a `Vec` with linear dedup would be faster than `IndexSet` (avoids hash overhead). But `IndexSet` is correct for the general case and the cost is negligible for typical input sizes.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The architecture is sound for performance: singleton WASM init with cached promise prevents redundant initialization, resource limits (256 modules, 10 MiB) bound computation, and the native backend path is zero-overhead (thin wrapper over napi calls). The two HIGH issues are easy fixes that improve hot-path efficiency without architectural changes. The module-scanner parallelizes child reads appropriately and the visited-set prevents re-scanning. Overall, this is a performance-aware implementation with minor inefficiencies to clean up.

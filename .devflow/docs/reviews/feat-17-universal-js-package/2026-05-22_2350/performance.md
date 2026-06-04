# Performance Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Sequential filesystem syscalls per module in scan() -- 3 serial awaits where 2 could be parallel** - `packages/mds/src/util/module-scanner.ts:151-163`
**Confidence**: 85%
- Problem: Each module file triggers three sequential async syscalls: `lstat()`, `realpath()`, and then `readFile()`. The `lstat` and `realpath` calls are both metadata operations that could run concurrently via `Promise.all` before `readFile`. For projects with many imports (up to 256 modules), this serializes 2 unnecessary round-trips per file. At typical NVMe SSD latencies (~100us per syscall), 256 modules pays ~51ms in avoidable sequential I/O.
- Fix: Parallelize the two metadata calls, then read conditionally:
```typescript
const [stats, resolved] = await Promise.all([
  lstat(absolutePath),
  realpath(absolutePath),
]);
if (stats.isSymbolicLink()) {
  throw new Error(`security: symlink detected at ${absolutePath} -- symlinks are not allowed`);
}
if (resolved !== absolutePath) {
  throw new Error(
    `security: path ${absolutePath} resolved to unexpected location ${resolved} -- possible symlink swap`,
  );
}
// ... then readFile
```

### MEDIUM

**Object spread on every compile/check call creates unnecessary allocation** - `packages/mds/src/backend/wasm.ts:120-121`
**Confidence**: 80%
- Problem: Every `compile()` and `check()` call spreads `varsOpt(options)` into a new object literal: `{ filename: 'input.mds', modules: {}, ...varsOpt(options) }`. This creates a fresh `modules: {}` object and a spread on every single call, even for the hot path of simple string compilation where no modules or vars are involved. For high-throughput use cases (e.g., batch compilation), this adds GC pressure.
- Fix: Pre-allocate a frozen default options object and only create a new one when vars are provided:
```typescript
const DEFAULT_COMPILE_OPTS = Object.freeze({ filename: 'input.mds', modules: {} });

compile(source: string, options?: CompileOptions): CompileResult {
  const wasm = assertInitialized();
  const opts = options?.vars !== undefined
    ? { filename: 'input.mds', modules: {}, vars: options.vars }
    : DEFAULT_COMPILE_OPTS;
  return wasm.compile(source, opts);
},
```

**Redundant path-within-project-root check after realpath already validates** - `packages/mds/src/util/module-scanner.ts:165-170`
**Confidence**: 82%
- Problem: After the TOCTOU `realpath` check at line 158-163 confirms `resolved === absolutePath`, the code re-checks `absolutePath.startsWith(projectRoot + '/')` at line 166. This is the same check already performed in `validateImportPath()` at line 126 before `scan()` is called. For the entry file, `absoluteEntry` is derived from `resolve(entryPath)` which is guaranteed to be within `projectRoot` (since `projectRoot = dirname(absoluteEntry)`). This redundant string comparison runs on every module.
- Fix: Remove the duplicate check at lines 165-170. The project-root containment is already enforced in `validateImportPath()` for child imports, and is tautologically true for the entry file.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Top-level await in node.ts blocks module loading** - `packages/mds/src/node.ts:19-44`
**Confidence**: 85%
- Problem: The module uses top-level `await` to load and initialize the backend at import time. When a consumer does `import { compile } from '@mds/mds'`, the entire backend resolution (trying native, falling back to WASM) runs before the import resolves. This delays application startup and blocks the event loop during module graph evaluation. If the native addon loads successfully (the common case), this is fast, but the fallback path with WASM init can be significantly slower. The sequential `try/catch` chain means WASM fallback only starts after native fails -- these could overlap.
- Fix: Consider lazy initialization on first use (similar to the browser.ts pattern), or at minimum document the startup cost. If eager init is intentional for the Node.js entry point, this is acceptable but should be noted in the README's performance section.

**Dynamic import of module-scanner inside buildFileModules** - `packages/mds/src/backend/wasm.ts:108-110`
**Confidence**: 80%
- Problem: The previous code used a dynamic `import()` for `module-scanner.js` inside `buildFileModules`. The refactored version correctly moves it to a static import at the top of the file (line 11-13). This is a performance improvement -- good change. However, `buildFileModules` is now a thin wrapper that only forwards arguments. Consider inlining to remove one layer of function call overhead.
- Fix: This is minor -- the function provides naming clarity. No action required unless profiling shows it as a hot path.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Record<string, string> for modules map grows linearly with no size hint** - `packages/mds/src/util/module-scanner.ts:104`
**Confidence**: 80%
- Problem: The `modules` object is a plain `Record<string, string>` that grows as files are discovered. V8 transitions object storage from fast properties to dictionary mode after ~27 properties. For projects with many modules (up to 256), this transition causes a one-time performance cliff. A `Map<string, string>` would maintain O(1) access without this transition, but changing the type would affect the WASM FFI boundary.
- Fix: Not actionable without changing the WASM interface. Informational only.

## Suggestions (Lower Confidence)

- **Eager backend selection could be deferred** - `packages/mds/src/node.ts:19` (Confidence: 65%) -- Top-level await eagerly loads the backend. A lazy pattern (resolve on first compile/check call) would reduce import-time cost for consumers who import the module but don't use it immediately.

- **String concatenation in error messages within hot path** - `packages/mds/src/util/module-scanner.ts:128-129` (Confidence: 60%) -- Template literals and string concatenation in error-path throw statements are only executed on failure, so this is not a real concern in practice.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates good performance awareness overall: the `visited` Set prevents redundant work, `Promise.all` parallelizes sibling imports, aggregate size is pre-checked against `stats.size` before reading (avoiding the read-then-check pattern), and the `varsOpt` utility avoids unnecessary object creation when no vars are provided. The main actionable finding is the sequential metadata syscalls in the scanner hot loop, which could be parallelized for measurable improvement on large module trees. The object allocation on every compile call is a secondary concern for high-throughput scenarios.

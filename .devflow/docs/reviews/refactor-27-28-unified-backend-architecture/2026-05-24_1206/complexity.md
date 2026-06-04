# Complexity Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-24
**Commits**: dc9589f, c77bde4, fd7ef89, c57685c

## Issues in Your Changes (BLOCKING)

### HIGH

**`openAndValidateModule` at 50 lines touches the complexity warning threshold** - `module-scanner.ts:150`
**Confidence**: 85%
- Problem: This newly written function is exactly 50 lines (the boundary between WARNING and CRITICAL in the complexity metrics table). It handles O_NOFOLLOW open, ELOOP/ENOTDIR mapping, fstat, realpath comparison, readFile, and handle cleanup via try/finally. The nesting depth reaches 4 levels (function > try > catch > if). While each concern is legitimate, the function does three distinct things: (1) open with symlink protection, (2) validate the opened fd, and (3) read content. Combining all three makes it harder to test individual security checks in isolation.
- Fix: Extract the open-with-symlink-protection into a helper that returns the handle, leaving `openAndValidateModule` responsible only for validation and reading:
```typescript
async function openNoFollow(absolutePath: string): Promise<FileHandle> {
  try {
    return await open(absolutePath, constants.O_RDONLY | O_NOFOLLOW);
  } catch (err) {
    const code = (err as NodeJS.ErrnoException).code;
    if (code === 'ELOOP' || code === 'ENOTDIR') {
      throw new Error(`security: symlink detected at ${absolutePath} — symlinks are not allowed`);
    }
    throw err;
  }
}
```
This would bring `openAndValidateModule` down to ~35 lines and isolate the platform-specific symlink logic.

**`buildModulesMap` outer function spans 169 lines with 3 nested function definitions** - `module-scanner.ts:91`
**Confidence**: 82%
- Problem: The outer `buildModulesMap` function body runs from line 91 to 259 (169 lines). While the actual orchestration logic (lines 91-111, 256-259) is only ~25 lines, the function contains three nested closures (`validateImportPath`, `openAndValidateModule`, `scan`) that capture `projectRoot`, `modules`, `visited`, and `aggregateSize` via closure. This is a pre-existing pattern, but this PR added `openAndValidateModule` (replacing the former `statAndValidateModule` at 30 lines) at 50 lines, growing the enclosing function body by 20 lines. The nested closures closing over shared mutable state (`aggregateSize`, `modules`, `visited`) make it harder to reason about the function as a unit. A reader must hold the entire 169-line scope in mind to understand what state each closure mutates.
- Fix: Consider extracting the closures into module-level functions that accept the shared state as an explicit parameter object:
```typescript
interface ScanContext {
  projectRoot: string;
  modules: Record<string, string>;
  visited: Set<string>;
  aggregateSize: number;
  maxModules: number;
  maxAggregateSize: number;
}
```
This would make state flow explicit rather than implicit via closure. However, this is partially a matter of style -- the closure approach is valid and the individual functions are each well-scoped internally. Severity is HIGH because the 169-line span exceeds the 50-line WARNING threshold significantly.

### MEDIUM

**`_initBrowser` at 46 lines approaches the warning threshold** - `wasm.ts:223`
**Confidence**: 80%
- Problem: This new function handles dynamic import, shape validation, default() initialization, and CSP error detection. At 46 lines it is within the 30-50 WARNING range. The nesting reaches 4 levels in the CSP detection path (function > try > catch > if). The CSP string matching block (lines 253-265) is a distinct concern from WASM module loading.
- Fix: Extract the CSP detection into a helper:
```typescript
function isCspError(msg: string): boolean {
  return msg.includes('Content Security Policy')
    || msg.includes('unsafe-eval')
    || msg.includes('wasm-unsafe-eval')
    || msg.includes('fetch');
}
```
This would reduce `_initBrowser` to ~35 lines and make the CSP detection testable independently.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`normalizeVirtualKey` at 46 lines is within the warning range** - `module-scanner.ts:34`
**Confidence**: 80%
- Problem: This function was not changed in this PR but sits in the same file that was modified. At 46 lines with cyclomatic complexity ~7 (multiple early returns, a for-loop with 3-way branching), it is in the WARNING zone. The logic is straightforward path normalization, but the segment counting and edge cases (empty base, .., .) make it moderately dense.
- Fix: No immediate action needed -- the function is clear and well-commented. Consider extracting the segment resolution loop into a `resolveSegments(baseSegments, relativeParts)` helper if this function grows further.

**`scan` nested function at 54 lines exceeds the 50-line warning threshold** - `module-scanner.ts:201`
**Confidence**: 82%
- Problem: The `scan` function is 54 lines and handles depth checking, visited tracking, module count limits, file validation, aggregate size tracking, content storage, import scanning, and recursive child processing. This is within the CRITICAL range (>50 lines). However, the function reads linearly and each block is clearly separated by comments. The nesting depth reaches 4 levels (function > if > Promise.all > async map callback).
- Fix: The depth/visited/module-count guard block (lines 201-222) could be extracted into a `checkScanLimits` helper, reducing `scan` to ~35 lines:
```typescript
function checkScanLimits(absolutePath: string, depth: number): 'skip' | 'proceed' {
  if (depth > MAX_IMPORT_DEPTH) throw new Error(...);
  if (visited.has(absolutePath)) return 'skip';
  visited.add(absolutePath);
  if (visited.size > maxModules) throw new Error(...);
  return 'proceed';
}
```

## Pre-existing Issues (Not Blocking)

No critical pre-existing complexity issues found in unchanged code.

## Suggestions (Lower Confidence)

- **Parallel init promise patterns are duplicated across 3 files** - `node.ts:159`, `browser.ts:57`, `wasm.ts:131` (Confidence: 70%) -- The "if cached return cached; set cached = work().catch(clear); return cached" pattern appears three times with minor variations. A shared `onceLazy(factory)` utility could eliminate this duplication, but the variations (node.ts resets `initPromise` on error; wasm.ts increments `nodeFailures`) make a generic abstraction non-trivial.

- **`_initBrowser` CSP string matching is fragile** - `wasm.ts:253-257` (Confidence: 65%) -- Matching error messages by substring (`'fetch'`, `'unsafe-eval'`) is fragile since unrelated errors containing these words would be misclassified. This is more of a reliability concern than complexity, but the branching adds to the function's cognitive load.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

This PR represents a significant complexity **improvement** over the prior state. The god-function in `node.ts` has been successfully decomposed into `loadNativeBackend()` (12 lines), `loadWasmNodeBackend()` (5 lines), `ensureBackend()` (31 lines), and `wrapWithFileOps()` (18 lines) -- all well under the 50-line threshold and at most 3 levels of nesting. The `ensureBackend` orchestrator uses early returns effectively to keep cyclomatic complexity low (~4).

The WASM split into `initWasmNode` / `initWasmBrowser` with a sync `createWasmBackend` factory is clean and each function is reasonably sized.

The primary remaining complexity concentration is in `module-scanner.ts`, where `buildModulesMap` spans 169 lines as a container for 3 nested closures sharing mutable state. While the PR made this slightly larger by replacing the 30-line `statAndValidateModule` with the 50-line `openAndValidateModule`, the TOCTOU fix is a legitimate security improvement that justifies the added complexity.

**Conditions for approval:**
1. Consider extracting `openNoFollow` from `openAndValidateModule` to bring it under 50 lines (the function is right at the threshold)
2. Consider extracting the CSP detection helper from `_initBrowser` to reduce nesting depth

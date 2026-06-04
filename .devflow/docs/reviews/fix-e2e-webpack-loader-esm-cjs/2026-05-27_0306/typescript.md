# TypeScript Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**`findProjectRoot` uses synchronous `existsSync` in an otherwise async codebase** - `packages/mds/src/util/module-scanner.ts:29`
**Confidence**: 82%
- Problem: `findProjectRoot` calls `existsSync` (blocking I/O) inside a tight loop that traverses up to 256 parent directories. The rest of `module-scanner.ts` is carefully async (`open`, `realpath`, `readFile`). On deep directory trees or slow/network-mounted filesystems, up to 512 blocking `existsSync` calls (256 iterations x 2 markers) could stall the Node.js event loop. The function is called once per `buildModulesMap` invocation, which is once per Webpack loader file transformation.
- Fix: Convert to async using `access` from `node:fs/promises`:
  ```typescript
  import { access } from 'node:fs/promises';
  
  export async function findProjectRoot(start: string): Promise<string> {
    let dir = start;
    for (let i = 0; i < MAX_TRAVERSAL_DEPTH; i++) {
      for (const marker of PROJECT_ROOT_MARKERS) {
        try {
          await access(resolve(dir, marker));
          return dir;
        } catch {
          // marker not found, continue
        }
      }
      const parent = dirname(dir);
      if (parent === dir) {
        return start;
      }
      dir = parent;
    }
    return start;
  }
  ```
  This would require updating the call site at line 155 to `await findProjectRoot(...)` and the return type of the function. Since `buildModulesMap` is already async, the change is minimal.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Runtime shape check could use narrower type assertion** - `packages/webpack-loader/src/index.ts:41` (Confidence: 65%) -- The `(mds as Record<string, unknown>)['compileFile']` check verifies one function but trusts the rest of the `typeof import('@mds/mds')` assertion on line 40. If the module shape diverges further (e.g., `init` is missing), the error would surface later with a less helpful message. Consider validating both `compileFile` and `init` since those are the two methods the `MdsApi` interface requires.

- **`relative()` can produce leading `..` segments** - `packages/mds/src/util/module-scanner.ts:156` (Confidence: 62%) -- If `findProjectRoot` returns a directory that is not an ancestor of the entry file (theoretically possible if `start` is returned as fallback and the entry is at a different location), `relative(projectRoot, absoluteEntry)` could produce a path starting with `../`. The `startsWith(projectRoot + '/')` guard at line 184 would still catch traversal, but the `entryFilename` used as the virtual key would be unusual. The current fallback behavior (returning `start`) makes this unlikely in practice.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The TypeScript changes are well-typed with proper strict mode (`strict: true`, `noUncheckedIndexedAccess: true`). The `_esmImport` workaround is correctly typed with explicit signatures and the `as typeof import(...)` assertion is appropriate for dynamic import from a `new Function` call. The `findProjectRoot` function is cleanly bounded (applies ADR-001 merge gate quality). The one condition is the synchronous I/O concern in `findProjectRoot` -- consider converting to async before merge if network filesystem use is a supported scenario.

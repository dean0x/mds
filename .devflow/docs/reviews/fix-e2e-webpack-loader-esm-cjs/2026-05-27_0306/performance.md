# Performance Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### HIGH

**Synchronous filesystem calls in `findProjectRoot` block the event loop** - `packages/mds/src/util/module-scanner.ts:28`
**Confidence**: 85%
- Problem: `findProjectRoot` uses `existsSync` inside a loop that traverses up to 256 parent directories, checking 2 markers at each level. This is called at the start of `buildModulesMap`, which is an async function. The synchronous `existsSync` calls block the Node.js event loop during the entire traversal. On deep directory trees (e.g., `/home/user/a/b/c/d/e/.../project`), this can stall the event loop for multiple milliseconds per `buildModulesMap` invocation. In a Webpack build with hundreds of `.mds` files, the loader calls `buildModulesMap` for each file, and the synchronous traversal runs every time.
- Fix: Either (a) cache the discovered project root after first resolution (it will not change within a build), or (b) convert to async `fs.promises.access` calls. Caching is simpler and more effective since the project root is invariant within a process:

```typescript
// Module-level cache — project root is invariant within a build.
const projectRootCache = new Map<string, string>();

export function findProjectRoot(start: string): string {
  const cached = projectRootCache.get(start);
  if (cached !== undefined) return cached;

  let dir = start;
  for (let i = 0; i < MAX_TRAVERSAL_DEPTH; i++) {
    for (const marker of PROJECT_ROOT_MARKERS) {
      if (existsSync(resolve(dir, marker))) {
        projectRootCache.set(start, dir);
        return dir;
      }
    }
    const parent = dirname(dir);
    if (parent === dir) {
      projectRootCache.set(start, start);
      return start;
    }
    dir = parent;
  }
  projectRootCache.set(start, start);
  return start;
}
```

Note: Even with caching, each unique `start` directory still pays the synchronous cost once. For full async, use `fs.promises.access` with `try/catch`.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **CJS test files call `require()` repeatedly for same module** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs:19-62` (Confidence: 65%) -- Each test function calls `require(resolve(__dirname, '../dist-cjs/index.js'))` independently. While Node.js caches `require()` results, the repeated `resolve()` computation and require-cache lookup across 7 tests is unnecessary. A single top-level require with destructuring would be cleaner and marginally faster.

- **Build script shell parallelization uses `&` and `wait` which is shell-specific** - `packages/bundler-utils/package.json:27` (Confidence: 60%) -- The `tsc ... & tsc ... & wait` pattern is a good performance improvement for parallel builds (the prior cycle noted this fix). However, this relies on POSIX shell job control which does not work on Windows `cmd.exe`. If Windows CI is ever needed, this would silently serialize. Not a performance regression -- just a portability note.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The Rust-side changes (parser, evaluator, validator, AST) are well-structured with no performance concerns: the new `evaluate_condition` and `values_equal` functions are O(1) match-based dispatches, the `@elseif` evaluation properly short-circuits on first match, `find_unquoted_operator` is an efficient single-pass byte scanner, and the `parse_condition`/`parse_dot_path`/`parse_cond_value` functions all operate on small bounded strings with no allocations beyond the output. The reduction of `MAX_NESTING_DEPTH` from 256 to 64 is a positive performance/safety improvement that reduces worst-case stack frame usage. The `prefix_terminators` check in `parse_body` adds a negligible `starts_with` call per directive token.

The one actionable finding is the synchronous `findProjectRoot` in the TypeScript module scanner, which performs blocking filesystem I/O inside an async code path. This is most impactful in Webpack builds where the loader processes many files. Adding a module-level cache would eliminate the repeated traversal cost.

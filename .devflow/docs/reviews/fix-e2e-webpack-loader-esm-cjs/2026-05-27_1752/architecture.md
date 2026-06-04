# Architecture Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**PR**: #34

## Issues in Your Changes (BLOCKING)

### HIGH

**Module-level mutable singleton (`projectRootCache`) lacks reset/invalidation** - `packages/mds/src/util/module-scanner.ts:25`
**Confidence**: 85%
- Problem: `projectRootCache` is a module-level `Map<string, string>` with no public API to clear it. In long-lived processes (Webpack watch mode, dev servers), the cache is never invalidated. If the project root changes (e.g., a `.mdsroot` file is added/removed during development), the stale cache entry will persist for the lifetime of the process, silently using the wrong project root for path resolution and security confinement.
- Impact: Stale cache entries could cause the security boundary (project root confinement) to use an incorrect root, potentially allowing or denying imports incorrectly. This also makes testing fragile -- the test `U-PR5` validates caching but cache entries from other tests could leak.
- Fix: Add an exported `_clearProjectRootCacheForTesting()` function gated on `NODE_ENV=test` (consistent with the webpack-loader pattern), and document in the JSDoc that the cache assumes an invariant project root within a build. Consider adding a `clearProjectRootCache()` export for use in watch-mode scenarios:
```typescript
/**
 * Clear the project root cache. Call when the project structure may have
 * changed (e.g., between watch-mode rebuilds).
 */
export function clearProjectRootCache(): void {
  projectRootCache.clear();
}
```

---

**Duplicated `findProjectRoot` implementation between scanner module and its refactored form** - `packages/mds/src/util/module-scanner.ts:37-63`
**Confidence**: 82%
- Problem: The diff shows `findProjectRoot` was extracted into the main body of `module-scanner.ts` but the original implementation was also inlined (pre-refactoring it used `dirname(absoluteEntry)` as the project root). The new implementation duplicates the traversal logic inline within `findProjectRoot` rather than delegating to `_findProjectRootUncached`, contrary to the single-responsibility principle. Looking at the current file, the cache-set calls are duplicated in three places within the function body (line 47, 53, 58) instead of using the extract-and-cache pattern that the function's JSDoc describes.
- Fix: Extract the uncached logic to a private helper (as shown in the read file -- the current code already does this correctly with `_findProjectRootUncached` in the final state). The diff shows an intermediate state that was refactored. **Upon re-reading the final file state, this is already resolved** -- the function delegates to `_findProjectRootUncached` and caches. This finding is withdrawn.

---

### MEDIUM

**Webpack loader singleton captures options from first invocation only** - `packages/webpack-loader/src/index.ts:42-58`
**Confidence**: 85%
- Problem: The `getLazy` function creates a `LazyInit<Transformer>` singleton on first call. The comment on line 38-41 documents this limitation ("options are captured from the first call"), but the code silently ignores different options passed in subsequent calls. If a consumer mistakenly uses different options across loader invocations (or in a multi-compiler setup), the second set of options is silently dropped with no warning.
- Impact: Silent misconfiguration. The comment acknowledges this but the code provides no runtime defense.
- Fix: Add a warning or assertion when `lazy !== null` and the incoming options differ from the originally captured ones:
```typescript
function getLazy(options: MdsPluginOptions): LazyInit<Transformer> {
  if (lazy === null) {
    capturedOptions = options;
    lazy = new LazyInit(async () => { /* ... */ });
  } else if (JSON.stringify(options) !== JSON.stringify(capturedOptions)) {
    // Warn about ignored options — fail fast instead of silent misconfiguration
    console.warn('[mds-webpack-loader] Options differ from first invocation; using originally captured options.');
  }
  return lazy;
}
```

---

**`_esmImport` via `new Function` is an architectural workaround that bypasses module resolution** - `packages/webpack-loader/src/index.ts:17-20`
**Confidence**: 82%
- Problem: The `new Function('id', 'return import(id)')` pattern evades TypeScript's CJS-to-require rewriting. While thoroughly documented with the CSP caveat and the upstream TypeScript issue link, this is an architectural workaround that creates a runtime eval-equivalent. It prevents static analysis tools from seeing the `@mds/mds` dependency and means bundlers/tree-shakers cannot trace this import.
- Impact: Static analysis blind spot. The import of `@mds/mds` is invisible to TypeScript's type checker at the call site (mitigated by the `as typeof import(...)` cast) and to any tooling that traces `require`/`import` calls.
- Fix: This is a known TypeScript limitation (linked issue #43329). The current approach is the established community workaround. No code change needed, but consider adding a `// @ts-expect-error` comment or a lint-disable narrowed to this specific line rather than the broader `@typescript-eslint/no-implied-eval` rule disable. Lower priority.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`MAX_ELSEIF_BRANCHES` constant defined in `ast.rs` rather than `limits.rs`** - `crates/mds-core/src/ast.rs:8-11`
**Confidence**: 85%
- Problem: The codebase has a dedicated `limits.rs` module that houses `MAX_DOT_SEGMENTS`. The new `MAX_ELSEIF_BRANCHES` constant is defined in `ast.rs` instead, breaking the single-source-of-truth pattern for resource limits. Similarly, `MAX_NESTING_DEPTH` is defined in `parser.rs:17`. The evaluator has its own limits (`MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`) defined locally. This scattering means an auditor reviewing resource limits must search across 4 files.
- Impact: Inconsistent limit placement makes it harder to audit all resource bounds in one place. Not a functional issue, but an organizational concern.
- Fix: Consider consolidating all `MAX_*` constants into `limits.rs` and re-exporting them. This is a refactoring opportunity, not a blocker:
```rust
// limits.rs
pub(crate) const MAX_DOT_SEGMENTS: usize = 32;
pub(crate) const MAX_NESTING_DEPTH: usize = 64;
pub(crate) const MAX_ELSEIF_BRANCHES: usize = 256;
pub(crate) const MAX_CALL_DEPTH: usize = 128;
// ... etc
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Module-level `projectRootCache` uses synchronous `existsSync` for I/O in an async context** - `packages/mds/src/util/module-scanner.ts:50-52`
**Confidence**: 82%
- Problem: `findProjectRoot` is a synchronous function that calls `existsSync` in a loop up to `MAX_TRAVERSAL_DEPTH` (256) times. It is called from the async `buildModulesMap` function. On deep directory trees or network filesystems, this blocks the event loop. The caching mitigates repeated calls but the first invocation per unique start directory is synchronous and potentially slow.
- Impact: Event loop blocking on first call per unique directory. Mitigated by caching, but the comment on line 23 acknowledges this concern.
- Fix: This is a known trade-off documented in the code. An async version (`findProjectRootAsync`) could be offered as an alternative for use in the async `buildModulesMap` path. Not blocking for this PR.

## Suggestions (Lower Confidence)

- **Condition enum growing without visitor pattern** - `crates/mds-core/src/ast.rs:31-40` (Confidence: 70%) -- The `Condition` enum now has 4 variants (Truthy, Not, Eq, NotEq). The evaluator, validator, and parser each independently match on all variants. If more condition types are added (e.g., `<`, `>`, `in`, `matches`), each new variant requires changes in 3+ files. A visitor or shared evaluation trait would localize the changes, but at the current scale (4 variants) the direct match approach is appropriate.

- **Dual-build script complexity** - `packages/bundler-utils/package.json:28`, `packages/webpack-loader/package.json:24` (Confidence: 65%) -- The build scripts chain `tsc -p tsconfig.json && tsc -p tsconfig.cjs.json && node -e "..."` inline. If a third output format is added, these one-liners become unmaintainable. Consider extracting to a `build.sh` or using a task runner. At two outputs this is acceptable.

- **`_esmImport` could be extracted to `@mds/bundler-utils`** - `packages/webpack-loader/src/index.ts:17-20` (Confidence: 65%) -- If other packages need the same CJS-safe dynamic import workaround, the `_esmImport` helper could live in `bundler-utils` alongside `LazyInit` and `createMdsTransformer`. Currently only webpack-loader needs it, so extraction is premature.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The architecture of this PR is well-structured overall. The Rust core follows a clean pipeline (lexer -> parser -> AST -> validator -> resolver -> evaluator) and the new features (negation, equality, elseif) are added consistently across all pipeline stages. The `Condition` enum expansion and `IfBlock.elseif_branches` field are natural extensions of the existing AST design. The `EvalContext` bundle pattern in the evaluator keeps the threading of mutable state clean.

The CJS dual-build approach for webpack-loader and bundler-utils is a pragmatic solution to the ESM/CJS interop problem, well-documented with the TypeScript issue link and CSP caveats. The `findProjectRoot` change to walk up to `.git`/`.mdsroot` markers correctly mirrors the Rust `NativeFs::find_project_root` behavior, fixing the cross-directory import limitation.

Conditions for approval:
1. Add a cache-clearing mechanism for `projectRootCache` (the HIGH finding) -- at minimum a test-only reset, ideally a public `clearProjectRootCache()` for watch-mode consumers.

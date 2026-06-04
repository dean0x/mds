# Complexity Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Prior Resolutions**: Cycle 2 addressed 18/21 issues. Key complexity fixes: normalizeRootKey helper extracted, module-scanner depth limit added, browser.ts now delegates to wasm.ts. This cycle verifies those fixes landed correctly and checks for remaining/new issues.

## Issues in Your Changes (BLOCKING)

### HIGH

**Inline ternary in compile/check methods reduces readability** - `packages/mds/src/backend/wasm.ts:117,123`
**Confidence**: 82%
- Problem: The ternary `vars !== undefined ? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS` is duplicated on lines 117 and 123. While each instance is not deeply nested, the expression requires mental parsing: one must understand `DEFAULT_COMPILE_OPTS` is frozen, that spreading it creates a new object, and that the branch avoids unnecessary allocation. The duplication means maintaining the options construction logic in two places.
- Fix: Extract a small helper to build compile options, eliminating the duplicated ternary:
```typescript
function compileOpts(options?: CompileOptions): { filename: string; modules: Record<string, string>; vars?: Record<string, unknown> } {
  const vars = varsOpt(options);
  return vars !== undefined ? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS;
}
```
Then in `compile`: `return wasm.compile(source, compileOpts(options));`
And in `check`: `return wasm.check(source, compileOpts(options));`

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`scan()` function is 80 lines with 4 levels of nesting** - `packages/mds/src/util/module-scanner.ts:133-213`
**Confidence**: 85%
- Problem: The inner `scan()` function spans lines 133-213 (80 lines). It contains depth check, visited check, module count check, concurrent filesystem calls, symlink check, TOCTOU check, project root check, aggregate size check, file read, and child import traversal. Each security guard adds a conditional, bringing cyclomatic complexity to approximately 8. This was partially addressed in cycle 2 (validateImportPath extraction), but the function still has 7 early-exit conditionals and 4 nesting levels (function > Promise.all > map > await scan).
- Fix: Extract the security/resource validation into a helper (e.g., `readAndValidateModule`) that takes `absolutePath`, `projectRoot`, `stats`, `resolved` and returns `content`. This would reduce `scan()` to orchestration logic only:
```typescript
async function readAndValidateModule(
  absolutePath: string,
  projectRoot: string,
): Promise<{ content: string; size: number }> {
  const [stats, resolved] = await Promise.all([lstat(absolutePath), realpath(absolutePath)]);
  if (stats.isSymbolicLink()) {
    throw new Error(`security: symlink detected at ${absolutePath}`);
  }
  if (resolved !== absolutePath) {
    throw new Error(`security: path ${absolutePath} resolved to unexpected location ${resolved}`);
  }
  if (!absolutePath.startsWith(projectRoot + '/') && absolutePath !== projectRoot) {
    throw new Error(`security: path escapes project root`);
  }
  const content = await readFile(absolutePath, 'utf-8');
  return { content, size: stats.size };
}
```

**`_init()` function has try/catch inside a for-loop with implicit fallthrough** - `packages/mds/src/backend/wasm.ts:59-94`
**Confidence**: 80%
- Problem: The `_init()` function iterates over `candidates`, catches failures silently to try the next candidate, then throws a combined error after exhaustion. The control flow -- loop with try/catch, conditional `mod.default` call, early return on success -- creates 3 levels of nesting and moderate cyclomatic complexity (approximately 5). The last-error capture pattern (`loadError`) is a common source of confusion.
- Fix: Consider extracting a `tryLoadCandidate(candidate, options)` function that returns `WasmModule | null`, simplifying the loop to a find-first-success pattern:
```typescript
async function tryLoadCandidate(candidate: string, options?: InitOptions): Promise<WasmModule | null> {
  try {
    const mod = require(candidate) as WasmModule;
    if (typeof mod.default === 'function') await mod.default(options?.wasmUrl);
    return mod;
  } catch { return null; }
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Repetitive test assertion patterns in browser.spec.mjs** - `packages/mds/__test__/browser.spec.mjs:27-81` (Confidence: 65%) -- Tests U-BR1 through U-BR4 share near-identical structure (assert.throws/rejects with an error predicate checking instanceof and message.includes). Could be reduced with a shared helper, but test readability may be more important.

- **`normalizeVirtualKey` handles two distinct cases in one function** - `packages/mds/src/util/module-scanner.ts:27-72` (Confidence: 70%) -- The `base.length === 0` branch (root entry point) vs the relative resolution branch are logically independent algorithms. Separating them would reduce cyclomatic complexity from ~7 to ~3 each, but the function was recently refactored (cycle 2) and the current length (45 lines) is within acceptable range.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions: The HIGH-severity duplicated ternary in wasm.ts should be extracted to a helper before merge. The MEDIUM items are recommended but not blocking.

Overall, this PR shows strong complexity discipline. The browser.ts refactoring reduced complexity significantly by delegating to wasm.ts. The module-scanner depth guard was added cleanly. The main remaining concern is the duplicated options-construction logic in the WASM backend, which is a straightforward extraction.

# Code Review Summary

**Branch**: feat-17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25 00:40
**Reviewers**: 9 (security, architecture, performance, complexity, consistency, regression, testing, reliability, typescript)
**PR**: #30

## Merge Recommendation: CHANGES_REQUESTED

**Reasoning**: The core bug fix is correct and well-tested, but multiple reviewers have flagged the same HIGH-severity issue: code duplication in `compileFile`/`checkFile` where the extract-and-delete pattern is copy-pasted verbatim. Additionally, test consistency issues in the WASM backend test file require alignment with existing patterns. The working tree contains an uncommitted `prepareFileArgs` refactoring that resolves the duplication -- this must be committed before merge.

---

## Convergence Status

### Issues Flagged by Multiple Reviewers (High Confidence)

| Issue | Flagged By | Consensus | Recommendation |
|-------|-----------|-----------|-----------------|
| **Code duplication: extract-and-delete in compileFile/checkFile** | consistency, complexity, regression, typescript | **5 reviewers** | Extract `prepareFileArgs` helper (working tree has this) |
| **U-WCF6 uses raw exec instead of runWasm helper** | consistency, testing, typescript | **3 reviewers** | Use `runWasm()` for consistency |
| **buildModulesMap return type ambiguity** | architecture, complexity (indirect) | **2 reviewers** | Document or refactor contract |
| **U-WCF7/U-WCF8 parity limited to simple.mds** | testing | **1 reviewer** | Add parity test for import scenario |
| **Silent fallback masks missing entry** | reliability | **1 reviewer** | Replace `?? ''` with assertion |

### Convergent Finding: Uncommitted Refactoring

All duplication-focused reviewers (consistency, complexity, regression, typescript) note that the **working tree contains improvements not in the committed diff**:
- `prepareFileArgs()` helper extracted in `src/node.ts`
- Shared `helpers.mjs` imports in test file
- Unified `runScript()` / `wasmEnv()` / `nativeEnv()` helpers

**Consensus**: These improvements should be committed as part of this PR. They directly address reviewer feedback.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 5 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 2 | 0 |

**Total blocking issues**: 7 (2 HIGH, 5 MEDIUM)

---

## Blocking Issues (HIGH Severity)

### Issue 1: Duplicate extract-and-delete pattern blocks merge

**Location**: `packages/mds/src/node.ts:67-73, 78-81`
**Confidence**: 88% (flagged by 5 reviewers with 82-90% individual confidence)
**Severity**: HIGH

**Problem**: The three-line sequence (`const source = modules[entryFilename] ?? ''`, `delete modules[entryFilename]`, WASM call) is copy-pasted identically between `compileFile` and `checkFile`. The comment on `checkFile` says "Same fix as compileFile", acknowledging the duplication. This violates the DRY principle and creates maintenance risk: if the logic ever needs adjustment, both sites must be updated in lockstep.

**Fix**: Extract a shared `prepareFileArgs` helper:
```typescript
async function prepareFileArgs(
  path: string,
  options: FileOptions | undefined,
): Promise<{ source: string; opts: ReturnType<typeof fileOpts> }> {
  const { entryFilename, modules } = await buildModulesMap(path, (src) => wasmModule.scanImports(src));
  const source = modules[entryFilename] ?? '';
  delete modules[entryFilename];
  return { source, opts: fileOpts(entryFilename, modules, options) };
}
```

**Status**: The working tree already contains this refactoring. It must be committed before merge.

---

### Issue 2: Test file violates established fixture path pattern

**Location**: `packages/mds/__test__/wasm-compileFile.spec.mjs:13-23`
**Confidence**: 92%
**Severity**: HIGH

**Problem**: The new test file re-declares `__dirname`, `SIMPLE_MDS`, `IMPORT_CONSUMER_MDS`, and `ENTRY_MDS` locally instead of importing them from `helpers.mjs` (where they already exist and are exported). Every other test file (`compileFile.spec.mjs`, `check.spec.mjs`, etc.) imports these from `helpers.mjs`. This violates the established codebase pattern and creates a maintenance risk: if fixtures move or paths change, this file must be updated separately.

**Fix**: Replace with imports from `./helpers.mjs`:
```javascript
import { SIMPLE_MDS, IMPORT_CONSUMER_MDS, ENTRY_MDS, __dirname } from './helpers.mjs';
import path from 'node:path';

const pkgRoot = path.join(__dirname, '..');
```

---

## Blocking Issues (MEDIUM Severity)

### Issue 3: buildModulesMap return type lacks documentation

**Location**: `packages/mds/src/node.ts:68-76` (buildModulesMap call)
**Confidence**: 82%
**Severity**: MEDIUM

**Problem**: `buildModulesMap()` returns `{ entryFilename, modules }` but the contract does not document whether `modules` includes or excludes the entry file. Callers must infer this through defensive code. The fix in `prepareFileArgs` works correctly but the ambiguity lives in the return type's JSDoc.

**Fix**: Add JSDoc to `BuildModulesMapResult.modules` documenting that it *includes* the entry file source keyed by `entryFilename`. Alternatively, refactor to return `{ entryFilename, entrySource, imports }` where `imports` excludes the entry.

---

### Issue 4: Silent empty-source fallback masks missing entry file

**Location**: `packages/mds/src/node.ts:73`
**Confidence**: 82%
**Severity**: MEDIUM

**Problem**: The expression `modules[entryFilename] ?? ''` silently falls back to an empty string if the entry filename is not in the map. While `buildModulesMap` should always populate this key, the fallback hides a potential invariant violation. A broken invariant should surface immediately rather than produce incorrect empty output.

**Fix**: Replace with explicit assertion:
```typescript
const source = modules[entryFilename];
if (source === undefined) {
  throw new Error(
    `@mds/mds: entry file "${entryFilename}" not found in modules map — this is a bug in buildModulesMap`,
  );
}
delete modules[entryFilename];
```

---

### Issue 5: Duplicate subprocess runner functions in tests

**Location**: `packages/mds/__test__/wasm-compileFile.spec.mjs:31-61`
**Confidence**: 82%
**Severity**: MEDIUM

**Problem**: `runWasm()` and `runNative()` share nearly identical subprocess-spawning logic, differing only in the `env` object. This duplicates logic and is inconsistent with the DRY principle seen elsewhere (e.g., `compileOpts`/`fileOpts` helpers). The pattern creates maintenance burden if subprocess configuration ever needs adjustment.

**Fix**: Consolidate into a single `runScript(script, env)` and create small `wasmEnv()` / `nativeEnv()` helpers. The working tree contains this refactoring.

---

### Issue 6: U-WCF6 error path test diverges from pattern

**Location**: `packages/mds/__test__/wasm-compileFile.spec.mjs:129-151`
**Confidence**: 85%
**Severity**: HIGH (testing calls it HIGH, others MEDIUM)

**Problem**: Test U-WCF6 manually spawns a subprocess with inline `exec()` call instead of using the `runWasm()` helper that all other tests use. The inline subprocess catches errors inside the script and returns `{ threw: true }`, which means the test never verifies the *error message content* -- only that *something* threw. This is weaker than the existing `compileFile.spec.mjs` (U-CF7) which uses `assert.rejects` and verifies `err instanceof Error`. The manual subprocess approach also duplicates the env/cwd/timeout configuration.

**Fix**: Use `runWasm()` with a script that lets the error propagate (no try/catch). The subprocess will exit non-zero and `exec()` will reject. Assert on rejection:
```javascript
test('U-WCF6: WASM compileFile on nonexistent file rejects with error', async () => {
  await assert.rejects(
    () => runWasm(`
      import { init, compileFile } from './dist/node.js';
      await init();
      const r = await compileFile('/nonexistent/path/file.mds');
      process.stdout.write(JSON.stringify(r));
    `),
    (err) => {
      assert.ok(err instanceof Error, 'must reject with Error');
      return true;
    },
  );
});
```

---

### Issue 7: Parity tests limited to simple.mds — missing import scenario

**Location**: `packages/mds/__test__/wasm-compileFile.spec.mjs:153-187`
**Confidence**: 82%
**Severity**: MEDIUM

**Problem**: The parity tests (U-WCF7, U-WCF8) compare WASM vs native output only for `simple.mds`. The bug being fixed (filename collision) was specifically triggered when imports are present, yet the parity tests do not verify parity for files with imports. `IMPORT_CONSUMER_MDS` and `ENTRY_MDS` are available in the fixture set and would provide more meaningful parity coverage.

**Fix**: Add a parity test (or extend U-WCF7) to compare WASM vs native output for `IMPORT_CONSUMER_MDS`:
```javascript
test('U-WCF7b: WASM compileFile with imports matches native (parity)', async () => {
  const script = `
    import { init, compileFile } from './dist/node.js';
    await init();
    const r = await compileFile(${JSON.stringify(IMPORT_CONSUMER_MDS)});
    process.stdout.write(JSON.stringify({ output: r.output, warnings: r.warnings, dependencies: r.dependencies }));
  `;
  const [wasmResult, nativeResult] = await Promise.all([
    runWasm(script),
    runNative(script),
  ]);
  assert.equal(wasmResult.output, nativeResult.output);
});
```

---

## Approved / Non-Blocking

### Security Review
**Score**: 9/10 — **APPROVED**

No CRITICAL or HIGH security issues found. The fix does not introduce new input trust boundaries, and test subprocess isolation is correct (uses `execFile`, not `exec`). Minor suggestions only.

---

### Architecture Review
**Score**: 8/10 — **APPROVED**

The fix is architecturally sound. The core bug is addressed correctly by extracting the entry source and removing it from the map before passing to WASM. The note about `buildModulesMap` contract ambiguity is a non-blocking architecture improvement.

---

### Performance Review
**Score**: 9/10 — **APPROVED**

No blocking performance issues. Sequential subprocess spawning in tests is acceptable for a test suite of this size.

---

### Regression Review
**Score**: 9/10 — **APPROVED_WITH_CONDITIONS**

The condition is to resolve the uncommitted `prepareFileArgs` refactoring -- either commit it or discard it. No API regressions detected. Package.json `files` change is safe.

---

## Summary of Changes

| File | Change | Impact |
|------|--------|--------|
| `packages/mds/src/node.ts` | Bug fix: extract entry source, delete from modules, pass separately to WASM | Fixes filename collision in `compileFile`/`checkFile` |
| `packages/mds/__test__/wasm-compileFile.spec.mjs` | 8 new subprocess-isolated tests (U-WCF1 through U-WCF8) | Validates fix and cross-backend parity |
| `packages/mds/package.json` | Remove `wasm/` from `files` array | Removes stale package manifest entry |

---

## Action Plan (Before Merge)

1. **[REQUIRED]** Commit the `prepareFileArgs` refactoring to `packages/mds/src/node.ts`
   - Addresses HIGH-severity duplication flagged by 5 reviewers
   - Working tree already has this change

2. **[REQUIRED]** Fix fixture path pattern in test file
   - Import `SIMPLE_MDS`, `IMPORT_CONSUMER_MDS`, `ENTRY_MDS` from `helpers.mjs`
   - Remove local declarations to match project pattern

3. **[REQUIRED]** Replace silent fallback with explicit assertion
   - Change `modules[entryFilename] ?? ''` to explicit check + throw
   - Catches invariant violations at definition time

4. **[REQUIRED]** Consolidate test subprocess runners
   - Extract `runScript(script, env)`, `wasmEnv()`, `nativeEnv()` helpers
   - Remove duplicate `runWasm`/`runNative` logic
   - Working tree has this refactoring

5. **[REQUIRED]** Fix U-WCF6 error test to use shared pattern
   - Use `runWasm()` with `assert.rejects` pattern
   - Verify error message content

6. **[RECOMMENDED]** Add parity test for import scenario
   - Test U-WCF7/U-WCF8 for `IMPORT_CONSUMER_MDS`
   - Validates the fix for the actual bug-trigger scenario

7. **[OPTIONAL]** Document buildModulesMap return contract
   - Add JSDoc clarifying whether `modules` includes entry

---

## Convergence Analysis

**Consensus Reviewers**: 5 (consistency, complexity, regression, typescript, testing)
**Blocking vs Informational**: 7 blocking issues, 2 pre-existing

All reviewers agree on:
- The core bug fix is correct
- Code duplication must be eliminated (HIGH priority)
- Test consistency must align with project patterns (HIGH priority)
- Uncommitted improvements in working tree should be included

No reviewer disagreement detected. All recommendations align on merge readiness criteria.

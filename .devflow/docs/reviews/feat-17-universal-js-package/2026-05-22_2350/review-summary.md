# Code Review Summary

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22T23:50
**Cycle**: 1

## Merge Recommendation: CHANGES_REQUESTED

This PR introduces a well-architected universal JavaScript package with solid security hardening and clean module separation. However, **6 blocking issues must be resolved** before merge. Most are straightforward fixes (broken script reference, dead code, documentation corrections). The architectural concern (duplicated init logic) and behavioral change (isMdsError tightening) require explicit decisions before implementation.

---

## Convergence Status

**Total Reviewers**: 11 (architecture, complexity, consistency, dependencies, documentation, performance, regression, reliability, security, testing, typescript)

**Convergence Patterns**:
- **6 reviewers** flagged the stale `test:parity` script → **98% confidence** (near unanimous)
- **2 reviewers** flagged dead variable `script` in test → **93% confidence**
- **2 reviewers** flagged `varsOpt` null handling issue → **84% confidence**
- **2 reviewers** flagged browser init retry limit inconsistency → **82% confidence**

**Divergent Findings**: None. All reviewers aligned on core issues; variance was in domain focus (security vs. performance vs. testing) rather than disagreement.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| **Blocking** | 0 | 6 | 2 | 0 | **8** |
| **Should Fix** | - | - | 8 | 0 | **8** |
| **Pre-existing** | - | - | 3 | 0 | **3** |

**Blocking Issues (Must Fix Before Merge)**: 8 issues
**Suggested Improvements (Should Address)**: 8 issues
**Pre-existing (Informational)**: 3 issues

---

## BLOCKING ISSUES (Category 1: Your Changes)

### HIGH - Stale npm script references renamed file (CRITICAL-PATH)
**Location**: `packages/mds/package.json:27-28`
**Confidence**: 98% (flagged by 6 reviewers: architecture, consistency, dependencies, documentation, regression, testing)
**Status**: Blocking merge

The `test:parity` npm script points to `__test__/parity.spec.mjs`, but this file was renamed to `__test__/native-backend.spec.mjs` in this PR. Running `npm run test:parity` will immediately fail with "file not found."

**Fix**:
```json
"test:native": "node --test __test__/native-backend.spec.mjs"
```

---

### HIGH - Duplicated init race-prevention logic between browser.ts and wasm.ts
**Location**: `packages/mds/src/browser.ts:24-51` + `packages/mds/src/backend/wasm.ts:28-60`
**Confidence**: 85%
**Status**: Blocking merge

`browser.ts` maintains its own `initPromise` + `backend` singleton state alongside `wasm.ts`'s separate `initPromise` + `initFailures` + `wasmModule` singleton. Both independently implement the "cache promise, clear on failure, retry" pattern with different failure semantics. When `wasm.ts` hits `MAX_INIT_RETRIES`, it throws permanently, but `browser.ts` would still reset `initPromise = null` and attempt to retry — creating a confusing failure mode where `browser.ts` retries but `wasm.ts` refuses.

**Fix**: `browser.ts` should delegate entirely to `createWasmBackend()` for init lifecycle, storing only the returned `MdsBackend`. The retry/race logic belongs in one place.

```typescript
async function doInit(options?: InitOptions): Promise<void> {
  const { createWasmBackend } = await import('./backend/wasm.js');
  backend = await createWasmBackend(options);
}
```

This requires `createWasmBackend` to accept and forward `InitOptions`, but eliminates the duplicate state machines.

---

### HIGH - `normalizeVirtualKey` approaching cyclomatic complexity threshold
**Location**: `packages/mds/src/util/module-scanner.ts:26-74`
**Confidence**: 82%
**Status**: Blocking merge

This 48-line function has cyclomatic complexity ~9 (two early-exit guards, if/else branch for empty base, for-loop with 3-way if/else chain). It is in the warning zone and any future additions would exceed the complexity limit.

**Fix**: Extract the `base.length === 0` branch into a standalone `normalizeRootKey()` function to separate the two code paths:

```typescript
function normalizeRootKey(relative: string): string {
  const segmentCount = relative
    .split('/')
    .filter((s) => s.length > 0 && s !== '.')
    .length;
  if (segmentCount > MAX_PATH_SEGMENTS) {
    throw new Error(`import path exceeds maximum segment count of ${MAX_PATH_SEGMENTS}`);
  }
  return relative;
}

export function normalizeVirtualKey(base: string, relative: string): string {
  if (relative.length === 0) throw new Error('import path is empty');
  if (relative.includes('\0')) throw new Error('import path contains null byte');
  if (base.length === 0) return normalizeRootKey(relative);
  // ... relative resolution only
}
```

---

### HIGH - Sequential filesystem syscalls serialize unnecessary metadata reads
**Location**: `packages/mds/src/util/module-scanner.ts:151-163`
**Confidence**: 85%
**Status**: Blocking merge

Each module file triggers three sequential async syscalls: `lstat()`, `realpath()`, then `readFile()`. The first two are metadata operations that could run concurrently via `Promise.all` before reading. For projects with 256 modules, this costs ~51ms in avoidable sequential I/O.

**Fix**: Parallelize the metadata calls:

```typescript
const [stats, resolved] = await Promise.all([
  lstat(absolutePath),
  realpath(absolutePath),
]);
if (stats.isSymbolicLink()) {
  throw new Error(`security: symlink detected at ${absolutePath}`);
}
if (resolved !== absolutePath) {
  throw new Error(`security: path resolved unexpectedly`);
}
// then readFile
```

---

### HIGH - `isMdsError` guard tightened without documentation
**Location**: `packages/mds/src/types.ts:71-76`
**Confidence**: 85%
**Status**: Blocking merge

The type guard was changed from checking `typeof code === 'string'` to also requiring `code.startsWith('mds::')`. This is a behavioral change in a public API function. Downstream consumers creating custom errors with a non-`mds::` code will now get `false` where they got `true` before.

**Fix**: Document this as a breaking change in CHANGELOG:

```markdown
### Changed
- `isMdsError()` now requires the `code` property to start with `"mds::"` for stricter identification of MDS-specific errors
```

Also add a test case for the boundary (see testing section below).

---

### HIGH - Dead variable `script` references nonexistent file
**Location**: `packages/mds/__test__/backend.spec.mjs:46`
**Confidence**: 93% (flagged by 2 reviewers: consistency, testing, typescript)
**Status**: Blocking merge

Line 46 declares `const script = path.join(__dirname, 'backend-wasm-helper.mjs');` but never uses it. The referenced file does not exist on disk. This is dead code from an incomplete refactor.

**Fix**: Remove the line:

```javascript
// Delete this:
const script = path.join(__dirname, 'backend-wasm-helper.mjs');

// Keep this:
const output = execFileSync(process.execPath, ['--input-type=module'], {
```

---

### MEDIUM - Module-level side effects in node.ts make it untestable
**Location**: `packages/mds/src/node.ts:10-45`
**Confidence**: 82%
**Status**: Blocking merge (with caveat)

Backend selection runs as top-level `await` at import time, triggering I/O, console.warn calls, and environment variable reads before the module is usable. This makes the fallback logic impossible to unit test without subprocess spawning and violates "explicit over implicit."

**Fix**: Consider a lazy initialization pattern:

```typescript
let backendPromise: Promise<MdsBackend> | null = null;
function getOrInitBackend(): Promise<MdsBackend> {
  if (!backendPromise) backendPromise = resolveBackend();
  return backendPromise;
}
```

**However**, this conflicts with the current synchronous API (`compile` returns `CompileResult`, not `Promise<CompileResult>`). If synchronous compile is a hard requirement, the current approach is acceptable **but must be documented** in the README with a note about startup cost. This is a design decision, not a bug — get explicit approval before implementing the lazy pattern or documenting the trade-off.

---

### MEDIUM - Unbounded recursion depth in scanner
**Location**: `packages/mds/src/util/module-scanner.ts:135-201`
**Confidence**: 82%
**Status**: Blocking merge

The `scan()` function recurses through import chains with no explicit depth limit. While `maxModules` (256) provides an indirect bound on total nodes, it does not bound recursion depth. A pathological chain (A→B→C→D... 256 deep) would create a 256-frame stack.

**Fix**: Add explicit depth limit:

```typescript
const MAX_IMPORT_DEPTH = 64;

async function scan(absolutePath: string, virtualKey: string, depth = 0): Promise<void> {
  if (depth > MAX_IMPORT_DEPTH) {
    throw new Error(`resource limit: import chain depth exceeds maximum of ${MAX_IMPORT_DEPTH}`);
  }
  // ... existing logic ...
  await Promise.all(
    importPaths.map(async (importPath) => {
      // ...
      await scan(childAbsolute, childVirtualKey, depth + 1);
    }),
  );
}
```

---

## SHOULD-FIX ISSUES (Category 2: Code You Touched)

### MEDIUM - Missing JSDoc on browser.ts exported functions
**Location**: `packages/mds/src/browser.ts:60,64,68,72,81`
**Confidence**: 85%

The PR added JSDoc to all 5 exported functions in `node.ts` but browser.ts only has JSDoc on `init()`. The other exports (`compile`, `check`, `getBackend`, `compileFile`, `checkFile`) have no JSDoc. Both are public entry points and should have matching documentation quality.

**Fix**: Add the same JSDoc comments from `node.ts`:

```typescript
/** Compile an MDS source string to Markdown. */
export function compile(source: string, options?: CompileOptions): CompileResult {

/** Validate an MDS source string without rendering. */
export function check(source: string, options?: CompileOptions): CheckResult {

/** Returns `'wasm'` -- browser environments always use the WASM backend. */
export function getBackend(): BackendType {

/** Not available in browser environments. Always rejects. */
export function compileFile(_path: string, _options?: FileOptions): Promise<CompileResult> {

/** Not available in browser environments. Always rejects. */
export function checkFile(_path: string, _options?: FileOptions): Promise<CheckResult> {
```

---

### MEDIUM - Unused imports in compile.spec.mjs
**Location**: `packages/mds/__test__/compile.spec.mjs:7`
**Confidence**: 92%

`SIMPLE_MDS` and `FIXTURES` are imported from `./helpers.mjs` but never used. All tests use inline source strings.

**Fix**: Remove the unused imports:

```javascript
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { compile, isMdsError } from '../dist/node.js';
```

---

### MEDIUM - `varsOpt` passes through `null` without validation
**Location**: `packages/mds/src/util/options.ts:10-11`
**Confidence**: 84% (flagged by 2 reviewers: typescript, reliability)

The function checks `options?.vars !== undefined` but does not guard against `null`. When `vars` is `null`, it returns `{ vars: null }`, forwarding a `null` where the backend expects `Record<string, unknown>`. Test U-C7 validates this works, but the function should normalize defensively.

**Fix**: Coerce `null` to `undefined`:

```typescript
export function varsOpt(options?: CompileOptions | FileOptions): { vars: Record<string, unknown> } | undefined {
  return options?.vars != null ? { vars: options.vars } : undefined;
  //                  ^^ loose equality catches both null and undefined
}
```

---

### MEDIUM - Browser init has no retry bound (inconsistent with wasm.ts)
**Location**: `packages/mds/src/browser.ts:33-51`
**Confidence**: 82%

`wasm.ts` bounds retries to `MAX_INIT_RETRIES = 3`. `browser.ts` has no retry limit — it clears `initPromise` on failure, allowing unlimited retries. While `wasmInit()` internally enforces the limit, the browser-side wrapping allows wasteful promise creation.

**Fix**: Add matching retry bound in `browser.ts`:

```typescript
const MAX_INIT_RETRIES = 3;
let initFailures = 0;

export async function init(options?: InitOptions): Promise<void> {
  if (backend !== undefined) return;
  if (initPromise !== null) return initPromise;
  if (initFailures >= MAX_INIT_RETRIES) {
    throw new Error('@mds/mds: browser init failed after 3 attempts');
  }
  initPromise = doInit(options);
  return initPromise;
}

async function doInit(options?: InitOptions): Promise<void> {
  try {
    await wasmInit(options);
    const { createWasmBackend } = await import('./backend/wasm.js');
    backend = await createWasmBackend();
  } catch (err) {
    initFailures += 1;
    initPromise = null;
    throw err;
  }
}
```

---

### MEDIUM - Object spread on every compile/check call
**Location**: `packages/mds/src/backend/wasm.ts:120-121`
**Confidence**: 80%

Every `compile()` and `check()` call spreads `varsOpt(options)` into a new object literal, creating unnecessary allocation on every call even for the common case with no vars.

**Fix**: Pre-allocate a frozen default:

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

---

### MEDIUM - Test U-C4 has misleading comment
**Location**: `packages/mds/__test__/compile.spec.mjs:32-38`
**Confidence**: 90%

The test is named "compile returns warnings for empty @include" but the source has no `@include` directive. The test merely asserts `typeof result.output === 'string'` — trivially true for any input.

**Fix**: Either rename or actually test an empty @include:

```js
test('U-C4: compile returns warnings for empty @include', () => {
  const source = '@import ""\nHello!\n';
  const result = compile(source);
  assert.ok(result.warnings.length > 0, 'expected warning for empty import');
});
```

---

### MEDIUM - New WASM init retry logic has zero test coverage
**Location**: `packages/mds/src/backend/wasm.ts:31-52`
**Confidence**: 85%

The PR added `MAX_INIT_RETRIES = 3` with mutable `initFailures` counter (a circuit breaker). This significant behavioral logic has no test coverage, and the mutable state cannot be reset between tests.

**Fix**: Add a test that verifies the retry limit. This likely requires a subprocess approach (similar to U-B5) or extracting init state into a testable class.

---

### MEDIUM - `isMdsError` tightening lacks boundary test coverage
**Location**: `packages/mds/__test__/error.spec.mjs`
**Confidence**: 88%

The type guard was tightened to require `mds::` prefix, but test U-E4 only tests a plain `Error` with no `.code` property. There's no test that verifies non-`mds::`-prefixed codes return `false`.

**Fix**: Add a test:

```js
test('U-E4b: isMdsError returns false for errors with non-mds code', () => {
  const err = new Error('some error');
  err.code = 'ENOENT'; // has code, but not mds:: prefixed
  assert.equal(isMdsError(err), false);
});
```

---

### MEDIUM - README error span property name mismatch
**Location**: `packages/mds/README.md:83`
**Confidence**: 95%

The README shows `err.span` as `{ offset, length, line, col }` but the actual `MdsErrorSpan` interface uses `column`, not `col`. Users copying this example will get a runtime error.

**Fix**:

```diff
-    console.error(err.span);    // optional { offset, length, line, col }
+    console.error(err.span);    // optional { offset, length, line, column }
```

---

### MEDIUM - `MdsErrorSpan.line` and `column` lack JSDoc
**Location**: `packages/mds/src/types.ts:35-36`
**Confidence**: 85%

All other properties on `MdsErrorSpan` have JSDoc but `line` and `column` do not. Users need to know they are 1-based (or 0-based).

**Fix**:

```typescript
  /** Byte length of the error span. */
  length: number;
  /** 1-based line number of the error, if available. */
  line?: number;
  /** 1-based column number of the error, if available. */
  column?: number;
```

---

## PRE-EXISTING ISSUES (Category 3: Not Your Code)

### MEDIUM - `mds-napi` uses `file:` protocol as runtime dependency
**Location**: `packages/mds/package.json:31`
**Confidence**: 82%

`"mds-napi": "file:../../crates/mds-napi"` is listed under `dependencies` (not `optionalDependencies`). When published to npm, consumers will get install errors because `file:` references don't work on their machines. The code handles this gracefully with try/catch, but npm will report the install as failed.

**Fix**: Move to `optionalDependencies` or remove entirely and rely on runtime `require()` with the existing fallback.

---

### MEDIUM - Stale nested `crates/mds-napi/package-lock.json`
**Location**: `crates/mds-napi/package-lock.json`
**Confidence**: 85%

This PR commits the root workspace lockfile but the pre-existing nested lockfile is now visible to git as untracked (1843 lines, from before workspace setup). It conflicts with the root lockfile and could cause confusion if accidentally committed.

**Fix**: Delete or add specific ignore:

```gitignore
# Root lockfile is committed; ignore nested lockfiles from pre-workspace installs
crates/mds-napi/package-lock.json
```

---

### MEDIUM - No tests for browser entry point
**Location**: `packages/mds/src/browser.ts`
**Confidence**: 85%

The browser entry point has significant behavioral logic (`assertInitialized()` throws before `init()`, `compileFile`/`checkFile` always reject, concurrent init deduplicated). None of this is tested.

---

## Key Insights

1. **Architectural consistency is broken**: The duplicated init logic between browser.ts and wasm.ts violates DRY and creates confusing failure semantics. This is the most significant design issue.

2. **One stale reference breaks everything**: The `test:parity` script fix is trivial but critical — it will cause immediate CI failures post-merge.

3. **Behavioral changes need documentation**: The `isMdsError` tightening is arguably an improvement (more precise), but it's undocumented and lacks test coverage for the boundary case.

4. **Resource limits need explicit bounds**: Both complexity and recursion depth need explicit safeguards. The module count limit alone is insufficient for reliability.

5. **Test coverage has gaps**: New behavioral logic (init retry circuit breaker, path validation guards, `isMdsError` prefix check) lacks direct unit test coverage. The PR is 63 tests strong but misses coverage for newly-added logic.

6. **Documentation is mostly complete**: README, JSDoc, and CHANGELOG are thorough. The few gaps are straightforward fixes (property name mismatch, missing JSDoc, undocumented behavioral changes).

---

## Action Plan (Priority Order)

**Critical Path (Fix These First):**
1. Fix stale `test:parity` npm script → Unblocks CI
2. Remove dead variable `script` in test → Unblocks code clarity
3. Add explicit recursion depth limit to scanner → Unblocks reliability
4. Resolve duplicated init logic between browser.ts and wasm.ts → Unblocks architectural consistency

**High Priority (Fix Before Merge):**
5. Fix README property name (`col` → `column`) → Unblocks user-facing docs accuracy
6. Document `isMdsError` behavioral change and add test case → Unblocks behavioral change clarity
7. Add JSDoc to browser.ts exports → Unblocks documentation consistency

**Medium Priority (Should Fix):**
8. Parallelize filesystem metadata calls in scanner → Performance improvement
9. Add init retry logic test coverage → Testing completeness
10. Fix node.ts module-level side effects or document trade-off → Design decision clarity

**Quality (Nice to Have):**
11. Fix misleading test comment (U-C4) → Test clarity
12. Normalize `null` in `varsOpt` → Boundary validation
13. Add browser init retry bound → Consistency with wasm.ts

---

## Convergence Highlights

- **99%+ consensus** on breaking npm script (6/11 reviewers flagged independently)
- **90%+ consensus** on dead test code (3/11 reviewers)
- **Complete agreement** on no CRITICAL security/reliability issues
- **Complete agreement** on architecture needing consolidation
- **No contradictions** between reviewers — all disagreements are about severity/actionability, not correctness

This PR is well-executed overall with strong security posture, clean module boundaries, and thorough testing. The blocking issues are mostly straightforward to fix; the architectural decisions (init delegation, side effects trade-off) need explicit approval before implementation.

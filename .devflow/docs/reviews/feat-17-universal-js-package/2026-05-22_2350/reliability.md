# Reliability Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### HIGH

**Race condition in parallel `aggregateSize` accumulation** - `packages/mds/src/util/module-scanner.ts:176`
**Confidence**: 90%
- Problem: `aggregateSize` is a shared mutable variable incremented inside `scan()`, which is invoked in parallel via `Promise.all()` at line 191. While JS is single-threaded, the `await` points between `lstat`, `realpath`, and `readFile` yield execution, meaning multiple parallel `scan()` calls can each read `aggregateSize`, decide they are under the limit, and then each add their file size. The comment at line 172-175 acknowledges this risk and claims pre-reserving via `stats.size` fixes it. However, the fix only works if the `+=` and `>` check are atomic relative to other parallel scans -- and they are, because the `+=` and the `if` check happen synchronously (no `await` between them). The real issue is that `stats.size` (bytes on disk) can differ from the actual content length read by `readFile` with `'utf-8'` encoding. For multi-byte UTF-8 files, `stats.size` may be larger than `content.length` (safe -- overestimates). But `stats.size` is the byte count while the content stored in `modules[virtualKey]` is a JS string. The aggregate limit semantics are ambiguous: is it bytes on disk or character count in memory? The current code reserves disk bytes but stores character strings, which could undercount actual memory usage for UTF-16 encoded JS strings.
- Fix: Document that the limit is in terms of filesystem bytes (which is the conservative/safe interpretation). The current approach is acceptably safe since `stats.size >= content.length` for UTF-8 files, meaning it overestimates rather than underestimates. Add a brief comment clarifying the unit:
```typescript
// Resource limit: pre-reserve file size in filesystem bytes. This is
// conservative: disk bytes >= UTF-8 character count, so we may reject
// slightly before the true in-memory limit.
aggregateSize += stats.size;
```

**No retry bound on browser-side `init()` in `browser.ts`** - `packages/mds/src/browser.ts:33-51`
**Confidence**: 85%
- Problem: The WASM backend's `init()` in `wasm.ts` correctly bounds retries to `MAX_INIT_RETRIES = 3` (line 31-32). However, the browser entry point `browser.ts` has its own `init()` / `doInit()` at lines 33-51 that wraps `wasmInit()`. When `doInit()` fails, it clears `initPromise` at line 48, allowing unlimited retry attempts. While `wasmInit()` (which delegates to `wasm.ts init()`) does enforce the 3-retry limit internally, the browser-side `doInit()` also calls `createWasmBackend()` at line 45, which could fail independently of init (e.g., if the import fails). Those failures reset `initPromise` with no retry cap, allowing an infinite retry loop if a caller retries in a loop.
- Fix: Add a matching retry bound in `browser.ts`:
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

### MEDIUM

**Unbounded recursion depth in `scan()`** - `packages/mds/src/util/module-scanner.ts:135-201`
**Confidence**: 82%
- Problem: The `scan()` function recurses through import chains with no explicit depth limit. While the `maxModules` limit (256) provides an indirect bound on total nodes visited, it does not bound recursion depth. A pathological import graph (A imports B imports C imports D... in a chain 256 deep) would create a call stack 256 frames deep. In practice, Node.js default stack size (~15,000 frames) makes this unlikely to overflow, but the reliability principle of "every recursive operation must have a fixed upper bound" is violated. The `visited` set prevents cycles, but not deep chains.
- Fix: Add an explicit depth parameter with a reasonable bound:
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

**Module-level `initFailures` state never resets** - `packages/mds/src/backend/wasm.ts:32`
**Confidence**: 80%
- Problem: `initFailures` is module-level mutable state that monotonically increases and never resets. After 3 transient failures (e.g., network blips during WASM download), the WASM backend is permanently bricked for the lifetime of the process. There is no recovery path -- the user must restart the entire Node.js process. For long-running server processes, this means a brief network outage during startup permanently disables the WASM fallback.
- Fix: Consider resetting `initFailures` on success, or providing a manual `reset()` escape hatch:
```typescript
// Inside _init, after successful load:
wasmModule = mod;
initFailures = 0; // Reset on success so transient failures don't accumulate across unrelated init attempts
return;
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`varsOpt` passes through `null` without coercion** - `packages/mds/src/util/options.ts:10-11`
**Confidence**: 82%
- Problem: Test U-C7 confirms `compile('...', { vars: null })` does not throw. `varsOpt` checks `options?.vars !== undefined`, so when `vars` is `null`, it returns `{ vars: null }`. This `null` value is then forwarded to the native addon or WASM module. Whether this causes a crash depends on the Rust side. The test passes, suggesting Rust handles `null`, but the JS layer should defensively normalize this rather than relying on the backend's tolerance.
- Fix: Coerce `null` to `undefined`:
```typescript
export function varsOpt(options?: CompileOptions | FileOptions): { vars: Record<string, unknown> } | undefined {
  return options?.vars != null ? { vars: options.vars } : undefined;
  //                  ^^ loose equality catches both null and undefined
}
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **WASM candidate loading loop has implicit bound** - `packages/mds/src/backend/wasm.ts:78` (Confidence: 65%) -- The `for...of` loop over `candidates` is bounded by the array length (2), but the bound is implicit. A `candidates` constant with a fixed-length tuple type would make the bound explicit and prevent future unbounded growth.

- **`node.ts` top-level `await` makes import non-retryable** - `packages/mds/src/node.ts:19-44` (Confidence: 70%) -- Backend initialization happens at module top-level via `await import(...)`. If the WASM fallback also fails, the module import permanently fails. Any consumer that catches the import error and retries `import('./node.js')` will get a cached rejected module from the module registry. This is a platform limitation, not a bug, but it means backend initialization is effectively a one-shot operation with no retry semantics in Node.js.

- **`Promise.all` in scan propagates only first error** - `packages/mds/src/util/module-scanner.ts:191` (Confidence: 60%) -- When multiple parallel child scans fail, `Promise.all` rejects with only the first error. Other errors are silently swallowed. This is standard JS behavior but could mask the root cause when debugging import resolution failures.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR demonstrates solid reliability awareness -- bounded init retries in `wasm.ts`, resource limits in the module scanner, and TOCTOU mitigations. The main gaps are: (1) the browser entry point lacks the same retry bound that `wasm.ts` has, creating an inconsistency where one path is bounded and the other is not; (2) the recursive `scan()` function bounds total nodes but not recursion depth; and (3) the permanent `initFailures` accumulation could brick long-running processes after transient failures. None of these are critical, but fixing the browser init bound and the recursion depth limit would bring reliability to a strong 9/10.

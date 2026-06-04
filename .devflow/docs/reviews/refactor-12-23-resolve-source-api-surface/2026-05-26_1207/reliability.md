# Reliability Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`reset()` during in-flight `get()` causes dangling promise with stale state** - `packages/bundler-utils/src/lazy-init.ts:34-38`
**Confidence**: 82%
- Problem: If `reset()` is called while a factory promise is still in-flight (the `pending` field is non-null but not yet settled), the pending promise is set to `null` and `resolved`/`instance` are cleared. However, the original in-flight promise's `.then()` callback still holds a closure over `this`. When the original factory eventually resolves, it will set `this.resolved = true` and `this.instance = result` — silently overwriting the post-reset state. Any subsequent `get()` call after that point returns the stale value from the pre-reset factory invocation instead of re-invoking the factory. The same applies to the rejection path: if the original factory rejects after reset, it sets `this.pending = null`, but `pending` is already `null`, so the effect is benign on the rejection side — however the success path is the real hazard.
- Impact: In the webpack-loader, `_resetForTesting()` calls `lazy.reset(); lazy = null;`, which mitigates this since the entire `LazyInit` instance is discarded. But `LazyInit` is now a public export from `@mds/bundler-utils` and intended for general use. Any consumer calling `reset()` while a `get()` is in-flight will get silently corrupted state.
- Fix: Guard the `.then()` callbacks with a generation counter so stale settlements are ignored:
```typescript
export class LazyInit<T> {
  private resolved = false;
  private instance: T | undefined = undefined;
  private pending: Promise<T> | null = null;
  private generation = 0;

  constructor(private readonly factory: () => Promise<T>) {}

  get(): Promise<T> {
    if (this.resolved) return Promise.resolve(this.instance as T);
    if (this.pending === null) {
      const gen = ++this.generation;
      this.pending = this.factory().then(
        (result) => {
          if (this.generation !== gen) return result; // stale — discard
          this.resolved = true;
          this.instance = result;
          return result;
        },
        (err: unknown) => {
          if (this.generation === gen) this.pending = null;
          throw err;
        },
      );
    }
    return this.pending;
  }

  reset(): void {
    this.resolved = false;
    this.instance = undefined;
    this.pending = null;
    this.generation++;
  }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`_setTransformerForTesting` fires-and-forgets the pre-resolve promise** - `packages/webpack-loader/src/index.ts:79`
**Confidence**: 84%
- Problem: `void lazy.get()` fires the factory and discards the promise. The `void` operator is intentional (suppress floating-promise lint), but if the factory (which is `async () => t`, so it resolves immediately on next microtick) somehow threw, the rejection would become an unhandled promise rejection crashing the Node process. More practically, because this is a `void` call, the `LazyInit` is not guaranteed to be in the `resolved` state by the time the next synchronous line executes — it will be resolved after the current microtask, not before. Any test that calls `_setTransformerForTesting(t)` and then immediately calls `getLazy(opts).get()` synchronously will get the right `LazyInit` instance (since `lazy` is set), but `get()` returns `this.pending` (the in-flight promise), not `Promise.resolve(this.instance)`, adding an unnecessary microtask hop.
- Impact: Low in practice since `async () => t` resolves near-instantly, but the pattern is fragile if the factory ever becomes non-trivial.
- Fix: Either await the pre-resolve in the function (make it async), or document that callers must `await _setTransformerForTesting(t)`:
```typescript
export async function _setTransformerForTesting(t: Transformer): Promise<void> {
  if (process.env['NODE_ENV'] !== 'test') {
    throw new Error('_setTransformerForTesting is only allowed when NODE_ENV=test');
  }
  lazy = new LazyInit(async () => t);
  await lazy.get(); // pre-resolve synchronously in test setup
}
```

## Pre-existing Issues (Not Blocking)

(none found in changed files at CRITICAL severity)

## Suggestions (Lower Confidence)

- **No retry bound on `LazyInit.get()` retry-after-failure** - `packages/bundler-utils/src/lazy-init.ts:16-32` (Confidence: 65%) — When the factory rejects, `pending` is cleared so the next `get()` retries. There is no upper bound on how many times a caller can retry via repeated `get()` calls. In practice, callers (bundler plugins) do not loop on `get()`, so this is unlikely to cause unbounded retries. But the class itself provides no safety net — a caller in a retry loop with no max-attempts would retry forever. Consider adding an optional `maxRetries` parameter or at minimum documenting the expectation that callers provide their own retry bounds.

- **`reset()` while concurrent waiters hold references to `this.pending`** - `packages/bundler-utils/src/lazy-init.ts:34-38` (Confidence: 72%) — If multiple callers are awaiting `get()` (they share `this.pending`) and `reset()` is called, those callers still hold the old pending promise. When the old factory resolves, all waiters receive the value, but the `LazyInit` state may have already been re-initialized by a new `get()` call post-reset. The waiters get a value that the `LazyInit` no longer considers its current value. This is a variant of the generation-counter issue above, but specifically about the promise references held by concurrent callers.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The core changes (Rust `&Path` -> `&str` with explicit UTF-8 validation, `LazyInit<T>` extraction) are well-structured and improve reliability by eliminating the `path.display()` lossy conversion. The `LazyInit` class is clean, well-tested, and correctly handles the common cases (dedup, retry-after-failure, void/null factories). The one blocking issue is the `reset()`-during-in-flight race in `LazyInit`, which is a real correctness hazard for a publicly exported utility — a generation counter would seal it. The Rust side of this PR has no reliability concerns: all existing bounds (MAX_IMPORT_DEPTH, MAX_FILE_SIZE, MAX_TRAVERSAL_DEPTH, LIFO invariant check) are preserved, and the new UTF-8 validation at each entry point is proper fail-fast error handling.

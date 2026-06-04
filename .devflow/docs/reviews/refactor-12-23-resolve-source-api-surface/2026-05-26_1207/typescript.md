# TypeScript Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`reset()` does not clear pending promise for concurrent callers** - `packages/bundler-utils/src/lazy-init.ts:34-38`
**Confidence**: 82%
- Problem: If `reset()` is called while a `get()` call is in-flight (pending !== null, resolved === false), the in-flight promise is nulled out but callers who already received a reference to the old `this.pending` promise will still resolve/reject with stale state. The `.then` handler on the old promise will set `this.resolved = true` and `this.instance = result` *after* `reset()` has already cleared them, corrupting the new lifecycle. This is a TOCTOU (time-of-check-time-of-use) race between `reset()` and the pending factory's `.then` callback.
- Fix: Guard the `.then` callback with a generation counter so stale resolutions are discarded:
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
        const gen = this.generation;
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
      this.generation++;
      this.resolved = false;
      this.instance = undefined;
      this.pending = null;
    }
  }
  ```

### MEDIUM

**Type assertion `this.instance as T` bypasses strict null checks** - `packages/bundler-utils/src/lazy-init.ts:17`
**Confidence**: 83%
- Problem: The cast `this.instance as T` is used when `this.resolved` is true. This is functionally safe because the `resolved` boolean flag guards it (and the PR description explicitly calls this out for T=void/T=null correctness). However, it is an `as` cast that bypasses the type checker. When `T` is not `void` or `null`, the field type is `T | undefined`, and the assertion silently suppresses the `undefined` possibility. If a future refactor breaks the `resolved` invariant, the cast will hide the bug.
- Fix: This is acceptable given the explicit design choice (boolean flag for void/null correctness). A slightly more type-safe alternative would use an explicit discriminated union internally, but the added complexity is not warranted for a 39-line class. Consider adding a brief inline comment documenting why the cast is safe:
  ```typescript
  // SAFETY: resolved === true guarantees instance was assigned by the .then() handler
  if (this.resolved) return Promise.resolve(this.instance as T);
  ```

**`_setTransformerForTesting` fire-and-forget `void lazy.get()` has no error handling** - `packages/webpack-loader/src/index.ts:79`
**Confidence**: 80%
- Problem: `void lazy.get()` fires the factory and discards the promise. Since the factory is `async () => t` (a synchronous value wrapped in a promise), it will never reject in practice. However, the `void` discard pattern means any unexpected rejection becomes an unhandled promise rejection. ESLint's `@typescript-eslint/no-floating-promises` would flag this.
- Fix: Since this is a testing-only function and the factory cannot reject, this is low risk. To be fully correct:
  ```typescript
  // Pre-resolve: factory is sync, cannot reject.
  lazy.get().catch(() => {/* unreachable — factory returns a constant */});
  ```
  Or use `await` since the function could be made async. But given the testing-only scope, adding a comment is sufficient:
  ```typescript
  // Pre-resolve so get() returns immediately. Factory is sync — cannot reject.
  void lazy.get();
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing test for `reset()` during in-flight `get()`** - `packages/bundler-utils/__test__/lazy-init.spec.mjs` (Confidence: 75%) -- The test suite covers sequential reset, but not a reset while a factory promise is still pending. This would validate the TOCTOU race described in the HIGH finding above.

- **`LazyInit` could benefit from a `readonly` `isResolved` getter** - `packages/bundler-utils/src/lazy-init.ts` (Confidence: 65%) -- An `isResolved` getter would let consumers (e.g., webpack-loader's `_setTransformerForTesting`) verify the pre-resolve completed without needing the `void` fire-and-forget pattern.

- **Test file imports from `../dist/index.js` (build output)** - `packages/bundler-utils/__test__/lazy-init.spec.mjs:6` (Confidence: 70%) -- Tests import from `../dist/index.js` which requires a build step before tests can run. This is likely an intentional project convention (testing the compiled output), but worth noting that it means test failures after a source change require a rebuild to surface.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The `LazyInit<T>` extraction is a clean, well-typed refactor that correctly replaces hand-rolled singleton patterns in both `transform.ts` and `webpack-loader/index.ts`. The generic is properly constrained, the `resolved` boolean flag is a sound design choice for `T=void`/`T=null` correctness, and strict tsconfig (`strict: true`, `noUncheckedIndexedAccess: true`) is enabled. The test suite covers the key behavioral properties (single-init, dedup, retry-on-rejection, void, null).

The single HIGH finding is the TOCTOU race between `reset()` and an in-flight factory promise. While unlikely to be hit in the current bundler plugin usage (where `reset()` is only called from `_resetForTesting`), it is a correctness issue in the general-purpose `LazyInit<T>` API that is now exported as a public utility. A generation counter is the standard fix and costs two lines of code.

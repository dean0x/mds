# TypeScript Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25
**Cycle**: 2 (incremental after 18 fixes from Cycle 1)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Non-null assertion on `transformer!` relies on implicit promise-ordering guarantee** - `packages/webpack-loader/src/index.ts:37`
**Confidence**: 82%
- Problem: The `ensureTransformer` function uses `transformer!` after awaiting `initPromise`, relying on the fact that the `.then()` callback sets `transformer` before the awaited promise resolves. While this is currently correct per the Promise spec (`.then` callbacks execute before dependent `await` continuations), the non-null assertion (`!`) bypasses TypeScript's type system and creates a fragile coupling between the promise chain shape and the assertion. If the `.then`/`.catch` chain is ever refactored (e.g., to `async/await`), the assertion could become unsound without any type error.
- Fix: Replace the promise chain with async/await in the initializer so that `transformer` is provably non-null after the `await`. Alternatively, add a runtime guard after the await:
```typescript
await initPromise;
if (transformer === null) {
  throw new Error('Invariant violation: transformer not initialized after awaiting initPromise');
}
return transformer;
```
This removes the `!` assertion while keeping the same control flow, and gives a clear runtime error if the invariant is ever violated.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Double type assertion in `isMdsErrorLike`** - `packages/bundler-utils/src/errors.ts:12`
**Confidence**: 85%
- Problem: `(err as unknown as Record<string, unknown>)['code']` uses a double assertion chain (`as unknown as Record<...>`). While this is not unsafe here (it is guarded by the `instanceof Error` check on line 11), a double assertion is a code smell that bypasses the type checker's structural check. Since `err` is already narrowed to `Error`, you can access arbitrary properties more idiomatically.
- Fix: Use a type-safe property check without double assertion:
```typescript
function isMdsErrorLike(err: unknown): err is MdsErrorLike {
  if (!(err instanceof Error)) return false;
  if (!('code' in err) || typeof err.code !== 'string') return false;
  return err.code.startsWith('mds::');
}
```
The `in` operator narrows the type to include the `code` property, avoiding the need for any assertion. This is the idiomatic TypeScript type guard pattern.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`JS_ESCAPE_MAP` could use a `const` assertion for stricter literal typing** - `packages/bundler-utils/src/transform.ts:12` (Confidence: 65%) -- The `Record<string, string>` annotation loses the exact set of keys. Using `as const satisfies Record<string, string>` would let TypeScript track the exact keys while preserving the `Record` constraint, though this is purely informational since the map is only used via the `??` fallback pattern.

- **Plugin interface types (`VitePlugin`, `RollupPlugin`, `PluginContext`) are hand-written rather than imported from bundler type packages** - `packages/vite-plugin/src/index.ts:9`, `packages/rollup-plugin/src/index.ts:10` (Confidence: 62%) -- These structural types work fine and avoid a hard dependency on the bundler type packages, but they could drift from the real types over time. The current approach is a defensible design choice for keeping the dependency tree small; flagging only as something to revisit if the interfaces expand.

- **`escapeForJs` fallback `?? ch` on line 23 may mask missing map entries** - `packages/bundler-utils/src/transform.ts:23` (Confidence: 60%) -- If the regex matches a character that is missing from `JS_ESCAPE_MAP` (due to a regex/map mismatch after future edits), the `?? ch` fallback silently passes the unescaped character through. This is currently safe since the regex and map are in sync, but a defensive approach would be to throw on an unmapped match.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**TypeScript Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Assessment

The TypeScript in this branch is well-structured. Key strengths:

- **No `any` types** -- all source files are free of `any`; `unknown` is used correctly at error boundaries.
- **Strict mode enabled** -- `strict: true` and `noUncheckedIndexedAccess: true` in `tsconfig.base.json`.
- **Clean compilation** -- all four packages compile with zero errors and zero warnings under `--noEmit`.
- **Structural typing for `MdsApi`** is well-documented with JSDoc (added in Cycle 1 resolution) explaining why the interface is narrower than the real `@mds/mds` API surface. This is a sound pattern.
- **Type-only imports** used correctly (`import type`) throughout.
- **Discriminated error handling** in `formatMdsError` properly narrows `unknown` through an `instanceof` check and a type guard.
- **Poisoned-promise fix** in `createMdsTransformer` and `ensureTransformer` correctly resets `initPromise` on rejection, preventing permanently stuck state.

The one blocking issue (non-null assertion in `ensureTransformer`) is a minor type-safety gap that can be resolved with a runtime guard. The should-fix item (double assertion in `isMdsErrorLike`) is an idiomatic improvement. Both are low-effort fixes.

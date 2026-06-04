# TypeScript Review Report

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25

## Issues in Your Changes (BLOCKING)

No blocking issues found.

## Issues in Code You Touched (Should Fix)

No should-fix issues found.

## Pre-existing Issues (Not Blocking)

No pre-existing issues found.

## Suggestions (Lower Confidence)

- **`safeJsonForJs` parameter type is wider than necessary** - `packages/bundler-utils/src/transform.ts:44` (Confidence: 65%) -- The function accepts `unknown` but `JSON.stringify(undefined)` returns `undefined` at runtime (not `string`), which would cause `.replace()` to throw. The only call site passes an object literal so this is safe in practice; narrowing the parameter to `Record<string, unknown>` would make the contract self-documenting and prevent future misuse.

- **`_setTransformerForTesting` missing NODE_ENV guard in vite-plugin and rollup-plugin** - `packages/vite-plugin/src/index.ts:46`, `packages/rollup-plugin/src/index.ts:34` (Confidence: 60%) -- webpack-loader guards `_setTransformerForTesting` and `_resetForTesting` with `NODE_ENV !== 'test'` checks, but vite-plugin and rollup-plugin do not. This is not a type safety issue but an API consistency gap. The `_` prefix and JSDoc are clear enough markers for test-only use.

- **`handleHotUpdate` only checks `.mds` extension, not `.md` with frontmatter** - `packages/vite-plugin/src/index.ts:102` (Confidence: 65%) -- `handleHotUpdate` uses `isMdsExtension(clean)` which only matches `.mds` files. Editing a `.md` file with `type: mds` frontmatter will not trigger HMR full-reload. This is a behavioral gap rather than a type issue, but the TypeScript types would support calling `shouldTransform` here (with async handling).

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**TypeScript Score**: 9/10
**Recommendation**: APPROVED

### Assessment

The TypeScript code across all four bundler packages is well-typed and follows best practices:

- **No `any` types** -- All parameters and return types are explicit. `unknown` is used correctly for error handling (`err: unknown`) and the `safeJsonForJs` parameter.
- **No type assertions** -- The prior double assertion in `isMdsErrorLike` (`err as unknown as Record<string, unknown>`) has been replaced with the `in` operator narrowing pattern (`'code' in err`), which is the idiomatic TypeScript approach.
- **No non-null assertions** -- The webpack-loader replaced `transformer!` with a proper invariant check (`if (transformer === null) throw`), which is safer.
- **Proper `import type`** -- Type-only imports use `import type` throughout, and the barrel `index.ts` correctly uses `export type` for interface re-exports.
- **Structural typing** -- The hand-rolled `VitePlugin`, `RollupPlugin`, and `LoaderContext` interfaces are well-documented with rationale for avoiding the heavy bundler type imports. The structural subset approach is sound.
- **Strict tsconfig** -- `strict: true` and `noUncheckedIndexedAccess: true` are enabled, and all packages compile cleanly with zero errors.
- **Explicit return types** -- All exported and private functions have explicit return type annotations.
- **`noUncheckedIndexedAccess` handled correctly** -- The `Record<string, string>` lookups in `JS_ESCAPE_MAP[ch]` and `SAFE_JSON_MAP[ch]` use `?? ch` fallbacks to handle the `string | undefined` return type.

Prior resolution context: The double type assertion fix in `isMdsErrorLike` (Cycle 2) is confirmed resolved -- the `in` operator narrowing pattern is clean and correct.

# Complexity Review Report

**Branch**: feat-bundler-plugins -> main
**Date**: 2026-05-25T17:35

## Issues in Your Changes (BLOCKING)

### HIGH

**`switch(true)` pattern in `escapeForJs` adds unnecessary cognitive load** - `packages/bundler-utils/src/transform.ts:11-21`
**Confidence**: 85%
- Problem: The `escapeForJs` function uses `switch (true)` with comparison expressions in each `case` branch. This is a non-standard pattern that simulates an if-else chain using switch syntax. While functionally correct, it forces readers to mentally re-map the semantics (switch on a value vs. switch on conditions) and increases cyclomatic complexity to 8 paths within a character-level loop.
- Fix: Replace with a straightforward `if`/`else if` chain or, better, use a replacement map with a regex for clarity and lower branching:
```typescript
const JS_ESCAPE: Record<string, string> = {
  '\\': '\\\\',
  '"': '\\"',
  '\n': '\\n',
  '\r': '\\r',
  ' ': '\\u2028',
  ' ': '\\u2029',
};
const JS_ESCAPE_RE = /[\\"\n\r  ]/g;

function escapeForJs(str: string): string {
  return str.replace(JS_ESCAPE_RE, (ch) => JS_ESCAPE[ch]!);
}
```
This reduces the function from 14 lines / 8 branches to 2 lines / 0 explicit branches, and is the standard idiom for character-class escaping in JS.

### MEDIUM

**Module-level mutable singletons in webpack-loader** - `packages/webpack-loader/src/index.ts:12-13`
**Confidence**: 82%
- Problem: `transformer` and `initPromise` are module-level `let` variables that form a mutable singleton with implicit lifecycle. The `_resetForTesting` function at line 50 confirms this creates enough complexity to require a test-only escape hatch. The ensureTransformer function (line 15) manages the initialization race with a captured promise and a non-null assertion on line 27 that bypasses the type system.
- Fix: Consider encapsulating the singleton in a factory or class so the lifecycle is explicit and testable without the `_resetForTesting` backdoor. However, given that webpack loaders are inherently singleton-per-process and the current code is only 55 lines, this is a moderate concern rather than a blocker.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Duplicated plugin structure across rollup/vite/webpack** - `packages/rollup-plugin/src/index.ts`, `packages/vite-plugin/src/index.ts`, `packages/webpack-loader/src/index.ts` (Confidence: 65%) -- The three plugin files share nearly identical transform-then-handle-error patterns (~15 lines each). This is borderline: the bundler-utils package already extracts the shared core, and each plugin adapts to its bundler's specific API (this.error vs throw vs callback). The current duplication level is acceptable for 3 plugins, but would become a maintenance concern at 5+.

- **`shouldTransform` mixed return type** - `packages/bundler-utils/src/frontmatter.ts:31` (Confidence: 62%) -- Returning `boolean | Promise<boolean>` from `shouldTransform` forces callers to handle both sync and async cases. All current callers already `await` the result (which works for both), so the practical impact is low. However, the mixed return type makes the function signature harder to reason about at a glance.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase is well-structured overall. The shared `bundler-utils` package correctly extracts common logic, keeping each plugin thin (55-76 lines). Functions are short, nesting is shallow (max 3 levels), and parameter counts are reasonable (0-2 per function). File lengths are all well under 100 lines for source files. The only notable complexity issue is the `switch(true)` pattern in `escapeForJs`, which has a clean fix. The mutable singleton in the webpack loader is a minor concern given the inherent constraints of the webpack loader API.

# Documentation Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22T23:50

## Issues in Your Changes (BLOCKING)

### HIGH

**README error span property name mismatch (`col` vs `column`)** - `packages/mds/README.md:83`
**Confidence**: 95%
- Problem: The README error handling example shows `err.span` as `{ offset, length, line, col }` but the actual `MdsErrorSpan` interface (types.ts:36) uses `column`, not `col`. Users copying this example will expect `err.span.col` which does not exist on the type.
- Fix:
```diff
-    console.error(err.span);    // optional { offset, length, line, col }
+    console.error(err.span);    // optional { offset, length, line, column }
```

**Broken `test:parity` script after file rename** - `packages/mds/package.json:27`
**Confidence**: 98%
- Problem: The `test:parity` npm script references `__test__/parity.spec.mjs` but this file was renamed to `__test__/native-backend.spec.mjs` in this branch. Running `npm run test:parity` will fail with a file-not-found error. Script documentation (via `npm run`) is a form of developer-facing docs.
- Fix:
```diff
-    "test:parity": "node --test __test__/parity.spec.mjs",
+    "test:native": "node --test __test__/native-backend.spec.mjs",
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`MdsErrorSpan.line` and `MdsErrorSpan.column` lack JSDoc** - `packages/mds/src/types.ts:35-36`
**Confidence**: 85%
- Problem: All other properties on `MdsErrorSpan` have JSDoc (`offset`, `length`) but `line` and `column` do not. Both are optional, and users need to know they are 1-based (or 0-based) line/column numbers.
- Fix:
```typescript
  /** Byte length of the error span. */
  length: number;
  /** 1-based line number of the error, if available. */
  line?: number;
  /** 1-based column number of the error, if available. */
  column?: number;
```

**`BackendType` exported but undocumented in README** - `packages/mds/src/node.ts:93`, `packages/mds/src/browser.ts:20`
**Confidence**: 80%
- Problem: The `BackendType` type is exported from both entry points alongside the other public types, but the README API section only lists functions -- not re-exported types. Users looking at the API table won't know `BackendType`, `CompileResult`, `CheckResult`, etc. are available as named type imports.
- Fix: Add a "Types" subsection to the README API section listing the re-exported types, or at minimum mention `BackendType` since `getBackend()` returns it.

## Pre-existing Issues (Not Blocking)

No pre-existing documentation issues found.

## Suggestions (Lower Confidence)

- **README omits `MDS_BACKEND` invalid-value behavior** - `packages/mds/README.md:59-63` (Confidence: 65%) -- The code now warns and ignores unrecognized `MDS_BACKEND` values (node.ts:13-15) but the README table only shows `native`, `wasm`, and `(unset)`. Adding a row for "other" values would prevent user confusion.

- **CHANGELOG could mention `isMdsError` tightening** - `CHANGELOG.md:15` (Confidence: 60%) -- The `isMdsError` type guard was tightened to require `code.startsWith('mds::')` (types.ts:75). Since this is the initial package release, the final behavior is what matters, but if any pre-existing code depended on the looser check, this could be a surprise.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The documentation is well-structured overall: the README covers installation, Node.js usage, browser usage, backend selection, error handling, and a full API table. JSDoc coverage on types.ts and all public functions in node.ts is thorough. The CHANGELOG accurately captures the scope of the change.

The two blocking issues are both straightforward fixes: (1) the `col` vs `column` property name mismatch in the README error handling example will mislead users, and (2) the broken `test:parity` script reference after the file rename will cause CI/developer confusion.

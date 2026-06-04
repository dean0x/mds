# Regression Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**`MAX_NESTING_DEPTH` reduced from 256 to 64 -- intentional breaking change not gated by semver bump** - `crates/mds-core/src/parser.rs:17`
**Confidence**: 85%
- Problem: The parser's `MAX_NESTING_DEPTH` was reduced from 256 to 64. Any template with nesting between 65 and 256 levels that previously compiled will now fail at parse time. This is a behavioral regression for any consumer of the `mds-core` crate. The CHANGELOG's `[Unreleased]` section does not document this as a breaking change. (applies ADR-002 -- PR content must address the intent of all stated changes; the nesting limit reduction is not mentioned in the PR description or CHANGELOG's breaking-changes section.)
- Fix: Add a `### Changed` entry in `CHANGELOG.md` documenting the `MAX_NESTING_DEPTH` reduction from 256 to 64. Given this is a pre-release project with zero users (per MEMORY.md), this is informational rather than blocking, but documenting breaking changes is good practice for traceability.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`buildModulesMap` return value semantics changed -- `entryFilename` is now a relative path from project root, not a basename** - `packages/mds/src/util/module-scanner.ts:176-179`
**Confidence**: 82%
- Problem: Previously, `entryFilename` was `basename(absoluteEntry)` (e.g., `"entry.mds"`). Now it is `relative(projectRoot, absoluteEntry)` (e.g., `"subdir/entry.mds"`). The `BuildModulesMapResult` interface's `entryFilename` field has changed semantics without a type change, making this an invisible contract change. The primary consumer in `node.ts:72-80` (`prepareFileArgs`) uses `entryFilename` as both a map lookup key and a WASM virtual filename. This works correctly because the module map keys are now also project-root-relative (they are computed via `normalizeVirtualKey` against the new longer entry key). The Rust VirtualFs resolver correctly handles multi-segment virtual keys. However, any external consumer of `buildModulesMap` that assumed `entryFilename` was a basename would break.
- Fix: The JSDoc on `BuildModulesMapResult.entryFilename` at line 91 should explicitly document that the key is now project-root-relative, not just a basename. This prevents future confusion:
  ```typescript
  export interface BuildModulesMapResult {
    /**
     * Virtual key for the entry file, relative to the discovered project root
     * (e.g., "subdir/entry.mds" rather than just "entry.mds"). All module keys
     * in `modules` use the same project-root-relative coordinate system.
     */
    entryFilename: string;
  ```

**Tests updated to use `.endsWith()` assertions instead of exact equality -- masks potential key format bugs** - `packages/mds/__test__/scanner.spec.mjs:103-113`
**Confidence**: 80%
- Problem: The `buildModulesMap` tests for U-SM1 and U-SM2 were changed from `assert.equal(entryFilename, 'entry.mds')` to `assert.ok(entryFilename.endsWith('imports/entry.mds'))`. While this accommodates the new relative-path semantics, the `.endsWith()` assertion is weaker -- it would also pass if the key contained unexpected parent-directory prefixes or absolute path leakage. The test should validate that the key is a clean relative path without leading slashes or absolute-path prefixes.
- Fix: Add a structural assertion alongside `.endsWith()`:
  ```javascript
  assert.ok(!entryFilename.startsWith('/'), 'entry key must be relative, not absolute');
  assert.ok(entryFilename.endsWith('imports/entry.mds'), `entry key should end with imports/entry.mds: ${entryFilename}`);
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`_esmImport` via `new Function` is fragile across bundler-of-bundlers scenarios** - `packages/webpack-loader/src/index.ts:17-20` (Confidence: 65%) -- The `new Function('id', 'return import(id)')` workaround to preserve ESM `import()` in CJS output is well-documented and appropriate for its intended use case (Webpack loaders in Node.js). However, if the webpack-loader itself is ever processed by another bundler (e.g., esbuild bundling a monorepo), the string-based `import()` may be rewritten or break. The CSP caveat is already documented. Consider adding a runtime check that verifies the dynamic import actually returns a thenable, as the current shape validation on line 48 already partially does.

- **`findProjectRoot` uses synchronous `existsSync` in what could be a hot path** - `packages/mds/src/util/module-scanner.ts:46` (Confidence: 70%) -- While the cache mitigates repeated calls, the first invocation performs up to `MAX_TRAVERSAL_DEPTH x |markers|` synchronous I/O calls. In Webpack builds with many entry points on deep filesystems, the first loader invocation could block the event loop. The comment on line 22-24 already acknowledges this. This is acceptable for the current use case but worth noting.

- **`IfBlock.condition` type changed from `Vec<String>` to `Condition` enum -- consumers of the AST are affected** - `crates/mds-core/src/ast.rs:141` (Confidence: 75%) -- The `IfBlock.condition` field type changed from `Vec<String>` (a plain dot-path) to `Condition` (an enum with `Truthy`, `Not`, `Eq`, `NotEq` variants). All internal consumers (evaluator, validator, parser tests) were updated. For any external consumer of the `mds-core` crate's AST types, this is a breaking API change. As a pre-release project with zero external users this is not blocking, but it reinforces the need for a breaking-change CHANGELOG entry.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions

1. Document the `MAX_NESTING_DEPTH` reduction (256 to 64) in the CHANGELOG as a breaking change.
2. Update the `BuildModulesMapResult.entryFilename` JSDoc to document the project-root-relative semantics.
3. Strengthen test assertions for `entryFilename` to verify relative-path format (no leading `/`).

### Regression Analysis Summary

The branch introduces several behavioral changes that were analyzed for regression risk:

1. **`@if` condition type change (`Vec<String>` to `Condition` enum)**: All internal consumers updated (evaluator, validator, tests). The old test that asserted negation was rejected (`if_negation_error_message_is_actionable`) was correctly replaced with a test asserting negation is accepted (`if_negation_supported`). No regression -- intentional feature addition.

2. **`@elseif` support**: New AST variant (`elseif_branches`), parser support, evaluator support, and validator support. Existing `@if`/`@else` behavior preserved -- tests for simple if/else paths still pass. Short-circuit evaluation correctly documented and tested.

3. **Equality/inequality operators (`==`/`!=`)**: New `Condition::Eq`/`Condition::NotEq` variants with strict typing (no coercion). NaN/Infinity rejection at parse time. No existing behavior changed -- these are additive.

4. **`buildModulesMap` entry key semantics**: Changed from basename to project-root-relative. All internal consumers (`node.ts`) correctly adapted. Tests updated but use weaker assertions.

5. **`MAX_NESTING_DEPTH` 256 to 64**: Intentional tightening to prevent stack overflow in debug builds. Breaking for templates with 65-256 nesting levels. Pre-release project mitigates real-world impact.

6. **Webpack loader CJS build**: `new Function` workaround for ESM import preservation is well-documented with appropriate caveats. Module shape validation added. No regression to ESM path. (applies ADR-001 -- pre-merge quality gate satisfied by comprehensive CJS compatibility tests.)

7. **No deleted files, no removed exports, no removed CLI options**: Zero lost-functionality signals detected.

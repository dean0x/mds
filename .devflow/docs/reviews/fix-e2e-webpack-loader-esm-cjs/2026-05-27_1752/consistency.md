# Consistency Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Inconsistent CJS test patterns: repeated `resolve(__dirname, '../dist-cjs/index.js')` vs. single require** - `packages/bundler-utils/__test__/cjs-compat.spec.mjs:19-62`, `packages/webpack-loader/__test__/cjs-compat.spec.mjs:18-56`
**Confidence**: 85%
- Problem: In the bundler-utils CJS test file, each test individually calls `require(resolve(__dirname, '../dist-cjs/index.js'))` (7 separate times). The webpack-loader CJS test does the same (5 times). Meanwhile, the existing test files in the project (e.g., `scanner.spec.mjs`) use a top-level import and share it across tests. Repeating the full path resolution in every test is noisy and inconsistent with the existing test organization pattern.
- Fix: Extract the require to a shared constant at the top of each describe block:
```javascript
describe('bundler-utils CJS build', () => {
  const cjsPath = resolve(__dirname, '../dist-cjs/index.js');
  // Use cjsPath in each test
  test('loads without error via require()', () => {
    const cjsBuild = require(cjsPath);
    assert.ok(cjsBuild, 'CJS build should load successfully');
  });
  // ...
});
```

**Inconsistent `exports` map field ordering across packages** - `packages/bundler-utils/package.json:11-17`, `packages/webpack-loader/package.json:11-17`
**Confidence**: 82%
- Problem: The new `exports` maps in bundler-utils and webpack-loader include `"require"` and `"default"` fields, while the existing vite-plugin and rollup-plugin exports maps only have `"types"` and `"import"`. This is intentional (CJS is only needed for webpack-loader and bundler-utils), but the two CJS-enabled packages differ in a subtle way from the Node.js `exports` map convention: the `"default"` entry should typically come last as the final fallback. In both modified packages, `"default"` is already last, which is correct. However, neither package provides a `"require"` condition with a separate types entry (e.g., `"types@require": "./dist-cjs/index.d.ts"`), meaning CJS consumers get no TypeScript type support.
- Fix: Since `tsconfig.cjs.json` sets `declaration: false`, this is intentional. Add a brief comment in the package.json or README noting that CJS consumers do not get type declarations (they should use the ESM import path for TypeScript). Alternatively, to maintain full consistency:
```json
"exports": {
  ".": {
    "types": "./dist/index.d.ts",
    "import": "./dist/index.js",
    "require": "./dist-cjs/index.js",
    "default": "./dist-cjs/index.js"
  }
}
```
This is already the case, so the types will resolve for both ESM and CJS via the shared `"types"` key at the top. No code change needed -- just noting the pattern is acceptable.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`findProjectRoot` uses synchronous `existsSync` in an otherwise fully async module** - `packages/mds/src/util/module-scanner.ts:37-60`
**Confidence**: 84%
- Problem: The entire `module-scanner.ts` module follows an async pattern -- `openNoFollow` is async, `buildModulesMap` is async, `scan` is async. The new `findProjectRoot` function uses `existsSync` (synchronous I/O) which blocks the event loop. The function is called once per `buildModulesMap` invocation, and results are cached, so the real-world impact is small. However, this breaks the existing async-everywhere pattern in this module. The caching mitigates event loop blocking after the first call.
- Fix: This is a pragmatic trade-off (documented in comments). The cache ensures it only blocks once per unique directory. Acceptable as-is given the single-call-per-build usage, but the async pattern deviation should be noted. If the module ever moves to a worker or needs to handle concurrent builds, this should be converted to async.

**`_esmImport` naming convention diverges from other module-level constants** - `packages/webpack-loader/src/index.ts:17-20`
**Confidence**: 80%
- Problem: The module uses an underscore-prefixed name `_esmImport` for a module-private constant, while other module-level bindings in the same file (like `lazy`, `LoaderContext`, `Transformer`) use no underscore prefix. The leading underscore in `_esmImport` suggests "test-only" or "internal" in this codebase (e.g., `_resetForTesting`, `_setTransformerForTesting`), but this is a regular runtime binding, not a test helper.
- Fix: Rename to `esmImport` (no underscore) to match the naming pattern of other module-private constants. The underscore prefix should be reserved for test-only exports:
```typescript
const esmImport: (id: string) => Promise<unknown> = new Function(
  'id',
  'return import(id)',
) as (id: string) => Promise<unknown>;
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`Value` enum and `CondValue` enum variant naming is now consistent** - `crates/mds-core/src/ast.rs`, `crates/mds-core/src/value.rs`
**Confidence**: 95%
- Observation: The prior resolution cycle (Cycle 2) correctly renamed `CondValue::Bool` to `CondValue::Boolean` to match `Value::Boolean`. This alignment is confirmed -- both enums now use `String`, `Number`, `Boolean`, `Null` consistently. No issue here; noting as a positive consistency outcome.

## Suggestions (Lower Confidence)

- **Test numbering gap: U-SM8 before U-SM7** - `packages/mds/__test__/scanner.spec.mjs:159,175` (Confidence: 72%) -- The new cross-directory test is numbered U-SM8 but appears before the existing U-SM7 (symlink test). The insertion order does not match the numbering sequence, which could confuse future readers.

- **`parse_body` signature change: positional arrays vs. options struct** - `crates/mds-core/src/parser.rs:117-121` (Confidence: 65%) -- The `parse_body` method now takes two `&[&str]` parameters (`exact_terminators`, `prefix_terminators`) which are positionally ambiguous. Other Rust parsers often use builder patterns or typed enums for terminator kinds. However, the parameters are well-named and the call sites are clear enough.

- **Inline build script in package.json** - `packages/bundler-utils/package.json:28`, `packages/webpack-loader/package.json:24` (Confidence: 68%) -- The build scripts contain a long inline `node -e` command for writing `dist-cjs/package.json`. This is functional but brittle. A shared build script or postbuild step would be more consistent with the simpler `"build": "tsc -p tsconfig.json"` pattern in vite-plugin and rollup-plugin.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The branch demonstrates strong consistency overall. The new Rust code (AST, parser, evaluator, validator) follows existing patterns faithfully: `Result<T, MdsError>` return types, `MdsError::syntax()` error construction, `#[derive(Debug, Clone)]` on new types, doc comments on all public items, and consistent enum variant naming (confirmed by the prior Cycle 2 `Bool` -> `Boolean` resolution -- applies ADR-001 merge gate quality). The `CondValue` enum mirrors `Value` exactly. New parser functions (`parse_condition`, `parse_dot_path`, `parse_cond_value`, `find_unquoted_operator`, `collect_elseif_branches`) follow the existing `parse_*` naming convention. Test naming in Rust follows the existing `snake_case_descriptive` pattern. The TypeScript changes are also largely consistent, with minor deviations noted above. The `exports` map additions for CJS follow Node.js best practices. The spec.md updates correctly extend the grammar summary and rules sections to document the new features.

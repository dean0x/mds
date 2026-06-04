# Architecture Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**PR**: #34

## Issues in Your Changes (BLOCKING)

### HIGH

**Stale comment: MAX_ELSEIF_BRANCHES claims to match MAX_NESTING_DEPTH but values diverge (256 vs 64)** - `crates/mds-core/src/ast.rs:9-10`
**Confidence**: 95%
- Problem: The comment on `MAX_ELSEIF_BRANCHES` states "Matches MAX_NESTING_DEPTH to prevent pathological chains" but `MAX_ELSEIF_BRANCHES = 256` while `MAX_NESTING_DEPTH` was lowered from 256 to 64 in this PR. The comment is now factually wrong, and the constant itself is unreasonably high — 256 `@elseif` branches on a single `@if` is far beyond any realistic use case and contradicts the stated motivation of preventing pathological inputs.
- Fix: Either lower `MAX_ELSEIF_BRANCHES` to match `MAX_NESTING_DEPTH` (64), or update the comment to explain why the two limits intentionally differ. Given the security rationale for lowering `MAX_NESTING_DEPTH`, 64 is the more defensible choice:
```rust
/// Maximum number of @elseif branches on a single @if block.
/// 64 branches is generous for real templates while preventing pathological chains.
pub const MAX_ELSEIF_BRANCHES: usize = 64;
```

### MEDIUM

**`new Function('id', 'return import(id)')` bypasses static analysis and CSP** - `packages/webpack-loader/src/index.ts:10-13`
**Confidence**: 85%
- Problem: The `new Function()` pattern is equivalent to `eval()` for Content Security Policy purposes — it creates code from a string at runtime. While the comment correctly explains *why* this is needed (TypeScript CJS compilation rewrites `import()` to `require()`), this is an infrastructure-level workaround that deserves a more visible architectural note. In environments with CSP (`script-src` without `'unsafe-eval'`), this will fail silently. The pattern also blocks tree-shaking and prevents bundlers from statically resolving the `@mds/mds` dependency.
- Fix: The workaround is pragmatically necessary for CJS compatibility. Add a note about CSP implications and consider externalizing `@mds/mds` in webpack configs:
```typescript
// WORKAROUND: new Function() is needed to preserve native import() in CJS output.
// This is functionally equivalent to eval() for CSP purposes — environments with
// a strict Content-Security-Policy that blocks 'unsafe-eval' will reject this.
// Webpack loaders run in Node.js (no CSP by default), so this is safe in practice.
```

**Shell-based build parallelization with bare `&` is fragile across platforms** - `packages/webpack-loader/package.json:23`, `packages/bundler-utils/package.json:27`
**Confidence**: 82%
- Problem: The build scripts use shell backgrounding (`tsc -p tsconfig.json & tsc -p tsconfig.cjs.json & wait && ...`) for parallel compilation. The `&` and `wait` operators are POSIX shell constructs that do not work on Windows `cmd.exe`. If a contributor runs `npm run build` on Windows without a POSIX-compatible shell, the build silently breaks. Additionally, if either `tsc` invocation fails, `wait` returns the exit code of the *last* background job, not the first failure — so a build failure in the ESM compilation may be silently swallowed while the CJS build succeeds.
- Fix: Use `npm-run-all`, `concurrently`, or separate named build scripts to avoid platform-dependent shell features:
```json
{
  "scripts": {
    "build:esm": "tsc -p tsconfig.json",
    "build:cjs": "tsc -p tsconfig.cjs.json && node -e \"require('fs').writeFileSync('dist-cjs/package.json', '{\\\"type\\\":\\\"commonjs\\\"}\\n')\"",
    "build": "npm run build:esm && npm run build:cjs"
  }
}
```
If parallelism is important, `concurrently` or GNU `parallel` is more robust than bare `&`.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Module-level mutable singleton (`let lazy`) with no multi-instance story** - `packages/webpack-loader/src/index.ts:35`
**Confidence**: 80%
- Problem: The existing `lazy` singleton captures options from the first `getLazy()` call. The comment on line 29-34 documents this limitation, but the new CJS build makes this module loadable by both `require()` and `import()` within the same Node process. If Webpack resolves the ESM path for one loader instance and the CJS path for another, there would be two separate module evaluations with two independent `lazy` singletons — options from the first call to one would not affect the other. This is a pre-existing design choice that becomes slightly more visible with dual ESM/CJS support.
- Fix: The comment is adequate for now. Consider documenting in the package README that mixing ESM and CJS imports of this loader in the same webpack config is not supported.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`parse_body` positional parameters (`exact_terminators`, `prefix_terminators`) grow signature complexity** - `crates/mds-core/src/parser.rs:117-121`
**Confidence**: 80%
- Problem: The `parse_body` method now takes two separate slice parameters that together define termination conditions. This is a step toward a more general terminator pattern, but the two-parameter design is a shallow abstraction — callers must understand the difference between exact and prefix matching. If additional termination conditions are needed later (e.g., regex-based, or suffix-based), the parameter list will grow further. This was deferred from cycle 1 and remains informational.
- Fix: Consider a `TerminatorSet` struct that encapsulates both matching strategies in a single, self-documenting parameter:
```rust
struct TerminatorSet<'a> {
    exact: &'a [&'a str],
    prefix: &'a [&'a str],
}
```

## Suggestions (Lower Confidence)

- **`findProjectRoot` using synchronous `existsSync` in an otherwise async module** - `packages/mds/src/util/module-scanner.ts:29` (Confidence: 65%) — The module-scanner is an async-first module (uses `fs/promises` throughout), but the new `findProjectRoot` uses `existsSync` for marker detection. This mixes sync and async I/O patterns within the same module. In practice this is a cold-path call (once per `buildModulesMap` invocation), so the performance impact is negligible, but it deviates from the module's established pattern.

- **`CondValue::Number(f64)` equality uses IEEE 754 `==` directly** - `crates/mds-core/src/evaluator.rs:339` (Confidence: 70%) — The `values_equal` function compares `f64` values with `==`, which means `NaN == NaN` is `false` (correct per IEEE 754, and documented in the comment on line 335). However, there is no handling for `-0.0 == 0.0` (which is `true` in IEEE 754 but may surprise template authors comparing values like `@if temp == -0:`). This is a minor edge case unlikely to appear in real templates.

- **Dual tsconfig pattern may drift** - `packages/webpack-loader/tsconfig.cjs.json`, `packages/bundler-utils/tsconfig.cjs.json` (Confidence: 62%) — Both packages now maintain two tsconfig files (ESM and CJS) that share most settings via `extends` but independently specify `module`, `outDir`, `paths`, etc. If `tsconfig.base.json` changes in ways that conflict with CJS requirements, the CJS builds could silently produce incorrect output. Consider a shared comment or CI check that verifies both builds produce loadable output.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The architecture of this PR is generally sound. The Rust-side changes follow clean separation of concerns: `Condition` enum variants are well-factored, `parse_condition` and `find_unquoted_operator` are properly extracted, and the evaluator/validator consume the new AST nodes through the established `Condition::path()`/`Condition::root()` interface without introducing new coupling. The CJS compatibility layer for webpack-loader is a pragmatic solution to a real ecosystem constraint. The `findProjectRoot` addition to module-scanner correctly mirrors the Rust `NativeFs::find_project_root` behavior, maintaining cross-language consistency (applies ADR-001 — PR content addresses the stated compatibility goal).

The blocking HIGH issue (stale comment + excessively permissive limit on `MAX_ELSEIF_BRANCHES`) should be addressed before merge to avoid a misleading invariant comment in the codebase.

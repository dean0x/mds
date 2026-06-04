# Resolution Summary

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Review**: .devflow/docs/reviews/feat-17-universal-js-package/2026-05-22_2350
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 21 |
| Fixed | 18 |
| False Positive | 2 |
| Deferred | 1 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Stale test:parity npm script references renamed file | packages/mds/package.json:28 | 43920cd |
| Dead variable `script` references nonexistent file | packages/mds/__test__/backend.spec.mjs:46 | 43920cd |
| Unused imports SIMPLE_MDS and FIXTURES | packages/mds/__test__/compile.spec.mjs:7 | 43920cd |
| Misleading test name U-C4 (no @include in source) | packages/mds/__test__/compile.spec.mjs:32 | 43920cd |
| isMdsError boundary test for non-mds:: code | packages/mds/__test__/error.spec.mjs:49 | 43920cd |
| normalizeVirtualKey complexity — extract normalizeRootKey | packages/mds/src/util/module-scanner.ts:26 | 8a8cff6 |
| Sequential lstat/realpath — parallelize with Promise.all | packages/mds/src/util/module-scanner.ts:151 | 8a8cff6 |
| Unbounded recursion depth — add MAX_IMPORT_DEPTH=64 | packages/mds/src/util/module-scanner.ts:135 | 8a8cff6 |
| Duplicated init logic — browser.ts delegates to wasm.ts | packages/mds/src/browser.ts:24 | 0185e21 |
| Browser init no retry bound — resolved by delegation | packages/mds/src/browser.ts:33 | 0185e21 |
| varsOpt null passthrough — use loose equality | packages/mds/src/util/options.ts:11 | 0185e21 |
| Object spread on every compile/check — frozen default | packages/mds/src/backend/wasm.ts:120 | 0185e21 |
| Missing JSDoc on browser.ts exports | packages/mds/src/browser.ts:60 | 0185e21 |
| isMdsError behavioral change undocumented — CHANGELOG | CHANGELOG.md | 43920cd |
| MdsErrorSpan.line/column missing JSDoc | packages/mds/src/types.ts:35 | 43920cd |
| README col vs column property name mismatch | packages/mds/README.md:83 | 43920cd |
| Stale nested lockfile — added to .gitignore | .gitignore | add919c |
| Browser entry point missing test coverage — added 10 tests | packages/mds/__test__/browser.spec.mjs | add919c |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| node.ts module-level side effects | packages/mds/src/node.ts:10 | Top-level await is the mechanism enabling the synchronous public API (compile returns CompileResult, not Promise). This is an intentional design decision, not a bug. Lazy init would require async API, breaking the core contract. |
| mds-napi file: protocol dependency | packages/mds/package.json:31 | Already resolved in a prior commit — HEAD already uses optionalDependencies. No change needed. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| WASM init retry circuit breaker untested | packages/mds/src/backend/wasm.ts:31 | initFailures is module-level mutable state that cannot be reset between tests without subprocess spawning or refactoring to class-based injectable state. Requires architectural redesign of the init subsystem for testability. |

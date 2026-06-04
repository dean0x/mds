# Resolution Summary

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Review**: .devflow/docs/reviews/feat-17-universal-js-package/2026-05-23_0720
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 21 |
| Fixed | 19 |
| False Positive | 1 |
| Deferred | 1 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Shallow-frozen DEFAULT_COMPILE_OPTS.modules — deep-freeze nested object | packages/mds/src/backend/wasm.ts:106 | 6283f33 |
| Type assertion as Record — typed const DEFAULT_MODULES | packages/mds/src/backend/wasm.ts:106 | 6283f33 |
| Duplicated ternary in compile/check — extract compileOpts() helper | packages/mds/src/backend/wasm.ts:117,123 | 6283f33 |
| Module-scoped singletons without reset — add _resetForTesting() | packages/mds/src/backend/wasm.ts:25 | 6283f33 |
| _init() try/catch in for-loop — extract tryLoadCandidate() | packages/mds/src/backend/wasm.ts:59 | 6283f33 |
| WASM candidate loop bound annotation | packages/mds/src/backend/wasm.ts:75 | 6283f33 |
| scan() 80 lines — extract statAndValidateModule() helper | packages/mds/src/util/module-scanner.ts:133 | 08bbfbb |
| Duplicate init-state machines — remove browser.ts retry reset | packages/mds/src/browser.ts:37 | e268fcc |
| Browser init() retry bound — resolved by removing .catch reset | packages/mds/src/browser.ts:37 | e268fcc |
| JSDoc style inconsistency — align browser.ts to single-line format | packages/mds/src/browser.ts:60 | e268fcc |
| varsOpt JSDoc mismatch — document null-coalescing behavior | packages/mds/src/util/options.ts:4 | 5714814 |
| Lockfile out of sync with optionalDependencies | package-lock.json | 5714814 |
| README isMdsError prefix not documented | packages/mds/README.md:100 | 5714814 |
| BackendType no JSDoc | packages/mds/src/types.ts:51 | 5714814 |
| isMdsError function no JSDoc | packages/mds/src/types.ts:73 | 5714814 |
| U-SM5 misleading test name — rename to maxModules | packages/mds/__test__/scanner.spec.mjs:133 | d03ce4d |
| U-E5b naming convention — renumber to U-E9 | packages/mds/__test__/error.spec.mjs:49 | d03ce4d |
| Browser test header misleading — correct ordering description | packages/mds/__test__/browser.spec.mjs:5 | d03ce4d |
| WASM init retry circuit breaker untested — add U-WB1, U-WB2 | packages/mds/__test__/wasm-backend.spec.mjs | d03ce4d |
| varsOpt null test coverage — strengthen U-C7 output equality | packages/mds/__test__/compile.spec.mjs:56 | d03ce4d |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| aggregateSize non-atomic across parallel scans | packages/mds/src/util/module-scanner.ts:188 | JavaScript is single-threaded. The `aggregateSize += fileSize` followed by `if (aggregateSize > max)` has no await between them — executes atomically in the event loop. Concurrent scan() calls via Promise.all cannot interleave at this point. The size limit is strict, not advisory. Added clarifying comment documenting the invariant. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| node.ts/browser.ts LSP tension — divergent APIs for same interface | packages/mds/src/browser.ts:64 | Inherent platform constraint — browser cannot do filesystem I/O. compileFile/checkFile correctly reject in browser. Splitting MdsBackend into base (compile/check) + extended (compileFile/checkFile) interfaces is a public-API-level architectural change. Not blocking for this PR scope. |

# Resolution Summary

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-23
**Review**: .devflow/docs/reviews/feat-17-universal-js-package/2026-05-23_1153
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 18 |
| Fixed | 12 |
| False Positive | 1 |
| Deferred | 0 |
| Blocked | 0 |
| Pre-existing (skipped) | 5 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| tryLoadCandidate swallows all errors — now only catches MODULE_NOT_FOUND | packages/mds/src/backend/wasm.ts:75-96 | 8066904 |
| as WasmModule unsafe assertion — added runtime shape check (compile/check) | packages/mds/src/backend/wasm.ts:87 | 8066904 |
| _init missing JSDoc — added internal documentation | packages/mds/src/backend/wasm.ts:124 | 8066904 |
| assertInitialized missing JSDoc — added single-line doc | packages/mds/src/backend/wasm.ts:166 | 8066904 |
| compileOpts manual return type — removed, let TypeScript infer | packages/mds/src/backend/wasm.ts:187 | 8066904 |
| browser.ts init() permanently caches rejected promise — added .catch() to clear cache | packages/mds/src/browser.ts:44 | 9e9f231 |
| Inconsistent options-building — extracted fileOpts() helper for compileFile/checkFile | packages/mds/src/backend/wasm.ts:208-226 | 927c8c2 |
| Double allocation in compileOpts — single direct construction, removed varsOpt import | packages/mds/src/backend/wasm.ts:187-190 | 927c8c2 |
| Phantom mds-wasm dependency — added forward-looking comment explaining future npm path | packages/mds/src/backend/wasm.ts:142 | 927c8c2 |
| U-WB1 test name misleading — renamed to clarify WASM build dependency | packages/mds/__test__/wasm-backend.spec.mjs:25 | 439d043 |
| MAX_INIT_RETRIES hardcoded without cross-reference — added source citation comment | packages/mds/__test__/wasm-backend.spec.mjs:12 | 439d043 |
| afterEach singleton reset undocumented — added isolation assumption docs | packages/mds/__test__/wasm-backend.spec.mjs:15-19 | 439d043 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| Missing test for browser.ts permanent failure path | packages/mds/src/browser.ts:41-48 | Already resolved: Batch B (commit 9e9f231) added U-BR11 which tests exactly this scenario — init() clears cached promise on rejection so next call can retry. |

## Pre-existing Issues (Not Addressed)
| Issue | File:Line | Reason |
|-------|-----------|--------|
| scanImports helper comment is outdated | packages/mds/__test__/scanner.spec.mjs:22-26 | Not introduced in this PR |
| TOCTOU window in statAndValidateModule | packages/mds/src/util/module-scanner.ts:138-208 | Inherent Node.js limitation, already mitigated |
| as object type assertions in node.ts | packages/mds/src/node.ts:27-29 | Pre-existing pattern |
| mds-napi uses file: protocol link | packages/mds/package.json:31 | Pre-release project, will be blocking at publish time |
| Test file header comment range incomplete | packages/mds/__test__/scanner.spec.mjs:2 | Minor, pre-existing |

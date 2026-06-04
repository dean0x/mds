# Resolution Summary

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22
**Review**: .devflow/docs/reviews/feat-17-universal-js-package/2026-05-22_1349
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 33 |
| Fixed | 28 |
| False Positive | 1 |
| Deferred | 4 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Object.keys(modules).length → visited.size (O(1)) + off-by-one fix | module-scanner.ts:132 | f8e0928 |
| Aggregate size race under parallel scan (use stats.size pre-reservation) | module-scanner.ts:125-130 | f8e0928 |
| content.length UTF-16 → stats.size byte-accurate (subsumed by above) | module-scanner.ts:125 | f8e0928 |
| Project root filesystem root edge case guard | module-scanner.ts:97 | f8e0928 |
| TOCTOU race: added realpath comparison after lstat | module-scanner.ts:111-123 | f8e0928 |
| Extract validateImportPath helper (reduce complexity) | module-scanner.ts:104-168 | f8e0928 |
| Dynamic import → static import for module-scanner | wasm.ts:103 | aa2d913 |
| Missing return type on buildFileModules | wasm.ts:102 | aa2d913 |
| Constants duplication: export from module-scanner, import in wasm | wasm.ts:12-13 | aa2d913 |
| WASM init() unbounded retry → MAX_INIT_RETRIES=3 | wasm.ts:40-50 | aa2d913 |
| isMdsError too broad → add .startsWith('mds::') discriminant | types.ts:46-48 | aa2d913 |
| Duplicated varsOpt → extracted to src/util/options.ts | native.ts:22, wasm.ts:98 | aa2d913 |
| MDS_BACKEND env var unsafe cast → validate at boundary | node.ts:10 | cb5e659 |
| Inconsistent type export order → standardized to types.ts order | node.ts:76-84, browser.ts:13-21 | cb5e659 |
| Mixed variable naming in browser.ts → drop underscore prefix | browser.ts:24-27 | cb5e659 |
| package-lock.json removed from .gitignore, lockfile committed | .gitignore:8 | 41c33f4 |
| @types/node ^25.9.1 → ^22.0.0 (match engines minimum) | packages/mds/package.json:34 | 41c33f4 |
| Added .npmrc with engine-strict=true | (new file) | 41c33f4 |
| Added header comment to hand-maintained napi/index.js | crates/mds-napi/index.js:1 | 41c33f4 |
| Added WASM backend subprocess test (MDS_BACKEND=wasm) | backend.spec.mjs | 14aae56 |
| Renamed parity.spec.mjs → native-backend.spec.mjs | parity.spec.mjs | 14aae56 |
| Vacuous assertion >= 0 → >= 1 | compileFile.spec.mjs:23 | 14aae56 |
| Added boundary condition tests (null/undefined vars) | compile.spec.mjs | 14aae56 |
| Package README created | packages/mds/README.md | 92e9523 |
| JSDoc on public API types | types.ts:1-48 | 92e9523 |
| JSDoc on exported functions | node.ts:42-60 | 92e9523 |
| InitOptions.wasmUrl documented with variant explanations | types.ts:35 | 92e9523 |
| CHANGELOG [Unreleased] section added | CHANGELOG.md | 92e9523 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| Scanner test uses hand-rolled regex instead of real scanImports | scanner.spec.mjs:22-39 | The napi addon only exposes check/compile/checkFile/compileFile — scanImports is not available in the native binding. The regex approach is intentional and documented with a comment. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| WASM module-level mutable singletons violate DI/testability | wasm.ts:27-29 | Architectural: changing singleton pattern affects init lifecycle across browser.ts and all consumers |
| Duplicated init logic between browser.ts and wasm.ts | browser.ts:24-51, wasm.ts:27-49 | Architectural: consolidation requires redesigning the init ownership model |
| Top-level await in node.ts makes module import fallible/non-retryable | node.ts:14-39 | API change: lazy init would change the public API contract |
| _init uses node:module unconditionally (not browser-safe) | wasm.ts:55-56 | Architectural: tied to init ownership redesign; browser.ts currently uses separate path |

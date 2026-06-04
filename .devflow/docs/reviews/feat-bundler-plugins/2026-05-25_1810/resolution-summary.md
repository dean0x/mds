# Resolution Summary

**Branch**: feat/bundler-plugins -> main
**Date**: 2026-05-25
**Review**: .devflow/docs/reviews/feat-bundler-plugins/2026-05-25_1810
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 27 |
| Fixed | 23 |
| False Positive | 3 |
| Deferred | 1 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| JSON.stringify metadata escaping (safeJsonForJs) | `bundler-utils/src/transform.ts:57` | `23e6a3d` |
| Redundant cleanId inside transform/shouldTransform | `bundler-utils/src/transform.ts:49`, `frontmatter.ts:31` | `c6facde` + `23e6a3d` |
| Double type assertion in isMdsErrorLike | `bundler-utils/src/errors.ts:12` | `c6facde` |
| Stale isMdsError in test mock | `bundler-utils/__test__/transform.spec.mjs:27` | `23e6a3d` |
| mds.d.ts metadata JSDoc | `bundler-utils/mds.d.ts:4` | `23e6a3d` |
| Non-null assertion → runtime invariant | `webpack-loader/src/index.ts:37` | `b1d6b6a` (prior) |
| _resetForTesting guard inversion | `webpack-loader/src/index.ts:63` | `b1d6b6a` (prior) |
| Poisoned-promise style unification | `webpack-loader/src/index.ts:24` | `b1d6b6a` (prior) |
| Structural types documented (vite/rollup) | `vite-plugin/src/index.ts:4`, `rollup-plugin/src/index.ts:4` | `c6facde` |
| JSDoc on exported functions | `bundler-utils/src/frontmatter.ts`, `errors.ts` | `c6facde` |
| Plugin factory JSDoc (vite/rollup) | `vite-plugin/src/index.ts:39`, `rollup-plugin/src/index.ts:33` | `c6facde` |
| HMR full-reload rationale documented | `vite-plugin/src/index.ts:81` | `c6facde` |
| file: protocol → ^0.1.0 in dependencies | 3 plugin `package.json` files | `609302d` |
| Description field added | 4 `package.json` files | `609302d` |
| Lockfile updated | `package-lock.json` | `609302d` |
| READMEs for all 4 packages | 4 `README.md` files | `0bf69c9` |
| CHANGELOG updated | `CHANGELOG.md` | `0bf69c9` |
| Top-level README bundler section | `README.md` | `0bf69c9` |
| Webpack warning emission test | `webpack-loader/__test__/loader.spec.mjs` | `512815e` |
| Vite warning emission test | `vite-plugin/__test__/plugin.spec.mjs` | `512815e` |
| U+2028/U+2029 escaping test | `bundler-utils/__test__/transform.spec.mjs` | `512815e` |
| Rollup warning path test | `rollup-plugin/__test__/plugin.spec.mjs` | `512815e` |
| Concurrent ensureInit test | `bundler-utils/__test__/transform.spec.mjs` | `512815e` |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| Webpack LoaderContext hand-rolled | `webpack-loader/src/index.ts:4` | Intentional: webpack's `LoaderContext<T>` uses CJS `export =` incompatible with pure-ESM packages. The minimal structural interface is accurate and documented. |
| Inconsistent init strategy across plugins | `vite/rollup/webpack` | Documented intentional choice: webpack loaders are stateless per-file functions requiring module-level singleton. Comments at webpack-loader:19-23 explain rationale. |
| No peerDependenciesMeta | 3 plugin `package.json` | Standard bundler plugin pattern. Each plugin declares only its own bundler as peer dep. No cross-contamination risk. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| Webpack loader duplicates init/retry logic (createLazyMdsTransformer) | `webpack-loader/src/index.ts:15-38` | Multi-package refactor touching bundler-utils + all 3 plugins. REL-1/REL-2/CONS-1b already address immediate reliability risks. Appropriate for separate refactoring PR when plugin ecosystem grows (esbuild, rspack). |

## Blocked
(none)

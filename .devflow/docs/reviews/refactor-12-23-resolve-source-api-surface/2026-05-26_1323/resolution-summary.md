# Resolution Summary

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26_1323
**Review**: .devflow/docs/reviews/refactor-12-23-resolve-source-api-surface/2026-05-26_1323
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 17 |
| Fixed | 11 |
| False Positive | 6 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| _setTransformerForTesting signature aligned to accept Transformer \| null (matching vite/rollup) | packages/webpack-loader/src/index.ts:73 | 91d545c |
| LazyInit JSDoc added to get() and reset() methods | packages/bundler-utils/src/lazy-init.ts:19,42 | 91d545c |
| LazyInit documented in bundler-utils README | packages/bundler-utils/README.md | 91d545c |
| LazyInit.get() caches resolved promise (eliminates per-call allocation) | packages/bundler-utils/src/lazy-init.ts:20 | 91d545c |
| _setTransformerForTesting JSDoc updated for sync signature | packages/webpack-loader/src/index.ts:73 | 91d545c |
| Concurrent rejection test added for LazyInit | packages/bundler-utils/__test__/lazy-init.spec.mjs | 91d545c |
| resolve_path doc comment updated (&str not "OS filesystem path") | crates/mds-core/src/resolver.rs:125 | a801394 |
| compile_str_with_rejects_non_utf8_base_dir test added | crates/mds-core/tests/api_surface.rs | a801394 |
| load_vars_file path.display() replaced with path_to_str | crates/mds-core/src/lib.rs:814 | a801394 |
| path_to_str and resolve_base_dir cross-reference docs added | crates/mds-core/src/lib.rs:255 | a801394 |
| CHANGELOG entries added for #12, #23, #32 | CHANGELOG.md | 140ca5e |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| LazyInit generation counter overflow at 2^53 | packages/bundler-utils/src/lazy-init.ts:15 | Theoretical only — 285 years at 1B resets/sec. Reviewer marked "not practically necessary." |
| Vite/Rollup don't use LazyInit for transformer init | packages/vite-plugin/src/index.ts:62 | Intentional divergence — vite/rollup have buildStart lifecycle hook guaranteeing single-call init. Reviewer marked "awareness only." |
| NODE_ENV test guard bypass | packages/webpack-loader/src/index.ts:60 | Attacker controlling NODE_ENV already has code execution. Not a real escalation path. |
| resolve_base_dir allocates String where Cow<str> could suffice | crates/mds-core/src/lib.rs:212 | Cold path (once per compile invocation). Reviewer's suggested_fix says "not practically necessary." |
| Repetitive path_to_str + resolve_path pattern across 4 functions | crates/mds-core/src/lib.rs:180 | Each function under 15 lines. Reviewer notes "not actionable now." |
| unwrap() on current_dir() in test | crates/mds-core/tests/api_surface.rs:712 | Standard Rust test pattern. Panicking is correct if process cannot determine cwd. |

## Deferred to Tech Debt

(none)

## Blocked

(none)

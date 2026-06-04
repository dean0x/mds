# Resolution Summary

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**Review**: .devflow/docs/reviews/fix-e2e-webpack-loader-esm-cjs/2026-05-27_1752
**Command**: /resolve

## Decisions Citations

- applies ADR-002 — batch-1 (verified PR content addresses linked issues for all doc fixes)
- applies ADR-001 — batch-4, batch-5 (pre-merge quality gate)

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 55 |
| Fixed | 36 |
| False Positive | 17 |
| Deferred | 2 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| I-7: MAX_NESTING_DEPTH 256→64 not in CHANGELOG | CHANGELOG.md | 2dc36f4 |
| I-27: Duplicate ### Added sections in CHANGELOG | CHANGELOG.md:10,24 | 2dc36f4 |
| I-28: string_chars grammar conflates quote contexts | spec.md:732 | 2dc36f4 |
| I-29: JS/TS bindings listed as deferred but shipped | spec.md:683 | 2dc36f4 |
| I-30: Status claims v0.1 complete with unreleased features | spec.md:741 | 2dc36f4 |
| I-51: projectRootCache no path normalization | module-scanner.ts:46 | 78a78cf |
| I-3: projectRootCache no reset/eviction mechanism | module-scanner.ts:65 | 78a78cf |
| I-2/I-12: findProjectRoot sync I/O undocumented | module-scanner.ts:37-56 | 78a78cf |
| I-8: buildModulesMap entryFilename JSDoc stale | module-scanner.ts:181 | 78a78cf |
| I-1: _esmImport accepts arbitrary string parameter | webpack-loader/src/index.ts:17 | 60d2bd0 |
| I-13: _esmImport underscore naming convention | webpack-loader/src/index.ts:17 | 60d2bd0 |
| I-22: Incomplete type guard (only checks compileFile) | webpack-loader/src/index.ts:47 | 60d2bd0 |
| I-4: Singleton silently drops differing options | webpack-loader/src/index.ts:42 | 60d2bd0 |
| I-14: parse_condition 62 lines, complexity ~9 | parser.rs:570 | 78a78cf |
| I-21: find_unquoted_operator lacks UTF-8 safety doc | parser.rs:515 | 78a78cf |
| I-18: evaluate_if lacks runtime bound on branches | evaluator.rs:377 | 8a46341 |
| I-20: CondValue::Number NaN invariant undocumented | ast.rs:17 | 8a46341 |
| I-9: Weak .endsWith() test assertions | scanner.spec.mjs:103 | 8a46341 |
| I-10: Repeated require() in CJS compat tests | cjs-compat.spec.mjs | 7bb2aae |
| I-23: U-PR3 environment-dependent test | scanner.spec.mjs:231 | 8a46341 |
| I-24: Dead capturedCallback variable | webpack-loader cjs-compat.spec.mjs:31 | 7bb2aae |
| I-25: CJS tests no skip guard for missing build | cjs-compat.spec.mjs:19 | 7bb2aae |
| I-26: Repeated require() calls (7+5 occurrences) | cjs-compat.spec.mjs | 7bb2aae |
| I-35: Section 8 example lacks new features | spec.md:541 | 84ab124 |
| I-36: number grammar missing NaN/Infinity note | spec.md:717 | 84ab124 |
| I-39: Test numbering gap U-SM8 before U-SM7 | scanner.spec.mjs:159 | 84ab124 |
| I-41: Inline build script duplicated and brittle | package.json build scripts | 84ab124 |
| I-47: No clean script for dist-cjs artifacts | package.json | 9643831 |
| I-49: Condition no-PartialEq undocumented | ast.rs:30 | 9643831 |
| I-42: Missing negation + undefined variable test | language.rs | c104bcf |
| I-44: No mixed ==/ != @elseif chain test | language.rs | c104bcf |
| I-45: esmImport lacks thenable guard | webpack-loader/src/index.ts:51 | c104bcf |
| I-19: Promise.all unbounded concurrency | module-scanner.ts:321 | ff32f30 |
| I-53: elseif_branches Vec not pre-sized | parser.rs:275 | ff32f30 |
| I-54: parse_dot_path two-pass iteration | parser.rs:430 | ff32f30 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| I-5 | webpack-loader/src/index.ts:17 | eslint-disable already line-scoped (narrowest possible); WORKAROUND comment documents intent |
| I-15 | parser.rs:174 | parse_directive 58 lines with clear dispatch structure; reviewer's own suggested_fix says "acceptable" |
| I-17 | parser.rs:455 | parse_cond_value 49 lines; reviewer says "no immediate action; well-structured" |
| I-37 | parser.rs:464 | find_unquoted_operator structure idiomatic Rust; reviewer confirms "correct" |
| I-38 | parser.rs:612 | unreachable!() already replaced with explicit error in prior commit 78a78cf |
| I-11 | package.json exports | Ordering correct per Node.js spec; single .d.ts serves both builds |
| I-31 | package.json build scripts | Already extracted to scripts/write-cjs-package.cjs in commit 84ab124 |
| I-32 | package.json:16 | default condition intentionally added per cycle-2 resolution; not redundant |
| I-33 | bundler-utils/package.json:18 | ./mds subpath is types-only ambient module declaration by design |
| I-34 | mds/package.json:8 | @mds/mds ESM-only intentional; CJS interop solved at webpack-loader level |
| I-40 | parser.rs:117 | parse_body params well-named with doc comments; builder pattern over-engineering |
| I-43 | scanner.spec.mjs | _clearProjectRootCacheForTesting already exported (batch-2 fix I-3) |
| I-46 | tsconfig.cjs.json | paths asymmetry intentional: webpack-loader imports @mds/mds, bundler-utils does not |
| I-48 | webpack-loader/src/index.ts:17 | esmImport unexported with comprehensive doc comment; no action needed |
| I-50 | webpack-loader/src/index.ts:17 | Extraction premature per YAGNI; only webpack-loader needs CJS dynamic import |
| I-52 | webpack-loader/src/index.ts:17 | Timeout machinery adds complexity without concrete failure mode; local package resolves deterministically |
| I-55 | parser.rs:580,595 | Reviewer misread control flow; negation and equality paths are mutually exclusive |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| I-6 | ast.rs:8-11 | Consolidating MAX_* constants to limits.rs touches 8+ files with mixed visibility; architectural scope |
| I-16 | parser.rs (whole file) | 1800+ lines; splitting into parser/expressions.rs requires module reorganization; no behavioral issue |

## Blocked
(none)

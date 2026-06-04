# Resolution Summary

**Branch**: feat/compiler -> main
**Date**: 2026-05-14_0114
**Review**: .docs/reviews/feat-compiler/2026-05-14_0114
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 22 |
| Fixed | 20 |
| False Positive | 2 |
| Deferred | 11 (architectural) |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| CONS-2: Add convenience constructors for Io/YamlError/JsonError/NotMdsFile | src/error.rs:437-470 | b8eed73 |
| RUST-4+RUST-6: Add #[must_use] and Clone to MdsError | src/error.rs:20 | b8eed73 |
| RUST-7: Replace unwrap() with expect() in Scope setters | src/scope.rs:90,101,112 | b8eed73 |
| TEST-3: Add error module unit tests (21 tests) | src/error.rs | b8eed73 |
| TEST-4: Add scope unit tests (10 tests) | src/scope.rs | b8eed73 |
| REL-1: Cap warnings vector with MAX_WARNINGS=1000 | src/evaluator.rs:362-366 | b9f5226 |
| REL-4: Deferred-error pattern in evaluate_for | src/evaluator.rs:339-344 | b9f5226 |
| REL-2: Add depth guard to resolve_args | src/evaluator.rs:145-177 | b9f5226 |
| TEST-1: Strengthen evaluator unit test for error content | src/evaluator.rs:411-420 | b9f5226 |
| CMPLX-3: Extract dot_notation_error helper | src/parser.rs:460-521 | ff832a2 |
| SEC-1: Migrate serde_yaml → serde_yml | Cargo.toml, src/value.rs, src/resolver.rs | 2bfb934 |
| CONS-1: Fix error variant in load_vars_file | src/lib.rs:344 | 2bfb934 |
| CONS-4: Add check_collecting_warnings API | src/lib.rs | 2bfb934 |
| SEC-2: Improve symlink TOCTOU check | src/resolver.rs:72-84 | 2bfb934 |
| PERF-6: Eliminate double-collect in to_namespace | src/resolver.rs:441-446 | 2bfb934 |
| CONS-3: Standardize Value path in integration tests | tests/integration.rs | 956cc57 |
| REG-2: Remove analysis report from repo | autobeat-orchestrator-analysis.md | 956cc57 |
| REG-5: Remove version from error messages | src/value.rs:62,96 | 956cc57 |
| REG-4: Add clean_output CRLF test | src/lib.rs | 956cc57 |
| CONS-2-CALLSITES: Migrate 14 call sites to constructors | src/resolver.rs, src/value.rs, src/lib.rs | 956cc57 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| TEST-7: EscapedCloseBrace evaluator test | src/evaluator.rs | EscapedCloseBrace does not exist in the AST. Only EscapedBrace exists. Feature knowledge was inaccurate. |
| SEC-3: mds init absolute path rejection | src/main.rs:304-313 | CLI tool — users control their own filesystem. Blocking absolute paths breaks legitimate usage (integration test uses absolute temp-dir paths). The existing .. component check covers the actual traversal vector. |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| ARCH-1/CMPLX-2: process_module SRP decomposition | src/resolver.rs:184-323 | Architectural overhaul — 140-line method with 10+ responsibilities |
| ARCH-2/PERF-2/RUST-1: Arc<FunctionDef> for closure capture | src/scope.rs, src/resolver.rs | Architectural — changes ownership model across 3+ modules |
| PERF-1/REL-3/CMPLX-1: Lexer streaming / Lexer struct decomposition | src/lexer.rs:25-241 | Architectural — 216-line function rewrite to streaming model |
| ARCH-3/CMPLX-4: resolve() method decomposition | src/resolver.rs:47-161 | Architectural — extract validation/IO from cache management |
| PERF-4: Arc<ResolvedModule> for cache hits | src/resolver.rs:93-95 | Architectural — changes cache ownership model |
| RUST-3: Arc<String> for error source strings | src/error.rs:16 | Architectural — changes error construction signature chain |
| ARCH-4: CapturedScope struct extraction | src/scope.rs:8-17 | Tied to Arc<FunctionDef> refactor |
| ARCH-5/CMPLX-6: EvalContext struct for evaluator | src/evaluator.rs | Architectural — reduces 5-param threading to context struct |
| RUST-2: HashSet→Vec for call_stack | src/evaluator.rs:29,188 | Refactoring risk — changes recursion detection semantics |
| REG-3: CLI exit code differentiation | src/main.rs:165 | API change — requires defining exit code contract |
| RUST-8: IndexSet for cycle detection | src/resolver.rs:52-53 | New dependency decision — requires evaluating indexmap |

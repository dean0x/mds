# Resolution Summary

**Branch**: feat/compiler -> main
**Date**: 2026-05-14_2158
**Review**: .docs/reviews/feat-compiler/2026-05-14_2158
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 15 |
| Fixed | 11 |
| False Positive | 4 |
| Deferred | 0 |
| Blocked | 0 |

## Fixed Issues
| Issue | File:Line | Commit |
|-------|-----------|--------|
| CRITICAL: assert! → Result return for LIFO invariant | src/evaluator.rs:196 | 01adb67 |
| Extract prefer_first_error double-fault helper | src/evaluator.rs:200 | 01adb67 |
| Promote debug_assert_eq! to assert_eq! for resolver LIFO | src/resolver.rs:204 | 3f5dfa2 |
| Extract collect_define/collect_export from collect_definitions_and_imports | src/resolver.rs:285 | 3f5dfa2 |
| Extract is_exported() private helper (3x DRY) | src/resolver.rs:467 | 3f5dfa2 |
| Fix doc comment separation (reorder MAX_CONFIG_SIZE before load_config) | src/main.rs:33 | 01adb67 |
| Extract prepare_output_dir helper from resolve_output_path | src/main.rs:111 | 01adb67 |
| Add doc comments to all ModuleCtx fields | src/resolver.rs:532 | 9f66ad7 |
| Add path traversal guard integration test | tests/integration.rs | 9f66ad7 |
| Add config size limit integration test | tests/integration.rs | 9f66ad7 |
| Add prefer_first_error unit tests (4 cases) | src/evaluator.rs | 9f66ad7 |

## False Positives
| Issue | File:Line | Reasoning |
|-------|-----------|-----------|
| Arc::new(f.clone()) per invocation | src/evaluator.rs:178 | Intentional design: CapturedScope.functions stores owned FunctionDef to break A-captures-B-captures-A reference cycles (documented in scope.rs:9-11). Changing to Arc would recreate cycles. |
| read_validated_file allocates before size check | src/resolver.rs:145 | Intentional: comment documents TOCTOU avoidance — separate metadata() + read() creates race window. Current read-then-check is the correct security tradeoff. |
| Import-depth guard runs on cache hits | src/resolver.rs:121 | Moving after cache check would allow depth bypass via cache pre-warming. Current placement is more conservative. MAX_IMPORT_DEPTH is compile-time constant, so the "lowered after caching" concern is moot. |
| exit_code_resource_limit test is slow/fragile | tests/integration.rs:3027 | Intentionally sized to trigger MAX_TOTAL_ITERATIONS (not MAX_LOOP_ITERATIONS). Comment documents the 1001x1001 sizing rationale. ~20KB source is not meaningfully slow. |

## Deferred to Tech Debt
(none)

## Blocked
(none)

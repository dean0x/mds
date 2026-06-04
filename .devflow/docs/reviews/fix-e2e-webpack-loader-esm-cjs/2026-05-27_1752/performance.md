# Performance Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Synchronous I/O in `findProjectRoot` on hot path** - `packages/mds/src/util/module-scanner.ts:52`
**Confidence**: 85%
- Problem: `findProjectRoot` uses `existsSync` (synchronous I/O) inside a loop that traverses up to `MAX_TRAVERSAL_DEPTH` (256) parent directories, checking 2 markers per level = up to 512 blocking `existsSync` calls. This runs on every Webpack loader invocation (though subsequent calls are cached per unique `start` directory). In a monorepo with deep directory structures or network-mounted filesystems, the first invocation from each unique subdirectory blocks the Node.js event loop for the duration of the traversal. The PRIOR_RESOLUTIONS note confirms a cache was already added to mitigate this, which is good. However, the uncached path remains synchronous and blocking.
- Fix: The cache mitigates repeated calls adequately for Webpack's per-file invocation pattern (same directory hit repeatedly). The synchronous approach is a deliberate design choice documented in the code comments — the project root is needed before any async work can proceed, and `findProjectRoot` mirrors the Rust `NativeFs::find_project_root` behavior. Consider adding a comment documenting the worst-case I/O count (`MAX_TRAVERSAL_DEPTH * |markers|` = 512 calls) for future maintainers. No code change required for current usage patterns.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`parse_dot_path` iterates path segments twice** - `crates/mds-core/src/parser.rs:430-446` (Confidence: 65%) — The function collects into a `Vec<&str>`, validates each part with `trim()` + `is_valid_identifier`, then maps again with `trim().to_string()`. For short dot-paths (typical: 1-3 segments) this is negligible, but a single-pass approach could trim and validate simultaneously. Micro-optimization on small data — not actionable unless profiling shows this as a hotspot.

- **`find_unquoted_operator` called twice in negation path** - `crates/mds-core/src/parser.rs:580,595` (Confidence: 60%) — When `parse_condition` receives a negated expression like `!var`, it calls `find_unquoted_operator(rest)` to check for the "negation + comparison" error case. If the condition is NOT negated, `find_unquoted_operator(s)` is called again on the full string. In the non-negated comparison path, this is a single call. In the negation error-rejection path, the double scan only occurs for malformed input that will error out anyway — no real-world performance impact.

- **`elseif_branches` Vec not pre-sized** - `crates/mds-core/src/parser.rs:275` (Confidence: 62%) — `collect_elseif_branches` starts with `Vec::new()` which will reallocate as branches are added. For the typical case (0-3 branches) this is fine. For pathological input approaching the `MAX_ELSEIF_BRANCHES` (256) limit, a `Vec::with_capacity` hint could avoid reallocations, but this is only relevant for adversarial input that's about to hit the limit anyway.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED

### Rationale

The changes in this PR are performance-neutral to positive overall:

1. **Rust parser/evaluator changes** (negation, equality, `@elseif`): All new parsing functions (`parse_condition`, `find_unquoted_operator`, `parse_cond_value`, `parse_dot_path`) operate on short condition strings (typically under 100 bytes) with O(n) single-pass scans. The `evaluate_condition` dispatch and `values_equal` comparison are both O(1) match statements. The `@elseif` evaluation loop in `evaluate_if` short-circuits on first match — no wasted work. The `MAX_ELSEIF_BRANCHES` limit (256) prevents adversarial input from creating excessive parse/eval work. No algorithmic regressions.

2. **`findProjectRoot` cache** (cycle 2 resolution): The `projectRootCache` Map converts repeated `existsSync` traversals into O(1) lookups. This directly addresses the synchronous I/O concern for the Webpack loader hot path where the same directory is visited repeatedly across file compilations. Well-designed mitigation.

3. **`_esmImport` via `new Function`**: This runs once (behind `LazyInit`), so the one-time cost of `new Function` construction is negligible. No per-file overhead.

4. **CJS dual builds**: The additional `tsc` compilation in the build script adds build-time cost but has zero runtime performance impact. The CJS output is a separate artifact that does not affect the ESM path.

5. **Nesting depth reduction** (256 -> 64): Reducing `MAX_NESTING_DEPTH` from 256 to 64 is a performance improvement — it bounds recursive stack frame depth to 1/4 of the previous limit, keeping well within the 2 MB default thread stack without needing enlarged stacks in tests. Real templates rarely exceed single-digit nesting.

No blocking performance issues. The single MEDIUM finding (sync I/O in `findProjectRoot`) is adequately mitigated by the cache that was already added in cycle 2 resolution. applies ADR-001 (squash merge gate) — the PR content demonstrates genuine performance awareness through the caching strategy and bounded resource limits.

# Security Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T13:23
**Prior Resolution Cycle**: 2026-05-26_1207 (6 fixed, 0 false positives, 0 deferred)

## Cross-Cycle Awareness

The prior resolution cycle (2026-05-26_1207) addressed 6 issues including the LazyInit TOCTOU race (generation counter fix), fire-and-forget async in `_setTransformerForTesting`, path_to_str DRY extraction, and non-UTF-8 rejection tests. All fixes are verified present in the current diff. No regressions from prior resolutions detected.

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **LazyInit generation counter overflow** - `packages/bundler-utils/src/lazy-init.ts:15,22,43` (Confidence: 60%) -- The `generation` counter is a JS `number` that increments on every `get()` and `reset()`. At Number.MAX_SAFE_INTEGER (2^53), increment produces incorrect values and the generation check silently passes. In practice this requires ~9 quadrillion calls which is unreachable in any realistic build session, so this is theoretical only. A `BigInt` or modular arithmetic guard would make it bulletproof but is not practically necessary.

- **`_setTransformerForTesting` / `_resetForTesting` env-check bypass via NODE_ENV override** - `packages/webpack-loader/src/index.ts:60,74`, `packages/vite-plugin/src/index.ts:43`, `packages/rollup-plugin/src/index.ts:37` (Confidence: 65%) -- The test-only guards rely on `process.env['NODE_ENV'] !== 'test'`. If an attacker controls the environment (e.g., supply chain attack sets NODE_ENV=test in production), they could replace the transformer with a malicious one. However, if an attacker controls process.env they already have code execution, making this a defense-in-depth observation rather than a real escalation path.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Rationale

This PR is clean from a security perspective. The changes are a net security improvement:

1. **Non-UTF-8 path handling (positive)**: The migration from `&Path` to `&str` at the resolver boundary with explicit `path_to_str()` validation eliminates silent data corruption from `path.display().to_string()` on non-UTF-8 paths. The old code could silently mangle paths containing invalid UTF-8 bytes (replacing them with U+FFFD), potentially causing the resolver to read unintended files. The new code returns an explicit error. This is the correct parse-at-boundaries pattern.

2. **TOCTOU race prevention (positive)**: The `LazyInit` generation counter prevents a stale in-flight factory resolution from overwriting state cleared by `reset()`. This eliminates a class of race condition where concurrent callers could observe inconsistent state.

3. **Import path validation (pre-existing, sound)**: `validate_import_path` requires relative paths (`./` or `../`), rejects null bytes, and delegates to `fs.normalize()` for canonicalization. This prevents path traversal attacks through import directives.

4. **JS output escaping (pre-existing, sound)**: `escapeForJs` and `safeJsonForJs` in `transform.ts` properly escape backslashes, quotes, newlines, null bytes, U+2028/U+2029 line separators, and `<` characters. This prevents XSS when compiled output is embedded in `<script>` blocks.

5. **No secrets or credentials**: No hardcoded secrets, tokens, or credentials introduced. No new network calls or external data flows.

6. **No new trust boundaries**: The `&str` API change is an internal refactor within the same trust boundary. Public API still accepts `impl AsRef<Path>` with conversion at the boundary inside `lib.rs`.

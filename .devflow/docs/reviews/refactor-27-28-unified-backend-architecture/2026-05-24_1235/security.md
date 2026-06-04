# Security Review Report

**Branch**: refactor/27-28-unified-backend-architecture -> main
**Date**: 2026-05-24
**Commits reviewed**: 5 (c57685c...3d4b9b0)

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

(none)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### Security Improvements in This Diff

The changes in this diff are overwhelmingly positive from a security perspective. No new vulnerabilities were introduced, and several existing security properties were strengthened:

1. **Aggregate size check before memory allocation** (`module-scanner.ts:239-259`): `openAndValidateModule` now returns `{handle, size}` instead of `{size, content}`. The aggregate size guard at line 246-252 runs against `fstat` metadata *before* `handle.readFile()` loads content into memory. This closes a resource exhaustion vector where a malicious file could force the process to allocate content that it knows will be rejected post-read. The handle is correctly closed on all paths (line 248 on size exceeded, line 258 in `finally` on read, line 211 in `catch` on validation failure).

2. **`openNoFollow` extraction** (`module-scanner.ts:24-34`): The symlink-rejection logic (ELOOP/ENOTDIR translation) was extracted from `openAndValidateModule` into a module-level helper. Behavior is identical -- same `O_NOFOLLOW` flag, same error codes, same security error message. The extraction reduces nesting depth in the caller without changing the security boundary. The Windows fallback (`O_NOFOLLOW=0` with post-open `realpath` comparison) remains intact at lines 200-207.

3. **Browser WASM shape validation before trust** (`wasm.ts:256-270`): `_initBrowser` previously cast the dynamically imported module directly to `WasmModule`. Now it assigns to `unknown` first and validates via `validateWasmShape()` before narrowing the type. This is a proper parse-at-boundary pattern -- untrusted dynamic imports are validated before being trusted.

4. **Browser circuit breaker** (`wasm.ts:41-42, 230-235`): The browser init path now has retry exhaustion parity with Node.js (`MAX_BROWSER_RETRIES = 3`). After 3 failed attempts, `initWasmBrowser` throws immediately without re-attempting, preventing infinite retry loops that could be exploited for DoS or timing attacks.

5. **Dead `lstat` import removed** (`module-scanner.ts:1`): The unused `lstat` import was removed. This is cosmetic but positive -- unused imports can mislead future maintainers into thinking `lstat` is part of the security model when it was replaced by `handle.stat()` (fstat) in the TOCTOU fix.

### Areas Verified (No Issues Found)

- **Handle lifecycle**: All `openNoFollow` handles are closed on every code path -- normal return, aggregate size exceeded, readFile failure, and validation error. No resource leaks.
- **No double-close**: The aggregate size exceeded path closes the handle and throws before reaching the `try/finally` readFile block. No risk of double-close.
- **No TOCTOU regression**: The O_NOFOLLOW + fstat + realpath defense-in-depth chain remains intact across the refactoring.
- **No new `any` types or unsafe casts**: `validateWasmShape` uses `unknown` input with explicit shape checks. The `as Record<string, unknown>` cast at line 124 is safe because `typeof` checks follow immediately.
- **No hardcoded secrets or credentials**: No new secret material introduced.
- **Error messages**: Security-related error messages include the path or context but do not leak internal implementation details that would aid an attacker.

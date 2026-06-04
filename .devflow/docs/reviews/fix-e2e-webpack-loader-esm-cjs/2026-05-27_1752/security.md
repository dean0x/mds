# Security Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27
**PR**: #34
**Prior Resolutions**: Cycle 2 resolved 19 issues (19 fixed, 0 FP, 0 deferred). All prior security/CSP/injection findings addressed.

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**`new Function()` used for ESM import wrapper -- eval-equivalent construct** - `packages/webpack-loader/src/index.ts:17-20`
**Confidence**: 82%
- Problem: `new Function('id', 'return import(id)')` is functionally equivalent to `eval()` and would be blocked by Content Security Policy `unsafe-eval` restrictions. While the comment accurately documents this caveat and the Node.js loader context has no CSP by default, the `_esmImport` wrapper accepts an arbitrary string `id` parameter that flows directly into a dynamic `import()`. In the current call site (line 47) it is hardcoded to `'@mds/mds'`, but the function signature accepts any string, creating a latent code-loading vector if future callers pass user-influenced values.
- Fix: Scope the wrapper more tightly -- either make it a parameter-less function that always imports `@mds/mds`, or add a JSDoc `@internal` annotation and an allowlist check:
  ```typescript
  const ALLOWED_MODULES = new Set(['@mds/mds']);
  const _esmImport: (id: string) => Promise<unknown> = new Function(
    'id',
    'return import(id)',
  ) as (id: string) => Promise<unknown>;

  async function safeEsmImport(id: string): Promise<unknown> {
    if (!ALLOWED_MODULES.has(id)) {
      throw new Error(`_esmImport: module '${id}' is not in the allowlist`);
    }
    return _esmImport(id);
  }
  ```
  This confines the dynamic import to known-safe module specifiers.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`findProjectRoot` uses unbounded synchronous `existsSync` traversal up to 256 directories** - `packages/mds/src/util/module-scanner.ts:37-60`
**Confidence**: 80%
- Problem: `findProjectRoot` performs `MAX_TRAVERSAL_DEPTH (256) * |markers| (2) = 512` synchronous `existsSync` calls in the worst case (no marker found). On network filesystems (NFS, CIFS) or deep directory trees, this can block the Node.js event loop for a significant duration. The cache mitigates repeated calls, but the first invocation per unique start directory pays the full cost. This is a defense-in-depth concern -- not exploitable for code execution, but a potential DoS vector in multi-tenant build environments.
- Fix: The bounded loop (`MAX_TRAVERSAL_DEPTH = 256`) is already a good mitigation. Consider reducing to 64 or 128 since legitimate project trees rarely exceed that depth, or document the blocking behavior more prominently for consumers running in latency-sensitive environments.

## Pre-existing Issues (Not Blocking)

(none identified in security scope)

## Suggestions (Lower Confidence)

- **`projectRootCache` is a module-level `Map` with no eviction** - `packages/mds/src/util/module-scanner.ts:25` (Confidence: 65%) -- In long-running processes (e.g., Webpack dev server with watch mode), the cache grows unboundedly as new start directories are encountered. Consider adding a max-size bound or using a WeakRef-based cache if memory pressure is a concern.

- **`find_unquoted_operator` operates on raw bytes, not UTF-8 code points** - `crates/mds-core/src/parser.rs:515-561` (Confidence: 62%) -- The byte-level scan is safe for ASCII operators (`==`, `!=`, `!`, `=`, `"`, `'`, `\\`) because all relevant characters are single-byte in UTF-8 and multi-byte UTF-8 sequences never produce bytes in the 0x00-0x7F range. However, this relies on a UTF-8 invariant that is not documented in the function. A brief comment would help future maintainers avoid introducing a byte-level check for a non-ASCII character.

- **`_esmImport` wrapper is accessible as a module-level `const`** - `packages/webpack-loader/src/index.ts:17` (Confidence: 60%) -- While not exported, bundler tooling or test harnesses that have access to the module's internal scope could invoke `_esmImport` with arbitrary specifiers. The risk is low since the module is not a library consumed by untrusted code.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions

1. Consider scoping `_esmImport` to a closed-over constant or adding an allowlist (MEDIUM blocking item above). The current pattern is safe for the single call site but creates a latent vector.

### Positive Security Observations

- **Nesting depth limit reduced from 256 to 64** (`parser.rs:17`) -- Reduces stack-overflow attack surface from crafted deeply-nested templates. Good proactive hardening.
- **`MAX_ELSEIF_BRANCHES` limit (256)** (`ast.rs:11`) -- Prevents adversarial templates from creating unbounded parse/evaluation work via excessive `@elseif` chains. The limit check runs before parsing each branch body, preventing work amplification on rejected input.
- **NaN/Infinity rejection in condition values** (`parser.rs:492-496`) -- Prevents injection of non-finite floats that could cause unexpected comparison behavior. `is_finite()` check is correct.
- **Strict equality semantics** (`evaluator.rs:336-344`) -- No type coercion means template authors cannot trick the evaluator into unintended branch selection via type confusion (e.g., `3 == "3"` is false).
- **`findProjectRoot` project-root discovery preserves path traversal guards** (`module-scanner.ts:178-183`) -- The filesystem-root sentinel check (`projectRoot === '/' || projectRoot === ''`) is correctly maintained after the refactor from `dirname(absoluteEntry)` to `findProjectRoot(dirname(absoluteEntry))`.
- **`_resetForTesting` and `_setTransformerForTesting` are gated by `NODE_ENV=test`** (`webpack-loader/src/index.ts:83-108`) -- Prevents test-only mutation helpers from being invoked in production.
- **Escape sequence handling in `find_unquoted_operator`** is correct -- backslash-escaped quotes inside string literals are properly skipped (line 528-530), preventing operator injection via `@if var == "escaped \" == injection":`.

### Decisions Context

ADR-001 (squash merge with pre-merge gate) and ADR-002 (verify PR content addresses linked issues) are process-level decisions that apply at merge time, not to code-level security findings. No code-level ADRs or pitfalls were relevant to the security changes in this PR.

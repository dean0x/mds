# Security Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`findProjectRoot` follows symlinks during marker discovery** - `module-scanner.ts:29` (Confidence: 65%) -- `existsSync(resolve(dir, marker))` follows symlinks when checking for `.git`/`.mdsroot` markers. An attacker who can place a symlink named `.git` pointing outside the filesystem tree could trick `findProjectRoot` into returning a parent directory higher than intended, widening the project root and weakening the path-confinement guard. Practical exploitation requires write access to the directory tree being scanned, which limits real-world risk. The subsequent `startsWith(projectRoot + '/')` checks and `O_NOFOLLOW` file opens still prevent reading outside the resolved root.

- **`new Function('id', 'return import(id)')` uses runtime code generation** - `webpack-loader/src/index.ts:10` (Confidence: 60%) -- While only called with the hardcoded string `'@mds/mds'` and well-documented as a TypeScript CJS workaround, `new Function` is equivalent to `eval` and would trigger CSP `unsafe-eval` in browser contexts. This is a Node.js-only webpack loader so CSP is not applicable, and the input is never user-controlled. The eslint-disable comment acknowledges the pattern. Not a real vulnerability in this context.

- **`existsSync` in `findProjectRoot` is synchronous I/O in an otherwise async codepath** - `module-scanner.ts:29` (Confidence: 62%) -- While not a direct security vulnerability, synchronous filesystem I/O in `findProjectRoot` blocks the event loop during the upward directory walk (up to 256 iterations). A deeply nested entry path on a slow filesystem could cause denial-of-service-like latency. The bounded loop (`MAX_TRAVERSAL_DEPTH = 256`) limits worst-case iterations, and this is a build-time tool where blocking is acceptable.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

### Analysis Notes

**Parser security (Rust):**
- The new condition parser (`parse_condition`, `find_unquoted_operator`, `parse_cond_value`) is well-structured with explicit rejection of edge cases: NaN/Infinity, unterminated strings, double negation, combined negation+comparison.
- `find_unquoted_operator` correctly handles backslash-escaped quotes inside string literals, preventing operator injection via crafted condition strings.
- `MAX_NESTING_DEPTH` was lowered from 256 to 64, reducing stack overflow risk -- a security improvement.
- `MAX_ELSEIF_BRANCHES` (256) prevents pathological @elseif chains from causing resource exhaustion.
- The `unescape_string` function only processes `\"`, `\'`, and `\\` -- safe and conservative. Unknown escape sequences are preserved verbatim (no interpretation of `\n`, `\t`, etc.), which avoids injection vectors.
- Strict equality semantics (no type coercion) prevent type-confusion attacks in conditional logic.

**Module scanner security (TypeScript):**
- The `findProjectRoot` change widens the project root from `dirname(absoluteEntry)` to the nearest `.git`/`.mdsroot` ancestor. This is intentional to support cross-directory imports. The existing path confinement checks (`startsWith(projectRoot + '/')`) remain intact and enforce that all resolved paths stay within the discovered root.
- Symlink detection (`O_NOFOLLOW`, realpath comparison) is unchanged and still effective.
- Resource limits (module count, aggregate size, import depth) are unchanged.
- The `projectRoot === '/' || projectRoot === ''` guard prevents the project root from being the filesystem root, which would disable path confinement.

**Webpack loader (TypeScript):**
- `_esmImport` via `new Function` is only ever called with the hardcoded `'@mds/mds'` module specifier -- no user input reaches this function.
- The runtime shape check (`typeof compileFile !== 'function'`) adds a defense-in-depth guard against module tampering or version incompatibility.
- Test-only functions (`_resetForTesting`, `_setTransformerForTesting`) are correctly gated behind `NODE_ENV=test` checks.

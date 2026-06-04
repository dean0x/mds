# Complexity Review Report

**Branch**: feat/17-universal-js-package -> main
**Date**: 2026-05-22

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`normalizeVirtualKey` approaching cyclomatic complexity threshold** - `packages/mds/src/util/module-scanner.ts:26-74`
**Confidence**: 82%
- Problem: This 48-line function has cyclomatic complexity of approximately 9 (two early-exit guards, an `if/else` branch for empty base, a for-loop with a 3-way `if/else if/else` chain, and a final empty-key guard). It is in the warning zone (5-10) per complexity guidelines. The function is still readable and well-commented, but the density of branching decisions means any future additions (new edge cases, new segment-type checks) would push it past 10.
- Fix: Consider extracting the `base.length === 0` branch into a standalone function (e.g., `normalizeRootKey`) to separate the two distinct code paths — root-level resolution and relative resolution. This would bring both halves under complexity 5:

```typescript
function normalizeRootKey(relative: string): string {
  const segmentCount = relative
    .split('/')
    .filter((s) => s.length > 0 && s !== '.')
    .length;
  if (segmentCount > MAX_PATH_SEGMENTS) {
    throw new Error(`import path exceeds maximum segment count of ${MAX_PATH_SEGMENTS}`);
  }
  return relative;
}

export function normalizeVirtualKey(base: string, relative: string): string {
  if (relative.length === 0) throw new Error('import path is empty');
  if (relative.includes('\0')) throw new Error('import path contains null byte');
  if (base.length === 0) return normalizeRootKey(relative);
  // ... relative resolution only
}
```

## Issues in Code You Touched (Should Fix)

No issues found.

## Pre-existing Issues (Not Blocking)

No issues found.

## Suggestions (Lower Confidence)

- **`node.ts` top-level backend selection uses nested try/catch** - `packages/mds/src/node.ts:19-44` (Confidence: 70%) — The native-then-WASM fallback chain nests a try/catch inside a catch block, reaching 3 levels of nesting. The nesting is justified by the fallback semantics (try native, optionally throw if forced, else try WASM, else throw combined error), and extracting helper functions for a top-level initialization sequence would add indirection without meaningful clarity gains. Noting for awareness; the current structure is acceptable given the linear fallback logic.

- **`buildModulesMap` is 120 lines including inner closures** - `packages/mds/src/util/module-scanner.ts:86-206` (Confidence: 65%) — The function spans 120 lines when counting `validateImportPath` and `scan`. However, the refactoring in this PR already extracted `validateImportPath` from the `scan` closure, reducing nesting from 4 to 3 levels inside the `Promise.all` callback. The remaining size comes from security and resource-limit checks that are deliberately co-located for auditability. Further decomposition (e.g., extracting security checks into a `validateFile` function) would scatter the security logic across functions, hurting reviewability. The current structure is a reasonable trade-off.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Complexity Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates good complexity management overall. The `normalizeVirtualKey` function is the only item approaching a threshold, and it is readable today — the suggestion is preventive, not urgent. The `validateImportPath` extraction in `module-scanner.ts` shows deliberate effort to reduce nesting. File sizes are well within limits (largest is 206 lines). Functions are focused and single-purpose. The backend abstraction pattern (native.ts, wasm.ts, browser.ts) keeps each adapter simple through consistent delegation.

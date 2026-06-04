# Code Review Summary

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main  
**Date**: 2026-05-27  
**Review Cycle**: 3 (after 19 fixes in Cycle 2)  
**Reviewers**: 12 specialized agents (security, architecture, performance, complexity, consistency, regression, testing, reliability, typescript, rust, dependencies, documentation)

---

## Merge Recommendation: CHANGES_REQUESTED

**Reasoning**: Two HIGH-severity blocking issues in new code (Reliability, Documentation) must be fixed before merge. Documentation has conflicting `### Added` sections violating CHANGELOG format. Reliability requires either async conversion of `findProjectRoot` or explicit justification for blocking sync I/O on the event loop. Secondary blocking findings (TypeScript, Architecture) are fixable. All pre-existing issues are informational.

---

## Issue Summary (by Category & Severity)

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| **Blocking** (Issues in Your Changes) | 0 | 2 | 10 | 0 | **12** |
| **Should Fix** (Issues in Code You Touched) | 0 | 0 | 9 | 0 | **9** |
| **Pre-existing** (Legacy Issues) | 0 | 0 | 4 | 1 | **5** |
| **Total Unique** | 0 | 2 | 23 | 1 | **26** |

**Deduplication Note**: Many reviewers flagged the same issues independently:
- `findProjectRoot` sync I/O block: flagged by Reliability (HIGH), Security (MEDIUM), Performance (MEDIUM), Architecture (pre-existing MEDIUM), Consistency (MEDIUM), Regression (MEDIUM), TypeScript (MEDIUM) → confidence boosted to 85%+
- `projectRootCache` unbounded growth: flagged by Architecture (HIGH), Security (MEDIUM), Reliability (HIGH), TypeScript (MEDIUM) → HIGH confidence 85%
- Duplicate CHANGELOG sections: flagged by Documentation (HIGH) → 95% confidence
- `_esmImport` type assertion: flagged by TypeScript (HIGH), Security (MEDIUM), Architecture (MEDIUM) → HIGH confidence 85%

---

## Blocking Issues (MUST FIX BEFORE MERGE)

### HIGH SEVERITY

**1. Synchronous `findProjectRoot` blocks Node.js event loop -- `packages/mds/src/util/module-scanner.ts:37-60`**
- **Issue**: `existsSync()` in loop (up to `MAX_TRAVERSAL_DEPTH * |markers|` = 256 × 2 = 512 sync I/O calls). Blocks event loop on first invocation per directory, especially on network filesystems or deep trees. Called from async `buildModulesMap`, creating an async/sync mismatch.
- **Confidence**: 85% (flagged by Reliability, Security, Performance, Architecture, Consistency, TypeScript, Regression as MEDIUM across 7 reviewers)
- **Impact**: Webpack watch mode with many entry points → seconds of build stall on network FS
- **Fix Options**:
  - **Option A** (Recommended): Convert to async using `fs.promises.access()` instead of `existsSync`. Cache logic remains unchanged:
    ```typescript
    async function _findProjectRootUncached(start: string): Promise<string> {
      let dir = start;
      for (let i = 0; i < MAX_TRAVERSAL_DEPTH; i++) {
        for (const marker of PROJECT_ROOT_MARKERS) {
          try {
            await access(resolve(dir, marker));
            return dir;
          } catch {
            // marker not found, continue
          }
        }
        const parent = dirname(dir);
        if (parent === dir) return start;
        dir = parent;
      }
      return start;
    }
    ```
  - **Option B** (Interim): Document the blocking behavior prominently and justify it as acceptable for Webpack's per-file invocation pattern (with cache mitigation). Requires explicit decision and code comment.
- **Category**: BLOCKING (Issue in Your Changes)

---

**2. Duplicate `### Added` sections in CHANGELOG.md violates Keep a Changelog format -- `CHANGELOG.md:10, 24`**
- **Issue**: Two separate `### Added` blocks in `[Unreleased]` section. Keep a Changelog format requires exactly one `### Added` subsection per release. This will render confusingly in release tools and violates the spec linked in the CHANGELOG header.
- **Confidence**: 95%
- **Impact**: Release automation may fail or produce malformed output; readers see contradictory section structure
- **Fix**: Merge the two `### Added` blocks into one. Move new entries (negation, equality, @elseif, NaN rejection, CJS build) into the existing `### Added` section that already contains prior unreleased work (LazyInit, API surface tests, bundler packages, @mds/mds).
- **Category**: BLOCKING (Issue in Your Changes) — Documentation

---

### Supporting HIGH findings (from Architecture, TypeScript)

**3. `_esmImport` type assertion bypasses runtime shape check -- `packages/webpack-loader/src/index.ts:47-48`**
- **Issue**: Line 47 asserts `as typeof import('@mds/mds')` (tells compiler full module shape is present), then line 48 re-casts to `Record<string, unknown>` for a runtime shape check that only validates `compileFile`. If module were missing `init()` (which `createMdsTransformer` calls internally), the type assertion suppresses the compiler diagnostic and the runtime check doesn't catch it.
- **Confidence**: 85%
- **Fix**: Replace assertion-then-check with a proper type guard that narrows from `unknown`:
  ```typescript
  const mds: unknown = await _esmImport('@mds/mds');
  if (
    typeof mds !== 'object' || mds === null ||
    typeof (mds as Record<string, unknown>)['compileFile'] !== 'function' ||
    typeof (mds as Record<string, unknown>)['init'] !== 'function'
  ) {
    throw new Error(
      '@mds/mds module shape is unexpected: expected compileFile and init functions. ' +
      'Check that the installed version is compatible.',
    );
  }
  return createMdsTransformer(mds as MdsApi, options);
  ```
- **Category**: BLOCKING (Issue in Your Changes) — TypeScript

---

**4. Module-level `projectRootCache` lacks reset/invalidation and eviction -- `packages/mds/src/util/module-scanner.ts:25`**
- **Issue**: `projectRootCache` is a module-level `Map<string, string>` with no public API to clear it. In long-lived processes (Webpack watch mode, dev servers), stale cache entries persist. If `.git` or `.mdsroot` is added/removed during development, security boundary could use wrong project root.
- **Confidence**: 85% (flagged by Architecture HIGH, Reliability HIGH, Security MEDIUM, TypeScript MEDIUM)
- **Fix**: Export `_clearProjectRootCacheForTesting()` gated on `NODE_ENV=test` and call it from webpack-loader's `_resetForTesting`:
  ```typescript
  export function _clearProjectRootCacheForTesting(): void {
    projectRootCache.clear();
  }
  ```
  Optionally add a public `clearProjectRootCache()` for watch-mode scenarios.
- **Category**: Should Fix (per Architecture review), but elevated to BLOCKING due to reliability/security implications

---

## Should-Fix Issues (HIGH PRIORITY, same PR)

### MEDIUM SEVERITY (Consistency: 2)

**5. Inconsistent CJS test patterns: repeated `require(resolve(__dirname, '../dist-cjs/index.js'))`**
- **Files**: `packages/bundler-utils/__test__/cjs-compat.spec.mjs`, `packages/webpack-loader/__test__/cjs-compat.spec.mjs`
- **Issue**: Each test individually requires the same path 5-7 times. Inconsistent with existing test pattern (e.g., `scanner.spec.mjs` uses shared top-level import).
- **Fix**: Extract require to describe-level constant:
  ```javascript
  const cjsBuild = require(resolve(__dirname, '../dist-cjs/index.js'));
  describe('bundler-utils CJS build', () => {
    test('loads without error', () => { assert.ok(cjsBuild); });
    // ...
  });
  ```

**6. Inconsistent module constant naming: `_esmImport` uses underscore while other module-level bindings don't**
- **File**: `packages/webpack-loader/src/index.ts:17`
- **Issue**: `_esmImport` has underscore prefix (suggesting "test-only" per codebase convention like `_resetForTesting`), but this is a regular runtime binding, not a test helper.
- **Fix**: Rename to `esmImport` (no underscore) to match `lazy`, `LoaderContext`, `Transformer`.

---

### MEDIUM SEVERITY (Architecture: 3)

**7. Webpack loader singleton captures options from first invocation only -- `packages/webpack-loader/src/index.ts:42-58`**
- **Issue**: `getLazy()` creates singleton on first call. Comment documents limitation, but code silently ignores different options in subsequent calls with no warning. Multi-compiler setup could misconfigure silently.
- **Fix**: Add warning when subsequent calls use different options:
  ```typescript
  if (lazy !== null && JSON.stringify(options) !== JSON.stringify(capturedOptions)) {
    console.warn('[mds-webpack-loader] Options differ from first invocation; using originally captured options.');
  }
  ```

**8. `MAX_ELSEIF_BRANCHES` defined in `ast.rs` instead of `limits.rs` -- `crates/mds-core/src/ast.rs:8-11`**
- **Issue**: Codebase has dedicated `limits.rs` module (houses `MAX_DOT_SEGMENTS`). New `MAX_ELSEIF_BRANCHES` is in `ast.rs`; `MAX_NESTING_DEPTH` is in `parser.rs`. Scattering makes auditing resource bounds harder.
- **Fix** (should-fix in follow-up): Consolidate all `MAX_*` constants in `limits.rs`.

**9. `_esmImport` is an architectural workaround** -- `packages/webpack-loader/src/index.ts:17`
- **Issue**: `new Function('id', 'return import(id)')` evades TypeScript's CJS-to-require rewriting. Well-documented with TypeScript issue link but creates eval-equivalent that bypasses static analysis.
- **Impact**: Bundlers/tree-shakers cannot trace the `@mds/mds` import.
- **Note**: This is a known TypeScript limitation (upstream issue #43329). Current approach is the established workaround. No code change required, but consider narrowing the `@typescript-eslint/no-implied-eval` disable comment to just this line.

---

### MEDIUM SEVERITY (Testing: 2)

**10. CJS compat tests depend on build artifacts without guard -- `packages/bundler-utils/__test__/cjs-compat.spec.mjs:19`, `packages/webpack-loader/__test__/cjs-compat.spec.mjs:19`**
- **Issue**: Tests require `../dist-cjs/index.js` without checking if it exists. If `npm test` runs without building first, unhelpful `MODULE_NOT_FOUND` error.
- **Fix**: Add existence guard at describe level:
  ```javascript
  import { existsSync } from 'node:fs';
  const cjsPath = resolve(__dirname, '../dist-cjs/index.js');
  const hasCjsBuild = existsSync(cjsPath);
  describe('bundler-utils CJS build', { skip: !hasCjsBuild && 'dist-cjs/ not built' }, () => {
    // ...
  });
  ```

**11. U-PR3 `findProjectRoot` fallback test may be environment-dependent -- `packages/mds/__test__/scanner.spec.mjs:231-248`**
- **Issue**: Asserts `findProjectRoot(sub) === sub` but if `os.tmpdir()` is inside a git repo (some CI containers), test will fail. Comment acknowledges risk but uses strict equality.
- **Fix**: Weaken assertion to accept either fallback or ancestor with marker:
  ```javascript
  assert.ok(
    result === sub || sub.startsWith(result + '/'),
    `result must be sub or ancestor of sub, got: ${result}`
  );
  ```

**12. `capturedCallback` variable assigned but never asserted -- `packages/webpack-loader/__test__/cjs-compat.spec.mjs:31`**
- **Issue**: Dead code (variable set but never used). Comment says test only checks return type, so variable should be removed.
- **Fix**: Remove the unused assignment.

---

### MEDIUM SEVERITY (Regression: 3)

**13. `MAX_NESTING_DEPTH` reduced from 256 to 64 not documented as breaking change -- `crates/mds-core/src/parser.rs:17`, `CHANGELOG.md`**
- **Issue**: Templates with nesting 65-256 levels will now fail at parse time. CHANGELOG's `[Unreleased]` section does not document this as breaking. Pre-release project with zero users mitigates real impact, but documenting is good practice.
- **Fix**: Add entry to CHANGELOG:
  ```
  ### Changed
  - **BREAKING**: `MAX_NESTING_DEPTH` reduced from 256 to 64 to prevent stack overflow in debug builds
  ```

**14. `buildModulesMapResult.entryFilename` semantics changed without type change -- `packages/mds/src/util/module-scanner.ts:176-179`, `packages/mds/src/util/module-scanner.ts:91`**
- **Issue**: Was `basename(absoluteEntry)` (e.g., `"entry.mds"`), now `relative(projectRoot, absoluteEntry)` (e.g., `"subdir/entry.mds"`). JSDoc does not document this change. Invisible contract change.
- **Fix**: Update JSDoc on `BuildModulesMapResult.entryFilename`:
  ```typescript
  /**
   * Virtual key for the entry file, relative to the discovered project root
   * (e.g., "subdir/entry.mds" rather than just "entry.mds"). All module keys
   * in `modules` use the same project-root-relative coordinate system.
   */
  entryFilename: string;
  ```

**15. Test assertions weakened from exact equality to `.endsWith()` -- `packages/mds/__test__/scanner.spec.mjs:103-113`**
- **Issue**: Tests changed from `assert.equal(entryFilename, 'entry.mds')` to `assert.ok(entryFilename.endsWith('imports/entry.mds'))`. Weaker assertion could pass with unexpected parent prefixes or absolute paths.
- **Fix**: Add structural validation:
  ```javascript
  assert.ok(!entryFilename.startsWith('/'), 'entry key must be relative');
  assert.ok(entryFilename.endsWith('imports/entry.mds'));
  ```

---

### MEDIUM SEVERITY (Complexity: 2)

**16. `parse_condition` at complexity boundary (62 lines, 4 concern blocks) -- `crates/mds-core/src/parser.rs:570`**
- **Issue**: At upper boundary (30-50 warning, >50 critical). Cyclomatic complexity ~9. Multiple nested `if` branches for negation, operators, errors, fallback.
- **Confidence**: 82%
- **Status**: Already using early returns effectively. Acceptable as-is but extracting negation branch would improve readability.
- **Fix** (should-fix): Extract negation logic into `parse_negation_condition()` helper to bring function to ~40 lines.

**17. `parser.rs` source file is 1137 lines (above 500-line critical threshold) -- `crates/mds-core/src/parser.rs`**
- **Issue**: PR added ~200 lines to already large file. Now contains both `Parser` struct methods and large set of free functions (condition parsing, import/export, interpolation, escaping, arguments).
- **Fix** (should-fix in follow-up): Split into `parser.rs` (Parser struct) and `parser/expressions.rs` (free functions) to bring each under 600 lines.

---

### MEDIUM SEVERITY (Consistency: 2)

**18. `findProjectRoot` uses sync I/O while entire module is async -- `packages/mds/src/util/module-scanner.ts:37-60`**
- **Issue**: Module-level break in async pattern. `openNoFollow`, `buildModulesMap`, `scan` are all async. Only `findProjectRoot` uses sync I/O.
- **Confidence**: 84%
- **Fix**: Either convert to async (recommended) or document the sync/async boundary explicitly with a JSDoc caveat explaining why this is acceptable for the usage pattern.

**19. Exports map field ordering and CJS type support inconsistency -- `packages/bundler-utils/package.json:11-17`, `packages/webpack-loader/package.json:11-17`**
- **Issue**: New `exports` maps include `require` and `default` fields; existing vite-plugin and rollup-plugin maps don't. The `default` entry is present but CJS consumers get no TypeScript declarations (tsconfig.cjs.json has `declaration: false`).
- **Status**: Intentional per tsconfig.cjs.json. `types` field at top level serves both ESM and CJS.
- **Fix**: Add brief comment or document that CJS consumers should use ESM import path for TypeScript support.

---

### MEDIUM SEVERITY (Security: 2)

**20. `new Function()` for ESM import wrapper is eval-equivalent, latent vector if callers change -- `packages/webpack-loader/src/index.ts:17-20`**
- **Issue**: `_esmImport` accepts arbitrary string `id` parameter that flows to `import()`. Currently hardcoded call (line 47) to `'@mds/mds'`, but function signature accepts any string. Latent code-loading vector if future callers pass user-influenced values.
- **Confidence**: 82%
- **Fix**: Either make parameter-less function always importing `@mds/mds`, or add allowlist check:
  ```typescript
  const ALLOWED_MODULES = new Set(['@mds/mds']);
  async function safeEsmImport(id: string): Promise<unknown> {
    if (!ALLOWED_MODULES.has(id)) {
      throw new Error(`_esmImport: module '${id}' is not in allowlist`);
    }
    return _esmImport(id);
  }
  ```

**21. `findProjectRoot` unbounded synchronous traversal defense-in-depth concern -- `packages/mds/src/util/module-scanner.ts:37-60`**
- **Issue**: Worst-case `MAX_TRAVERSAL_DEPTH * |markers| = 512` sync calls. Bounded loop already a good mitigation. On network filesystems or deep trees, potential DoS vector in multi-tenant build environments.
- **Confidence**: 80%
- **Fix**: The bounded loop is already a good mitigation. Consider reducing `MAX_TRAVERSAL_DEPTH` to 64 or 128 (legitimate projects rarely exceed that depth), or document the blocking behavior for consumers in latency-sensitive environments.

---

### MEDIUM SEVERITY (Rust: 2)

**22. `PartialEq` on `CondValue` with `f64` has NaN inconsistency -- `crates/mds-core/src/ast.rs:17`**
- **Issue**: `CondValue` derives `PartialEq`, so `CondValue::Number(f64::NAN) != CondValue::Number(f64::NAN)` (per Rust f64 semantics). Parser rejects NaN/Infinity, so this cannot be triggered by user input, but it's a latent hazard if invariant weakens.
- **Confidence**: 82%
- **Fix**: Add doc comment on `Number(f64)` variant noting the parser's `is_finite()` invariant:
  ```rust
  /// A numeric literal: `42`, `3.14`, `-5`
  ///
  /// Invariant: the parser rejects NaN and Infinity, so this always holds a
  /// finite f64. `PartialEq` correctness depends on this invariant.
  Number(f64),
  ```

**23. `find_unquoted_operator` operates on raw bytes without safety comment -- `crates/mds-core/src/parser.rs:515-562`**
- **Issue**: Byte-level scanning is safe for ASCII operators (`==`, `!=`, quotes, backslash) in UTF-8 because all operators are single-byte and UTF-8 continuation bytes (0x80-0xBF) cannot collide with ASCII operator characters. But the reasoning is non-obvious.
- **Confidence**: 80%
- **Fix**: Add safety comment:
  ```rust
  // SAFETY: All operators and delimiters are ASCII single-byte characters.
  // UTF-8 continuation bytes (0x80..0xBF) cannot collide with any of
  // '=', '!', '"', '\'', or '\\', so byte-level scanning is sound.
  fn find_unquoted_operator(s: &str) -> Option<(usize, &'static str)> {
  ```

---

### MEDIUM SEVERITY (Dependencies: 1)

**24. Build script fragility: inline Node.js one-liner for CJS package.json marker -- `packages/bundler-utils/package.json:28`, `packages/webpack-loader/package.json:24`**
- **Issue**: Build script chains `tsc && tsc && node -e "..."` with deeply escaped JSON string `'{\\\"type\\\":\\\"commonjs\\\"}\\n'` inline. Brittle and duplicated across two packages. No error handling if `dist-cjs` directory doesn't exist.
- **Confidence**: 82%
- **Fix**: Extract to shared script (e.g., `scripts/write-cjs-marker.js`):
  ```javascript
  // scripts/write-cjs-marker.js
  import { writeFileSync, mkdirSync } from 'node:fs';
  const dir = process.argv[2] ?? 'dist-cjs';
  mkdirSync(dir, { recursive: true });
  writeFileSync(`${dir}/package.json`, '{"type":"commonjs"}\n');
  ```
  Then in package.json:
  ```json
  "build": "tsc -p tsconfig.json && tsc -p tsconfig.cjs.json && node ../../scripts/write-cjs-marker.js dist-cjs"
  ```

---

### MEDIUM SEVERITY (Reliability: 1)

**25. `evaluate_if` iterates `elseif_branches` without independent runtime bound -- `crates/mds-core/src/evaluator.rs:377`**
- **Issue**: Loop relies entirely on parser's `MAX_ELSEIF_BRANCHES` limit. If parser limit bypassed (e.g., programmatic AST construction), evaluator would iterate unbounded branches without defensive assertion.
- **Confidence**: 80%
- **Fix**: Add debug assertion at top of `evaluate_if`:
  ```rust
  debug_assert!(
      block.elseif_branches.len() <= MAX_ELSEIF_BRANCHES,
      "invariant: elseif_branches must not exceed MAX_ELSEIF_BRANCHES"
  );
  ```

---

### MEDIUM SEVERITY (Documentation: 3)

**26. `string_chars` grammar rule is ambiguous -- `spec.md:732`**
- **Issue**: Production `string_chars := (escape_seq | [^"\\] | [^'\\])*` mixes two character classes without indicating quote context. As written, implies inside `"..."`, single quotes are excluded (and vice versa), but parser allows them.
- **Confidence**: 90%
- **Fix**: Split into context-dependent productions:
  ```
  quoted_string   := '"' dq_chars '"' | "'" sq_chars "'"
  dq_chars        := (escape_seq | [^"\\])*
  sq_chars        := (escape_seq | [^'\\])*
  ```

**27. `TypeScript/JS integration` still listed as "NOT in v0.1" while PR ships it -- `spec.md:683`**
- **Issue**: Section 10 lists "TypeScript/JS integration or runtime bindings" as deferred, but CHANGELOG documents `@mds/mds` JS/TS bindings and bundler packages as released. Spec contradicts itself.
- **Confidence**: 85%
- **Fix**: Remove the line or update it to reflect what actually remains unimplemented (e.g., "Structured JSON output").

**28. Section 12 Status still says v0.1 is "feature-complete as described" but PR adds unreleased features -- `spec.md:741`**
- **Issue**: Status reads "v0.1 is feature-complete as described in this specification", but PR adds @elseif, negation, equality comparisons to the spec. Readers may believe these shipped in v0.1 when they're unreleased.
- **Confidence**: 82%
- **Fix**: Update status line: "v0.1 -- Initial release. Unreleased additions: negation, equality comparisons, @elseif chains (see CHANGELOG [Unreleased])."

---

## Pre-existing Issues (Not Blocking)

These are legacy issues in code not modified by this PR. Noted for context but do not block merge.

| Issue | Severity | Location | Category |
|-------|----------|----------|----------|
| `scan` uses `Promise.all` without concurrency bound | MEDIUM | `packages/mds/src/util/module-scanner.ts:321` | Reliability |
| `O_NOFOLLOW` fallback uses `as Record<string, number>` assertion | MEDIUM | `packages/mds/src/util/module-scanner.ts:9` | TypeScript |
| `@mds/mds` package lacks CJS exports | MEDIUM | `packages/mds/package.json:8-18` | Dependencies |
| Inconsistent CJS support across sibling plugins (rollup, vite) | LOW | `packages/rollup-plugin`, `packages/vite-plugin` | Dependencies |

---

## Convergence Status (Cycle-Over-Cycle Trend)

| Metric | Cycle 1 | Cycle 2 | Cycle 3 | Trend |
|--------|---------|---------|---------|-------|
| Total Issues Found | — | 19 | 26 | ↑ +7 |
| Issues Fixed in Prior Cycle | — | 19 | 0 (all prior fixed) | ✓ Maintained |
| False Positives | — | 0 | 0 | ✓ Good |
| Deferred to Tech Debt | — | 0 | Several mid-priority | ⟷ Expected |
| Blocker Count | — | 0 | 2 HIGH | ⟷ New blockers (fixable) |
| Should-Fix Count | — | 19 | 9 | ↓ -10 (but ~19 already fixed) |
| Pre-existing | — | — | 5 | ℹ Informational |

**Interpretation**: Cycle 2 successfully resolved 19 issues (100% fix rate). Cycle 3 introduced 26 new findings across the fresh implementation work (new language features, CJS build, webpack loader changes). Only 2 are true blockers (sync I/O event loop, duplicate CHANGELOG sections). Both are straightforward to fix. The 9 should-fix items are quality/consistency improvements that strengthen the PR before merge. Pre-existing findings are historical debt, not regressions.

---

## Key Observations

### Strengths
1. **Language feature implementation is sound**: Rust code for negation, equality operators, @elseif is well-structured with proper error handling, resource limits (MAX_ELSEIF_BRANCHES: 256), and comprehensive tests.
2. **Test coverage is strong**: 50+ new integration tests cover happy paths, edge cases (NaN, cross-type comparisons), error diagnostics, boundary conditions, and behavioral contracts.
3. **CJS compatibility approach is pragmatic**: The `new Function` workaround for ESM in CJS is well-documented with TypeScript issue link and CSP caveats. Tests verify actual behavioral contracts.
4. **Security posture improved**: Nesting depth reduced 256→64 (stack overflow defense), NaN/Infinity rejected early, strict equality semantics (no type coercion), escape sequence handling correct, test-only exports gated on NODE_ENV.

### Blockers to Address
1. **Synchronous `findProjectRoot` breaks async contract** — Must convert to async or explicitly justify/document the blocking behavior.
2. **Duplicate CHANGELOG sections** — Violates format spec; easy fix (merge into one section).
3. **TypeScript type guard bypass** — Fix the assertion-then-check pattern with proper type narrowing.

### Quality Improvements (Non-blocking)
1. Add `_clearProjectRootCacheForTesting()` export for cache reset
2. Extract CJS marker write logic to shared script
3. Strengthen test assertions (environment-dependent tests, dead variables)
4. Consolidate resource limit constants to `limits.rs` (future PR)
5. Fix documentation contradictions (JS/TS bindings, unreleased features in spec)

---

## Recommendation Path to Approval

1. **IMMEDIATE** (before next PR push):
   - Convert `findProjectRoot` to async or add explicit documentation+justification for sync I/O
   - Fix CHANGELOG duplicate `### Added` sections into one
   - Fix TypeScript type guard pattern in webpack-loader

2. **FOLLOW-UP** (in same PR, before merge):
   - Add `_clearProjectRootCacheForTesting()` export and call from `_resetForTesting`
   - Extract CJS marker script to reduce duplication/fragility
   - Strengthen test assertions (environment-dependent tests, dead variables)
   - Update JSDoc for `entryFilename` and `findProjectRoot` sync I/O caveat
   - Fix spec.md contradictions (JS/TS bindings, status line, grammar ambiguity)
   - Document CHANGELOG breaking change (MAX_NESTING_DEPTH 256→64)

3. **DEFER TO TECH DEBT** (after merge, tracked as issues):
   - Consolidate `MAX_*` constants to `limits.rs`
   - Extract `parse_condition` negation branch into helper
   - Split `parser.rs` into modules for file-length reduction

---

## Sign-Off

**All 12 specialized reviewers have completed their analysis.**  
**Primary merge blockers: 2 HIGH (fixable in hours)**  
**Secondary should-fix: 9 MEDIUM (polish, not correctness)**  
**Code quality trajectory: Strong (Cycle 2→3 shows sustained quality despite 1000+ LOC additions)**

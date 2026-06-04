# Regression Review Report

**Branch**: feat/17-wasm-compileFile-filename-collision -> main
**Date**: 2026-05-25T00:40
**PR**: #30

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Uncommitted refactoring diverges from committed fix** - `packages/mds/src/node.ts` (working tree)
**Confidence**: 90%
- Problem: The working tree contains an uncommitted refactoring that extracts the inline fix into a `prepareFileArgs()` helper function. This refactoring is not part of the committed diff and will not be included in the PR merge unless committed. If the intent is to ship the refactored version, the PR is incomplete. If the intent is to ship the inline version, the working tree change should be discarded or stashed before merge.
- Fix: Either commit the `prepareFileArgs` refactoring as part of this PR, or discard the unstaged changes. The refactoring itself is correct and reduces duplication -- it would be a net positive to include it.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

(none)

## Regression Checklist

| Check | Status | Notes |
|-------|--------|-------|
| No exports removed without deprecation | PASS | No public exports were removed. |
| Return types backward compatible | PASS | `compileFile` and `checkFile` return types unchanged (`Promise<CompileResult>`, `Promise<CheckResult>`). |
| Default values unchanged | PASS | No default values were modified. |
| Side effects preserved | PASS | The only behavioral change is removing the entry key from `modules` before passing to WASM -- this fixes a collision, it does not remove an intended side effect. `buildModulesMap()` returns a fresh object per call, so the `delete` has no external impact. |
| All consumers of changed code updated | PASS | `wrapWithFileOps` is called from one location (`loadWasmNodeBackend` at `node.ts:121`). No other consumers exist. |
| Migration complete across codebase | PASS | No API migration was needed. The fix is internal to `wrapWithFileOps`. |
| Commit message matches implementation | PASS | Commit `1a12ca3` accurately describes the fix: extract entry source, delete from modules map, pass separately to WASM compile/check. |
| Breaking changes documented | N/A | No breaking changes introduced -- this is a bug fix. |
| Removed files check | PASS | No files were deleted. |
| `wasm/` removal from package.json `files` | PASS | The `wasm/` directory does not exist on disk and was never part of the published package contents. The WASM binary is bundled inside `dist/`. Removing the stale `files` entry is correct and prevents a publish-time warning. No consumer regression. |

## Regression Analysis

### 1. Lost Functionality

**No regressions detected.** No exports, CLI options, API endpoints, or event handlers were removed. The public API surface (`init`, `compile`, `check`, `compileFile`, `checkFile`, `getBackend`, `isMdsError`, and all type exports) is unchanged.

### 2. Broken Behavior

**No regressions detected.** The behavioral change is strictly a bug fix:
- **Before**: `compileFile`/`checkFile` via WASM always threw `mds::filename_collision` because the entry file appeared in `modules` under the same key that `build_modules()` uses internally.
- **After**: The entry source is extracted and the key is deleted before calling WASM, so WASM's `build_modules()` can insert the entry without collision.
- The `modules` object is a fresh allocation from `buildModulesMap()` on every call, so the `delete` mutation has no observable side effects outside the function scope.
- Native backend (`createNativeBackend`) delegates `compileFile`/`checkFile` directly to the NAPI addon with `addon.compileFile(path, varsOpt(options))` -- it never uses `buildModulesMap()` or `wrapWithFileOps`. The native path is completely unaffected.

### 3. Intent vs Reality Mismatch

**No mismatch detected.** The commit messages accurately describe the changes:
- `d0dbf26` adds failing tests (RED phase) -- 8 tests confirming the bug.
- `1a12ca3` fixes the bug and updates tests to match `CheckResult` type (no `dependencies` field).
- `db99f70` closes the tracking issue.

### 4. Incomplete Migrations

**No incomplete migrations.** The fix is self-contained within `wrapWithFileOps`. All call sites go through the same code path.

### 5. Test Coverage of the Fix

The new test file `wasm-compileFile.spec.mjs` provides thorough coverage:
- **U-WCF1**: Basic compile shape validation
- **U-WCF2**: Compile with imports (dependency resolution)
- **U-WCF3**: Deep import chain
- **U-WCF4**: Runtime variable overrides
- **U-WCF5**: Check result shape
- **U-WCF6**: Error handling (nonexistent file)
- **U-WCF7**: WASM/native parity for compile output
- **U-WCF8**: WASM/native parity for check output

Tests U-WCF7 and U-WCF8 (parity tests) are particularly strong regression guards -- they ensure WASM and native backends produce identical results, which would catch future drift.

The test approach (subprocess isolation with `MDS_BACKEND=wasm`) is correct for testing the WASM path in isolation without contaminating the module-level backend singleton. The test fixtures (`simple.mds`, `import_consumer.mds`, `imports/entry.mds`) are shared with existing `compileFile.spec.mjs`, ensuring consistency.

### 6. Package.json `files` Change

Removing `"wasm/"` from the `files` array is safe:
- The `wasm/` directory does not exist on disk
- The WASM binary is already bundled inside `dist/` (which remains in `files`)
- No import paths in source or tests reference `wasm/` directly
- This change prevents npm from emitting a warning about a missing directory during publish

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The single condition: resolve the uncommitted `prepareFileArgs` refactoring in the working tree -- either commit it as part of this PR (recommended, as it eliminates duplication) or discard it. The committed diff itself is clean, the fix is correct, and no regressions were detected across exports, behavior, or API surface.

# Regression Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Stale comment and assertion in security nesting depth test** - `crates/mds-cli/tests/security.rs:238,260`
**Confidence**: 90%
- Problem: The comment says "just past MAX_NESTING_DEPTH=256" and the assertion checks `err.contains("256")`, but `MAX_NESTING_DEPTH` was reduced from 256 to 64 in `parser.rs:17`. The test still passes because (a) 257 > 64 so the depth limit fires, and (b) the assertion is `||`-chained with `err.contains("nesting")` and `err.contains("depth")` which do match. However, the comment is misleading and the `"256"` check is dead code that will never match (the error now says "64"). This is a drift risk — future readers will believe the limit is 256 when it is 64.
- Fix: Update the comment and the nesting level to "65 nested @if blocks (just past MAX_NESTING_DEPTH=64)" and change `0..257` to `0..65`. Update or remove the `"256"` string from the assertion. The 8 MB stack thread is also no longer necessary with 65 levels (64 was chosen to fit the default 2 MB stack), but keeping it is harmless.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Build script error propagation** - `packages/webpack-loader/package.json:23` (Confidence: 65%) — The parallel build script `tsc -p tsconfig.json & tsc -p tsconfig.cjs.json & wait` uses shell background jobs. If the first `tsc` fails but the second succeeds, `wait` returns the exit code of the last-reaped job, potentially masking the failure. Consider `tsc -p tsconfig.json & tsc -p tsconfig.cjs.json & wait $! && wait` or sequential execution for deterministic error reporting. Same pattern applies to `packages/bundler-utils/package.json:27`.

- **`findProjectRoot` uses synchronous `existsSync`** - `packages/mds/src/util/module-scanner.ts:29` (Confidence: 60%) — The module-scanner is otherwise fully async, but `findProjectRoot` calls `existsSync` in a bounded loop (up to MAX_TRAVERSAL_DEPTH=256 iterations). For typical projects this is fast (1-5 iterations), but on deep directory trees or network filesystems the synchronous I/O could block the event loop. Consider an async variant using `fs.promises.access` if this function is ever called in latency-sensitive contexts.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Regression Analysis Summary

This PR makes several significant changes with regression potential. All were evaluated:

1. **AST breaking change: `IfBlock.condition` from `Vec<String>` to `Condition` enum** — All consumers (evaluator, validator, parser tests) were updated. The change is crate-internal (`pub` but only used within `mds-core`). No external crate references `IfBlock.condition` directly. Migration is complete. (applies ADR-002 — verified implementation matches intent)

2. **`MAX_NESTING_DEPTH` reduced from 256 to 64** — The reduction is intentional to avoid stack overflows in debug builds. The parser unit tests were updated with comments explaining the new limit. The integration test in `security.rs` still functions correctly (257 > 64 triggers the guard), though it has a stale comment and dead assertion string (see MEDIUM finding above).

3. **`entryFilename` changed from `basename()` to `relative(projectRoot, absoluteEntry)`** — Previously `buildModulesMap` returned just the filename (e.g., `entry.mds`), now it returns a project-root-relative path (e.g., `packages/mds/__test__/fixtures/imports/entry.mds`). This is an intentional behavioral change to support cross-directory imports. All consumers and tests were updated. The downstream consumer in `node.ts` passes `entryFilename` as `filename` to the WASM backend's `build_modules()`, which uses it as a virtual key — the Rust VirtualFs already expects relative paths, making this the correct behavior.

4. **New `findProjectRoot` function** — Walks up the directory tree looking for `.git` or `.mdsroot` markers. Falls back to the start directory if no marker is found within 256 parents. Bounded loop, correct filesystem root detection, no regression risk.

5. **`_esmImport` via `new Function`** — Uses a runtime-constructed function to preserve native `import()` in CJS output, preventing TypeScript from rewriting it to `require()`. This is a well-documented workaround (linked TypeScript issue). Includes a defensive runtime check that `compileFile` is a function, guarding against module shape changes.

6. **`parse_body` signature change** — Added `prefix_terminators` parameter for `@elseif` prefix matching. All 7 call sites were updated. The two-terminator-type design (exact vs prefix) cleanly separates `@end`/`@else:` (exact) from `@elseif <condition>:` (prefix).

7. **Test assertions updated for `entryFilename` change** — Tests in `scanner.spec.mjs` changed from `assert.equal(entryFilename, 'entry.mds')` to `assert.ok(entryFilename.endsWith('imports/entry.mds'))`. This is intentional and correctly validates the new relative-path behavior.

8. **Spec updated** — `spec.md` was updated to document negation, equality, `@elseif`, and all constraint rules. The removed line "No @elseif in v0.1" is correctly replaced with the new @elseif specification.

No exports were removed. No public function signatures were changed in breaking ways for external consumers. All internal API migrations are complete. Commit messages accurately describe the implementation.

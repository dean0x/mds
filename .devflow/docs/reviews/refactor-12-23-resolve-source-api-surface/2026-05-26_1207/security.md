# Security Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T12:07

## Issues in Your Changes (BLOCKING)

No blocking security issues found.

## Issues in Code You Touched (Should Fix)

No should-fix security issues found.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`display()` still used for path-to-string in `NativeFs::normalize` and `NativeFs::canonicalize`** - `crates/mds-core/src/fs.rs:310`, `crates/mds-core/src/fs.rs:353`
**Confidence**: 82%
- Problem: The PR correctly eliminates `display()` from `resolver.rs` (the old `let path_str = path.display().to_string()`) by switching `resolve_path`/`resolve_source` to accept `&str`. However, `NativeFs::normalize` at line 310 and `NativeFs::canonicalize` at line 353 still use `canonical.display().to_string()` to convert the canonicalized `PathBuf` back to a `String`. On platforms where canonicalization can produce non-UTF-8 paths (Linux with byte-sequence filenames), `display()` silently replaces invalid bytes with the Unicode replacement character, which could cause the resolved key to differ from the actual filesystem path. This is the same class of silent corruption the PR is designed to eliminate.
- Impact: If a file's canonical path contains non-UTF-8 bytes, the key stored in the module cache would not match the actual path, causing subsequent reads to fail or (in an adversarial scenario) resolve to a different file. This is a defense-in-depth concern rather than a direct exploit vector, since `check_symlink` and `check_path_traversal` are applied before the `display()` call.
- Fix: Replace `display().to_string()` with `to_str().ok_or_else(...)` in both locations, matching the pattern used throughout the new code in `lib.rs`:
```rust
// In NativeFs::normalize, line 310:
canonical.to_str()
    .ok_or_else(|| MdsError::io("canonical path is not valid UTF-8"))
    .map(str::to_owned)?

// In NativeFs::canonicalize, line 353:
Self::check_symlink(Path::new(path))
    .and_then(|p| p.to_str()
        .ok_or_else(|| MdsError::io("canonical path is not valid UTF-8"))
        .map(str::to_owned))
```

## Suggestions (Lower Confidence)

- **`_setTransformerForTesting` fire-and-forget promise** - `packages/webpack-loader/src/index.ts:79` (Confidence: 65%) -- `void lazy.get()` pre-resolves the lazy value but the promise is intentionally not awaited. If the factory somehow threw, the rejection would be unhandled until the next explicit `get()` call. This is acceptable for a test-only helper with a synchronous factory (`async () => t`), but a `.catch(() => {})` guard would be more robust against future changes.

- **`LazyInit.reset()` does not await in-flight promise** - `packages/bundler-utils/src/lazy-init.ts:34` (Confidence: 62%) -- If `reset()` is called while a factory promise is in-flight, the pending promise's `.then` handler will still write to `this.resolved` and `this.instance` after the reset. This is a race condition in theory, though current call sites only invoke `reset()` in test teardown where no concurrent `get()` is expected.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

### What this PR does well from a security perspective

1. **Eliminates silent data corruption**: The switch from `&Path` to `&str` in `resolve_path`/`resolve_source` removes the `path.display().to_string()` calls in `resolver.rs` that silently replaced non-UTF-8 bytes. The new code fails explicitly with `MdsError::io("path is not valid UTF-8")`, which is the correct behavior -- fail loudly rather than silently corrupt.

2. **Consistent UTF-8 validation at boundaries**: All five public API entry points in `lib.rs` (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`, and `resolve_base_dir`) now validate UTF-8 with explicit error messages before passing strings to the resolver. This is classic "parse at boundaries" -- exactly right.

3. **No new attack surface introduced**: The `LazyInit<T>` class is a straightforward deduplication primitive with no security-relevant surface area. The webpack loader and bundler-utils refactoring is purely structural -- the `NODE_ENV` guards on test-only functions are preserved, and no new trust boundaries are introduced.

4. **Existing security controls are preserved**: Path traversal checks (`check_path_traversal`), symlink detection (`check_symlink`), null byte rejection, import depth limits (`MAX_IMPORT_DEPTH`), file size limits (`MAX_FILE_SIZE`), and path segment limits (`MAX_PATH_SEGMENTS`) are all untouched by this PR. The `validate_import_path` function continues to enforce relative-path-only imports.

5. **XSS-safe output generation preserved**: The `escapeForJs` and `safeJsonForJs` functions in `transform.ts` (which handle U+2028/U+2029 line separators, null bytes, and `<script>` injection) are unchanged.

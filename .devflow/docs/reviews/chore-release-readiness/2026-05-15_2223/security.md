# Security Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**LIFO invariant violation in resolver allows continued execution with corrupted state** - `src/resolver.rs:212-222`
**Confidence**: 82%
- Problem: When the LIFO invariant is violated (`popped.as_ref() != Some(&canonical)`), the code first extracts the `resolved` value via `resolved?` on line 216, then checks the invariant on line 218. If `resolved` is `Ok(...)` and the LIFO invariant fails, the resolved module is discarded and an error is returned, which is correct. However, the ordering means the successfully-resolved module was computed with a corrupted `resolving` stack (the wrong entry was popped). While the error prevents the result from being used, the `process_module` call that produced `resolved` executed its full pipeline (including recursive `resolve()` calls for imports) while the `resolving` set was in an inconsistent state. In practice, this state (wrong entry popped from an IndexSet) can only occur from a compiler bug, not from user input, so exploitation is not feasible. The pre-existing `assert_eq!` that was replaced would have panicked earlier, preventing any continued execution. The new error-based approach is a net improvement for robustness (no panic on user-facing path), but the ordering of `resolved?` before the LIFO check is a subtle pre-existing design choice, not introduced by this PR.
- Fix: This is informational. No action required for this PR.

## Suggestions (Lower Confidence)

- **`mds init` path traversal check only rejects `..` components** - `src/main.rs:556-563` (Confidence: 65%) -- The check for `ParentDir` components blocks `../foo.mds` but does not validate against absolute paths (e.g., `/etc/foo.mds`). On typical Unix systems this is benign since `fs::write` to an absolute path would write to a user-controlled location, and the init command creates a benign starter template. However, for defense-in-depth, consider also rejecting absolute paths in the filename argument.

- **Symlink detection window is inherently non-atomic** - `src/resolver.rs:74-96` (Confidence: 62%) -- The symlink check performs two separate `canonicalize` syscalls (parent, then full path). Between these calls, a race condition could theoretically allow a symlink to be swapped in. The existing code comment acknowledges this ("shrinks the TOCTOU window to the unavoidable OS-level race"). For a compiler tool processing local files, this residual race is acceptable. No practical fix exists without OS-level atomic path resolution primitives.

- **Error messages include file paths that could leak directory structure** - `src/resolver.rs:90-93, src/main.rs:60-61` (Confidence: 60%) -- Canonical paths and raw user paths are included in error messages. For a CLI tool this is expected behavior (users need to see paths to debug issues). Flagging only because if the library were embedded in a server context, these paths could leak internal directory structure. Not actionable for a CLI tool.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Detailed Analysis

### Positive Security Changes in This PR

This PR makes several meaningful security improvements:

1. **TOCTOU fix in `load_config`** (`src/main.rs:57-62`): Replaced the previous `metadata().len()` check followed by `read_to_string()` with a single `fs::read()` followed by a size check on the bytes. This eliminates the TOCTOU race where a file could be swapped between the metadata check and the read. This is a genuine security hardening.

2. **Panic-to-error conversion in resolver LIFO check** (`src/resolver.rs:212-222`): Replaced `assert_eq!(popped.as_ref(), Some(&canonical))` with a graceful error return. The old `assert_eq!` would panic in production if the invariant was violated (potentially by a compiler bug triggered by crafted input). The new approach returns a structured `MdsError::Syntax` error, which is safer for a published crate that may be called from library code where panics are unacceptable.

3. **Panic-to-error conversion in evaluator LIFO check** (`src/evaluator.rs:208-215`): Same pattern -- replaced a potential panic with a structured error and the `prefer_first_error` pattern.

4. **API surface hardening** (`src/error.rs`, `src/value.rs`):
   - `#[non_exhaustive]` on `MdsError` and `Value` prevents external crates from exhaustively matching, preserving semver compatibility for future security-relevant additions.
   - `pub(crate)` on all error constructors prevents external code from constructing arbitrary error values, keeping error construction as an internal concern.
   - `pub(crate)` on `Value::from_yaml` and `Value::from_json` prevents external code from bypassing the normal parsing pipeline.

5. **Named constants** (`src/main.rs:29`, `src/resolver.rs:47`): Magic numbers replaced with `MAX_TRAVERSAL_DEPTH`, improving auditability. Both traversal loops were already bounded, so this is a readability/maintainability improvement rather than a new security control.

6. **Safe index access** (`src/main.rs:225`): `names[0]` replaced with `names.first().map(|s| s.as_str()).unwrap_or("<file>.mds")`, eliminating a potential panic on an empty vector in auto-detect error messages.

7. **Security check extraction** (`src/resolver.rs:74-119`): The three security checks (`check_symlink`, `check_import_depth`, `check_path_traversal`) are now separate named functions, improving auditability. The actual security logic is unchanged -- this is a refactoring that makes the security boundary clearer.

8. **New resource limit tests** (`src/evaluator.rs:562-633`, `tests/integration.rs:3134-3243`): Added test coverage for `MAX_CALL_DEPTH`, `MAX_OUTPUT_SIZE`, `MAX_NESTING_DEPTH`, `MAX_WARNINGS`, directory input rejection, and `mds init` path traversal. These tests validate that existing security controls actually work, which is valuable for a pre-release hardening PR.

9. **Value depth limit tests moved to unit tests** (`src/value.rs:259-303`): The YAML/JSON depth limit tests were moved from integration tests (where they called `pub` methods) to unit tests inside `value.rs` (where they can call `pub(crate)` methods). This is necessary because `from_yaml`/`from_json` are now `pub(crate)` -- the tests still exist, they just moved to a location that can access the restricted API.

### Security Controls Verified (Pre-existing, Unchanged)

The following security controls were in place before this PR and remain intact:

- **Symlink rejection**: `check_symlink` detects symlinks in the final path component
- **Path traversal prevention**: `check_path_traversal` ensures canonical paths stay within project root
- **Import depth limit**: `MAX_IMPORT_DEPTH = 64` prevents stack overflow from deep import chains
- **File size limit**: `MAX_FILE_SIZE = 10MB` prevents memory exhaustion (TOCTOU-safe read-then-check)
- **Import path validation**: `validate_import_path` requires relative paths and rejects null bytes
- **Cycle detection**: `IndexSet`-based detection prevents infinite import loops
- **All evaluator resource limits**: call depth, loop iterations, total iterations, output size, warning cap
- **Parser nesting depth limit**: `MAX_NESTING_DEPTH = 256` prevents stack overflow from crafted input
- **Config size limit**: `MAX_CONFIG_SIZE = 1MB` prevents memory exhaustion from malicious `mds.json`
- **Directory input rejection**: Prevents confusing errors from directory paths
- **Init path traversal rejection**: Prevents `mds init ../escaped.mds` from writing outside CWD

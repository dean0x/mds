# Security Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

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

## Analysis Notes

The changes in this PR are security-positive overall. Here is the detailed security analysis:

### Security Improvements Introduced by This PR

1. **Path traversal guard on `mds.json` output_dir** (`src/main.rs:149-159`): New check rejects `..` components in `output_dir` from `mds.json`, preventing a malicious config from directing output outside the project tree. The check uses `std::path::Component::ParentDir` matching, which is the correct approach since the directory may not exist yet (cannot canonicalize).

2. **Config size limit** (`src/main.rs:33-34, 54-64`): `MAX_CONFIG_SIZE` (1 MB) is now a module-level constant and enforced before reading `mds.json`, preventing memory exhaustion from maliciously large config files. The metadata check runs before `read_to_string`, so the guard is effective.

3. **Promoted `debug_assert!` to `assert!` for call_stack LIFO invariant** (`src/evaluator.rs:196-198`): The call stack invariant now enforces in release mode. This is security-relevant because a corrupted call stack would bypass recursion detection, potentially enabling stack overflows. The cost is negligible at MAX_CALL_DEPTH=128.

4. **Double-fault error preservation** (`src/evaluator.rs:200-208, src/evaluator.rs:299-307`): Render errors are now correctly preserved over scope pop errors, ensuring that actionable diagnostic information is never silently swallowed. This prevents a class of error-masking bugs.

5. **Resolver decomposition** (`src/resolver.rs:73-161`): Splitting `validate_and_read_file` into `canonicalize_and_check` + `read_validated_file` is a clean separation. All security checks (symlink detection, path traversal, import depth) remain in `canonicalize_and_check` and execute before any file I/O. The ordering is correct: security checks run even on cache hits (Step 1), file read only on cache miss (Step 4).

6. **`to_namespace()` prompt_body visibility fix** (`src/resolver.rs:508-516`): Now correctly respects export visibility for `prompt_body`, preventing leakage of non-exported prompt content through namespace aliases. This fixes a data exposure issue where `prompt_body` was previously exposed regardless of export declarations.

7. **`resolving.pop()` instead of `shift_remove()`** (`src/resolver.rs:203`): Using `pop()` for LIFO removal is both correct and O(1). The `debug_assert_eq!` verifies the invariant. This is appropriate for an internal invariant (unlike the call_stack case, this only fires during the resolve phase and a violation would cause incorrect cycle detection rather than a stack overflow).

### Pre-existing Security Controls Verified

The diff touches code near these existing security controls, and all remain intact:

- **Symlink detection** in `canonicalize_and_check`: TOCTOU-minimized comparison still in place
- **Import path validation** (`validate_import_path`): Null byte rejection and relative-path enforcement unchanged
- **Project root containment**: `starts_with(root)` check for path traversal prevention still present
- **MAX_FILE_SIZE** (10 MB): File size enforcement in `read_validated_file` unchanged
- **MAX_IMPORT_DEPTH** (64): Deep chain guard unchanged
- **MAX_CALL_DEPTH** (128), **MAX_LOOP_ITERATIONS** (100K), **MAX_TOTAL_ITERATIONS** (1M): All resource limits intact
- **Stdin size limit**: `MAX_STDIN_SIZE` read-limit-then-check pattern unchanged

### Dependency Change

`indexmap` bumped from `"2"` to `"2.2"` -- this is a minor SemVer tightening (requiring at least 2.2.x). `indexmap` is a well-maintained crate from the Rust ecosystem. No security concern.

### Why Score is 9 Rather Than 10

The `mds.json` config size check (`src/main.rs:55-57`) has a minor TOCTOU gap: `metadata().len()` is checked before `read_to_string()`, so a file could theoretically grow between the check and the read. However, this is a defense-in-depth guard (the OS imposes its own limits on `read_to_string`), the window is negligible in practice for a local CLI tool, and matching the `read-then-check-bytes.len()` pattern used in `read_validated_file` would be a style-only improvement. Not worth blocking.

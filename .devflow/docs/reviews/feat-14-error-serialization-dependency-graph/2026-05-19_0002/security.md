# Security Review Report

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19
**Focus**: Security
**Reviewer**: Automated (devflow:security)

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

This PR introduces three capabilities: (1) serializable error types (`SerializedError`/`SerializedSpan`), (2) `CompileOutput` with dependency tracking, and (3) `FileSystem::canonicalize()` trait method with symlink rejection. The security posture of this branch is strong. Detailed analysis follows.

### Path Traversal & Symlink Protection (OWASP A01)

The `NativeFs::canonicalize()` implementation at `fs.rs:343-361` correctly routes through `check_symlink()` rather than calling `std::fs::canonicalize()` directly. This prevents the root re-anchoring attack described in issue #21, where a symlinked directory could redirect the security root to an attacker-controlled location. The error re-mapping logic (`ImportError` passes through, `FileNotFound` re-wraps as `Io`) is clean and preserves the correct error semantics.

The existing path traversal defenses remain intact:
- Null-byte rejection in both `NativeFs::normalize()` and `VirtualFs::normalize()` (`fs.rs:102,288`)
- Empty-path rejection (`fs.rs:99,284`)
- Traversal depth check via `check_path_traversal()` (`fs.rs:252-262`)
- Project root boundary enforcement with `starts_with(root)` (`fs.rs:254`)
- Segment count limits in `VirtualFs` (`fs.rs:112,143`) with `MAX_PATH_SEGMENTS = 256`
- Import path validation requiring `./` or `../` prefix (`resolver.rs:727-731`)

All of these are well-tested with dedicated security boundary tests (e.g., `native_normalize_absolute_path_injection_rejected`, `native_normalize_relative_traversal_rejected`, `native_canonicalize_symlink_rejected`).

### Error Serialization - Information Disclosure (OWASP A05)

The `MdsError::serialize()` method (`error.rs:528-570`) exposes error codes, messages, help text, and source spans. This is by design for a compiler/template engine -- the error information is derived from the template source code itself, not from server internals. The serialized data includes:

- `code`: Diagnostic codes like `mds::syntax` -- safe, these are static strings
- `message`: Display representation of the error -- contains template paths and user-provided names, which are the user's own input
- `help`: Static help text from `#[diagnostic]` attributes -- safe
- `span`: Byte offsets and line/column numbers -- safe, references into user-provided source

No secrets, environment variables, stack traces, or internal filesystem paths leak through the serialization path. The `at()` helper (`error.rs:58-68`) stores file paths in `NamedSource`, but these are the normalized keys the user already provided as input.

### Resource Exhaustion / DoS (OWASP A04)

Resource limits are properly maintained across all new code paths:
- `MAX_FILE_SIZE` (10 MB) is enforced in both `NativeFs::read()` and `VirtualFs::read()` using the TOCTOU-safe read-then-check pattern
- `MAX_IMPORT_DEPTH` (64) prevents stack overflow from deep import chains
- `MAX_TRAVERSAL_DEPTH` (256) bounds the upward project-root search
- `MAX_PATH_SEGMENTS` (256) bounds VirtualFs path segment accumulation
- The new `compile_*_with_deps` functions use the same `ModuleCache` infrastructure and inherit all existing resource limits

The `compute_line_column()` function (`error.rs:40-55`) iterates over `source[..offset]` which is bounded by the already-validated source string. The `offset > source.len()` guard at line 41 prevents out-of-bounds access.

### Dependency Tracking - No New Attack Surface

`CompileOutput` (`lib.rs:67-76`) derives `serde::Serialize` and exposes `output`, `warnings`, and `dependencies`. The `dependencies` field contains normalized module keys from the `IndexMap` -- these are paths the user explicitly imported, not internal state. The entry-module exclusion logic (`lib.rs:531-534`, `lib.rs:610`) is safe and deterministic.

### Deserialization Surface (OWASP A08)

Notably, `SerializedError`, `SerializedSpan`, and `CompileOutput` only derive `serde::Serialize`, not `serde::Deserialize`. This is a correct security choice -- these types are output-only and do not accept untrusted input through deserialization.

### Why 9/10 and Not 10/10

The single point deduction reflects that the `FileSystem` trait's security contract (`fs.rs:27-42`) is documented but not enforced by the type system. Custom `FileSystem` implementations provided via `ModuleCache::with_fs()` could bypass all security controls if they ignore the documented obligations (path traversal prevention, null-byte rejection, file size limits). This is a pre-existing architectural choice, not introduced by this PR, and is appropriately documented with clear warnings. The trait-based approach is the right design for extensibility; enforcement would require sealed traits or a validation wrapper, which would limit legitimate custom backends.

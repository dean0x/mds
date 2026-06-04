# Security Review Report

**Branch**: refactor-consolidate-limits-split-parser -> main
**Date**: 2026-06-01

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
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Security Score**: 10
**Recommendation**: APPROVED

## Analysis Notes

This PR is a pure structural refactoring with no behavioral changes. The security review confirmed:

1. **Limit values preserved**: All 5 security-critical constants (`MAX_DOT_SEGMENTS=32`, `MAX_NESTING_DEPTH=64`, `MAX_ELSEIF_BRANCHES=256`, `MAX_FILE_SIZE=10MB`, `MAX_TRAVERSAL_DEPTH=256`) retain their original values. A pinning test in `limits.rs` guards against accidental drift.

2. **Visibility maintained**: All constants remain `pub(crate)`. Extracted helper functions use `pub(super)` (module-internal only), matching or tightening the original visibility. No new public API surface.

3. **No new input handling paths**: The parser helpers are a move-only extraction from `parser.rs`. No new parsing logic, no new input acceptance, no relaxed validation.

4. **Security documentation updated**: `SECURITY.md` resource limits table correctly reflects the new file locations (`limits.rs` instead of `resolver.rs` / `parser.rs`).

5. **No regression in defense-in-depth controls**: Path traversal prevention, symlink rejection, null-byte rejection, and all resource limits are unaffected by these changes.

6. **Unused parameter cleanup**: `parse_export_directive` dropped `_offset` (prefixed unused). The parameter was never consumed, so removing it has no security impact.

Decisions context reviewed (ADR-001, ADR-002): both relate to merge process, not security patterns. No applicable citations.

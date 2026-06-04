# Security Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Potential ReDoS / Algorithmic Complexity in Dot-Path Parsing** - `src/parser.rs:216-226`, `src/parser.rs:271-281`
**Confidence**: 80%
- Problem: The `@if` condition and `@for` iterable parsing splits user-supplied strings on `.` and validates each segment with `is_valid_identifier`. While `is_valid_identifier` is O(n) linear, there is no explicit upper bound on the number of dot-separated segments. An attacker could craft input like `@if a.b.c.d.e.f...` (hundreds of segments) to produce a large `Vec<String>` and many small heap allocations. However, since the overall file size is limited to 10MB by `MAX_FILE_SIZE`, this bounds the practical impact.
- Fix: Consider adding a maximum segment count for dot paths (e.g., 32 levels), consistent with `MAX_VALUE_DEPTH = 64`. This is defense-in-depth; existing limits provide indirect protection.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`strip_type_mds` may not strip all `type: mds` variants** - `src/lib.rs:342-361`
**Confidence**: 82%
- Problem: `strip_type_mds` only strips lines matching `type:` followed by whitespace then `mds`. It correctly avoids stripping indented lines (preserving nested YAML), but does not account for YAML-valid quoting styles: `type: "mds"`, `type: 'mds'`, or `type:mds` (no space). A user who writes `type: "mds"` in frontmatter to indicate a compiler file would have that line leak into compiled output, which is a minor information disclosure of internal metadata.
- Fix: Extend the check to handle common YAML quoting:
  ```rust
  let is_type_mds = line
      .strip_prefix("type:")
      .is_some_and(|v| {
          let trimmed = v.trim();
          trimmed == "mds" || trimmed == "\"mds\"" || trimmed == "'mds'"
      });
  ```

## Pre-existing Issues (Not Blocking)

(none found in changed files at CRITICAL severity)

## Suggestions (Lower Confidence)

- **`resolve_dot_path` error messages may disclose object structure** - `src/evaluator.rs:108-110` (Confidence: 65%) — When a field is not found, the error message says `"field '{field}' not found on object '{root}'"`. In contexts where templates are compiled on behalf of a user with limited knowledge of the data model, this could disclose field names of the root object. However, this is a template compiler typically run by template authors, so the risk is likely acceptable.

- **`Value::Object` with `HashMap` yields non-deterministic iteration in `Display`** - `src/value.rs:236-237` (Confidence: 62%) — The `Display` impl sorts keys before rendering, which is correct for deterministic output. However, if `Display` output were ever used in security-sensitive contexts (e.g., hashing or comparison), the sort provides the needed determinism. No current security issue, just noting the pattern is correctly implemented.

- **No depth limit on `resolve_dot_path` field traversal** - `src/evaluator.rs:100-123` (Confidence: 72%) — `resolve_dot_path` traverses arbitrarily deep nested objects. With `MAX_VALUE_DEPTH = 64` on input parsing, the maximum object nesting is bounded at 64, which bounds `resolve_dot_path` traversal depth to 64 levels. Since the parser also validates each path segment, and `fields: Vec<String>` is constructed at parse time from a bounded-length line, the practical risk is low. Still, an explicit depth guard would be defense-in-depth.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The changes introduce object/map type support with dot-notation access, frontmatter preservation, and key-value iteration. From a security perspective, the implementation is well-hardened:

1. **Existing depth limits are correctly extended** — `MAX_VALUE_DEPTH = 64` now applies to both arrays and objects in `from_yaml`/`from_json`, preventing deeply nested object bombs.
2. **Input validation is consistent** — All new dot-path parsing validates each segment with `is_valid_identifier`, preventing injection of special characters.
3. **Resource limits apply to new iteration** — Key-value object iteration correctly checks `MAX_LOOP_ITERATIONS` and `MAX_TOTAL_ITERATIONS`, preventing DoS via large objects.
4. **No new file I/O surface** — The object support does not add new filesystem access paths; all existing symlink, path traversal, and size checks remain active.
5. **Non-string YAML keys are rejected with a diagnostic** — Prevents confusion from silently dropped entries.
6. **`strip_type_mds` correctly avoids stripping nested YAML** — Only top-level (no leading whitespace) `type: mds` is removed, preserving document structure.

The two MEDIUM findings are minor: one is defense-in-depth (dot-path segment count) already bounded by `MAX_FILE_SIZE`, and the other is a minor metadata leakage edge case in frontmatter handling. Neither represents an exploitable vulnerability in the current threat model (template authors compiling their own files).

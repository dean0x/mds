# Security Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`validate_file_type` does not recognize quoted YAML variants of `type: mds`** - `src/resolver.rs:717-722`
**Confidence**: 82%
- Problem: The admission gate `validate_file_type` only accepts `.md` files when frontmatter contains the unquoted `type: mds`. However, the new `strip_type_mds` function (in `lib.rs:353-355`) now recognizes and strips `type: "mds"` and `type: 'mds'` from output. This asymmetry means `.md` files with quoted variants will be rejected at compilation time, which is the safe direction (strict admission, permissive stripping). No security bypass is possible, but it creates user confusion: if a user quotes the value, compilation is refused for `.md` files.
- Fix: This is informational — the asymmetry is safe (admission is stricter than output filtering). If consistency is desired, update `validate_file_type` at `src/resolver.rs:721` to also match quoted variants:
  ```rust
  .is_some_and(|v| {
      let v = v.trim();
      v == "mds" || v == "\"mds\"" || v == "'mds'"
  })
  ```

## Suggestions (Lower Confidence)

(none)

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 1 | 0 |

**Security Score**: 9/10
**Recommendation**: APPROVED

## Analysis Notes

The changes in this PR are security-positive. Key observations:

1. **Replacing `assert!()` with `Result` returns (evaluator.rs:330-334, 410-413)**: The previous code used `assert!(!block.condition.is_empty())` and `assert!(!block.iterable.is_empty())` which would panic in release builds if the invariant were ever violated. The new code uses `.first().ok_or_else(...)` to return a proper error. This eliminates a potential denial-of-service vector where a crafted AST (if the parser invariant were violated) could crash the process.

2. **`MAX_DOT_SEGMENTS = 32` guard (parser.rs:16, evaluator.rs:103)**: Adds a defense-in-depth limit on dot-path traversal depth. This prevents resource exhaustion from pathologically deep object paths (e.g., `a.b.c.d...` with hundreds of segments). The limit is enforced at both parse time and evaluation time, following defense-in-depth principles.

3. **`strip_type_mds` expansion (lib.rs:353-355)**: Now strips quoted YAML variants (`"mds"`, `'mds'`). This prevents leaking the compiler-internal `type: mds` directive into output when users write it with YAML quoting. The function only operates on the output side and does not affect admission/authorization logic.

4. **`run_loop_body` helper (evaluator.rs:349-362)**: Consolidates scope push/evaluate/pop into a single function that uses `prefer_first_error` for double-fault handling. This eliminates the possibility of scope leaks if the previous inline pattern were incorrectly modified.

5. **Existing security guards remain intact**: All pre-existing limits (`MAX_IMPORT_DEPTH`, `MAX_FILE_SIZE`, `MAX_NESTING_DEPTH`, `MAX_CALL_DEPTH`, `MAX_LOOP_ITERATIONS`, `MAX_TOTAL_ITERATIONS`, `MAX_OUTPUT_SIZE`, `MAX_VALUE_DEPTH`) are untouched. The symlink detection, path traversal checks, and root_dir boundary enforcement are unmodified.

No injection vectors, no hardcoded secrets, no authentication bypasses, and no new trust boundary violations were introduced.

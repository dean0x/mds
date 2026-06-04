# Reliability Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`resolve_dot_path` traversal depth is implicitly bounded but lacks an explicit guard** - `src/evaluator.rs:100-123`
**Confidence**: 82%
- Problem: The `resolve_dot_path` function iterates over `fields: &[String]` without an explicit depth bound. While the `fields` vector originates from the parser splitting on `.` within an `Interpolation` token (whose content is bounded by `MAX_FILE_SIZE = 10MB`), the loop has no local invariant assertion. In the `@if` and `@for` paths, `fields` comes from `block.condition[1..]` and `block.iterable[1..]` respectively, also parser-produced. The implicit bound is the input file size and identifier validation -- a malicious 10MB file with `{a.b.c.d....}` containing millions of dot segments would allocate a proportionally large `Vec<String>` in the parser before `resolve_dot_path` is called. Since `MAX_FILE_SIZE` caps this at 10MB of source, and each segment requires at least 2 bytes (char + dot), the maximum number of segments is ~5 million. This is bounded by file size but not by an explicit named constant.
- Fix: Add an early-exit check in `resolve_dot_path` against a named constant (e.g., `MAX_VALUE_DEPTH = 64`, which already exists for nesting) since any object nested deeper than 64 levels would have been rejected by `from_yaml`/`from_json` at parse time. This would make the implicit bound explicit and serve as defense-in-depth:
  ```rust
  fn resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError> {
      if fields.len() > MAX_VALUE_DEPTH {
          return Err(MdsError::resource_limit(format!(
              "dot path depth {} exceeds maximum of {MAX_VALUE_DEPTH}", fields.len()
          )));
      }
      // ... rest unchanged
  }
  ```
  Note: Since `from_yaml`/`from_json` already reject values nested > 64 levels, this is a belt-and-suspenders check. The implicit bound already prevents exploitation, but the explicit check satisfies the Iron Law ("every loop must have a fixed upper bound") and prevents future regressions if a new Value construction path is added that bypasses depth checks.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`evaluate_for` key-value path builds a local `output` buffer without per-iteration size check** - `src/evaluator.rs:366-381` (Confidence: 65%) -- The key-value `@for` iteration builds a local `String` buffer (`output`) that is only checked against `MAX_OUTPUT_SIZE` by the caller (`evaluate_nodes`) after the entire loop result is returned. For a very large object (up to 100k entries per `MAX_LOOP_ITERATIONS`), the local `output` could temporarily exceed `MAX_OUTPUT_SIZE` before being checked. The existing array `@for` path has the same pattern, so this is consistent behavior, not a regression. The `MAX_LOOP_ITERATIONS` bound (100k entries) combined with realistic value sizes makes this unlikely to cause OOM in practice.

- **Key-value iteration sorts all keys into a new Vec before iterating** - `src/evaluator.rs:362-364` (Confidence: 62%) -- `entries.sort_by(...)` allocates a `Vec<(String, Value)>` from the consumed `HashMap`. For large objects (up to 100k entries allowed by `MAX_LOOP_ITERATIONS`), this is a full copy of all key-value pairs. This is acceptable given the bounded iteration limit, but could be noted as a future optimization target if objects grow large.

- **`strip_type_mds` iterates all lines without size bound** - `src/lib.rs:342-361` (Confidence: 60%) -- The function iterates over `raw.lines()` and builds a new `String`. Since `raw` comes from the frontmatter content within a file already bounded by `MAX_FILE_SIZE = 10MB`, this is implicitly bounded and not a practical concern.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The implementation demonstrates strong reliability practices overall:

1. **Bounded iteration**: All loops respect `MAX_LOOP_ITERATIONS` (100k per loop) and `MAX_TOTAL_ITERATIONS` (1M cumulative). The new key-value `@for` path correctly checks both limits.
2. **Assertion density**: The `assert!(!block.condition.is_empty())` and `assert!(!block.iterable.is_empty())` in the evaluator enforce parser invariants at runtime in release builds. The validator uses `.first().ok_or_else()` instead of indexing -- avoiding panics.
3. **Depth bounds**: `MAX_VALUE_DEPTH = 64` bounds YAML/JSON nesting during parsing. `MAX_NESTING_DEPTH = 256` bounds parser block and argument depth. `MAX_CALL_DEPTH = 128` bounds call stack.
4. **Error propagation**: All new paths use `?` for error propagation. The `prefer_first_error` pattern is correctly applied in the key-value loop.
5. **No unbounded retries or I/O**: All new code is CPU-bound traversal of pre-parsed in-memory data structures.

The single MEDIUM finding is a defense-in-depth suggestion. The `resolve_dot_path` loop is already implicitly bounded by `MAX_VALUE_DEPTH` (values deeper than 64 cannot exist) and by the file-size limit on parser input. Adding an explicit bound would document this invariant and prevent regressions.

Condition for approval: Consider adding the explicit depth guard in `resolve_dot_path` before merge (low effort, high clarity). If intentionally deferred, document the implicit bound with a comment.

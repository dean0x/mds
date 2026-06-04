# Architecture Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Redundant depth guard in evaluator duplicates parser-layer validation** - `src/evaluator.rs:103-107`
**Confidence**: 82%
- Problem: `resolve_dot_path` checks `fields.len() > MAX_DOT_SEGMENTS` at runtime, but the parser already enforces this limit at parse time for all dot-path inputs (interpolation, @if condition, @for iterable, function args). The evaluator guard can only trigger if an internal caller constructs a path that bypasses the parser — making this a defense-in-depth check rather than a layering issue. However, importing `MAX_DOT_SEGMENTS` from `parser` into `evaluator` creates a cross-layer dependency where the evaluator depends on a parser constant.
- Fix: Define the constant once in a shared location (e.g., a `limits` module or re-export from `lib.rs`) rather than having evaluator reach into parser internals. This preserves the defense-in-depth check while respecting layer boundaries:
  ```rust
  // src/limits.rs (new)
  pub(crate) const MAX_DOT_SEGMENTS: usize = 32;
  
  // src/parser.rs
  use crate::limits::MAX_DOT_SEGMENTS;
  
  // src/evaluator.rs
  use crate::limits::MAX_DOT_SEGMENTS;
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`evaluate_for_key_value` takes ownership of HashMap unnecessarily** - `src/evaluator.rs:365-372`
**Confidence**: 83%
- Problem: The function signature `evaluate_for_key_value(..., map: HashMap<String, Value>, ...)` takes ownership of the map, which is fine for the current call site (the map is moved out of a `Value::Object` match arm). However, this creates a function that can only be called with owned data. If a future call site has a reference to the map (e.g., iterating without consuming), it would need to clone. The `into_iter().collect()` followed by sort is also less efficient than collecting keys and iterating by key lookup.
- Fix: This is acceptable for now since the single call site naturally owns the data. Consider accepting `&HashMap<String, Value>` if a second call site emerges that would need to clone.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`parse_dot_expr` parameter count (6 params)** - `src/parser.rs:521-528` (Confidence: 68%) — The extracted helper takes 6 parameters (content, dot_pos, offset, len, file, source). This is at the edge of comfortable arity. A context struct could reduce this, but the function is private and called from a single site, so the tradeoff is marginal.

- **Sorted key iteration allocates a full Vec** - `src/evaluator.rs:381-382` (Confidence: 62%) — `map.into_iter().collect()` then `.sort_by()` allocates a Vec for deterministic output. For small objects this is fine; for objects approaching MAX_LOOP_ITERATIONS (100k), a BTreeMap conversion or sorted insertion would be more memory-efficient. Current limit makes this unlikely to matter in practice.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The architecture of these changes is sound. The pipeline layering (lexer -> parser -> validator -> resolver -> evaluator -> render) is respected. Helper extraction (`run_loop_body`, `evaluate_for_key_value`, `parse_dot_expr`) improves SRP by giving each function a single responsibility. The replacement of `assert!()` with `Result` returns aligns with the project's error-handling contract (no panics in business logic). The `MAX_DOT_SEGMENTS` constant is a good defense-in-depth addition.

The single blocking issue is the cross-layer import (`evaluator` importing a constant from `parser`). This creates a dependency where the evaluator layer knows about parser internals. The fix is straightforward: move the constant to a shared location. This is a minor layering concern rather than a fundamental design flaw.

Strengths observed:
- Clean helper extraction follows SRP
- Consistent error-handling pattern (Result throughout, no panics)
- Depth guards applied uniformly at all parse sites
- `EvalContext` struct keeps mutable state bundled cleanly
- `run_loop_body` eliminates code duplication with a clean abstraction

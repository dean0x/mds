# Code Review Summary

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16_1249
**Reviewers**: 9 (security, architecture, performance, complexity, consistency, regression, testing, reliability, rust)

## Merge Recommendation: CHANGES_REQUESTED

The three enhancements (object/map support, frontmatter preservation, escape docs) are well-implemented with strong test coverage (325 passing tests, zero clippy warnings). However, **one HIGH-severity blocking issue and multiple MEDIUM-severity blocking issues must be fixed before merge**:

1. **Data corruption in `strip_type_mds`** - silently corrupts nested YAML objects (HIGH - CRITICAL to fix)
2. **Debug assertions should be production guards** - three `debug_assert` + unchecked index patterns create release-mode panic risks (MEDIUM - 4 reviewers flagged)
3. **Silent YAML key drops** - non-string keys in YAML are silently discarded (MEDIUM - 3 reviewers flagged)
4. **Diagnostic quality regressions** - lost source-location context in parse errors and validator errors (MEDIUM - 2 reviewers flagged)
5. **Redundant allocations** - Vec allocations on hot paths in MemberAccess evaluation (HIGH - 3 reviewers flagged)

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 2 | 5 | 0 | 7 |
| Should Fix | 0 | 0 | 5 | 0 | 5 |
| Pre-existing | 0 | 0 | 2 | 1 | 3 |
| **TOTAL** | 0 | 2 | 12 | 1 | 15 |

---

## Blocking Issues (Must Fix Before Merge)

### HIGH Severity

**1. `strip_type_mds` silently corrupts nested YAML objects** - `src/lib.rs:342-358`
- **Severity**: HIGH
- **Confidence**: 95%
- **Problem**: The function uses `line.trim().strip_prefix("type:")` which removes leading whitespace. This matches `type: mds` inside nested YAML structures (e.g., `nested:\n  type: mds`), corrupting the output YAML. Example: frontmatter with a nested object containing `type: mds` will have that line stripped, breaking the object structure.
- **Impact**: Silent data corruption in compiled output when templates have `type: mds` as an application-specific key in nested objects.
- **Fix**: Only strip top-level `type: mds` by checking for no leading indentation:
```rust
fn strip_type_mds(raw: &str) -> Option<String> {
    let filtered: String = raw
        .lines()
        .filter(|line| {
            // Only strip top-level type: mds (no leading whitespace)
            !(!line.starts_with(' ') && !line.starts_with('\t')
              && line.strip_prefix("type:")
                  .is_some_and(|v| v.trim() == "mds"))
        })
        .map(|line| format!("{line}\n"))
        .collect();
    if filtered.trim().is_empty() {
        None
    } else {
        Some(filtered)
    }
}
```

**2. Redundant Vec allocation on every MemberAccess evaluation** - `src/evaluator.rs:164-166`, `src/evaluator.rs:207-209`
- **Severity**: HIGH
- **Confidence**: 85%
- **Problem**: Both `evaluate_expr(Expr::MemberAccess)` and `resolve_args(Arg::MemberAccess)` allocate a new `Vec<String>` by cloning the object name and fields, only to pass it to `resolve_dot_path(&[String])`. This occurs on every dot-notation access and is especially wasteful in loops.
- **Impact**: Unnecessary allocation pressure on hot paths.
- **Fix**: Refactor `resolve_dot_path` to accept root + fields directly:
```rust
fn resolve_dot_path_parts(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError> {
    let mut current = scope
        .get_var(root)
        .cloned()
        .ok_or_else(|| MdsError::undefined_var(root))?;
    for field in fields {
        match current {
            Value::Object(ref map) => {
                current = map.get(field).cloned().ok_or_else(|| {
                    MdsError::syntax(format!("field '{field}' not found on object '{root}'"))
                })?;
            }
            _ => {
                return Err(MdsError::syntax(format!(
                    "cannot access field '{field}' on {}", current.type_name()
                )));
            }
        }
    }
    Ok(current)
}
```

### MEDIUM Severity (Blocking)

**3. Debug assertions should be production guards** - `src/evaluator.rs:100`, `src/validator.rs:25,49`
- **Severity**: MEDIUM
- **Confidence**: 90% (4 reviewers flagged)
- **Problem**: Three locations use `debug_assert!()` followed by unchecked indexing:
  - `resolve_dot_path`: `debug_assert!(!path.is_empty(), ...)` → `let root = &path[0];`
  - `validate_node` (If arm): `debug_assert!(!block.condition.is_empty(), ...)` → `&block.condition[0]`
  - `validate_node` (For arm): `debug_assert!(!block.iterable.is_empty(), ...)` → `&block.iterable[0]`
  
  `debug_assert!` is stripped in release builds, meaning if a parser bug produces an empty Vec, release builds will panic with an opaque index error instead of a diagnostic message. The project's precedent (LIFO check in evaluator) uses `assert!` for safety-critical invariants.
- **Impact**: Release-mode panics without helpful error messages if invariants are violated.
- **Fix**: Use `.first()` with proper error return for production defense:
```rust
// In resolve_dot_path
let root = path.first().ok_or_else(|| {
    MdsError::syntax("internal error: empty dot path (parser bug)")
})?;

// In validate_node (If arm)
let condition_val = scope.get_var(&block.condition.first()
    .ok_or_else(|| MdsError::syntax("empty condition path"))?)?;

// In validate_node (For arm)
let iterable_val = if block.iterable.len() > 1 {
    // dot path
    resolve_dot_path(&block.iterable, scope)?
} else {
    scope.get_var(&block.iterable.first()
        .ok_or_else(|| MdsError::syntax("empty iterable path"))?)?
};
```

**4. Silent YAML non-string key drops** - `src/value.rs:66-70`
- **Severity**: MEDIUM
- **Confidence**: 85% (3 reviewers flagged)
- **Problem**: When converting YAML mappings with integer, boolean, or null keys, they are silently skipped with no warning or error. YAML allows non-string keys, but MDS only supports strings. A user with `42: answer` in frontmatter will see it disappear silently.
- **Impact**: Silent data loss; confusing "field not found" errors at runtime with no indication the key existed.
- **Fix**: Return an error with clear diagnostic:
```rust
for (k, v) in mapping {
    let key = match k {
        serde_yml::Value::String(s) => s,
        other => {
            return Err(MdsError::yaml_error(format!(
                "MDS only supports string keys in objects, found: {other:?}"
            )));
        }
    };
    let value = Self::from_yaml_inner(v, depth + 1)?;
    map.insert(key, value);
}
```

**5. Lost source-location context in parse errors** - `src/parser.rs:500-501`
- **Severity**: MEDIUM
- **Confidence**: 90% (2 reviewers flagged)
- **Problem**: The `file` and `source` parameters in `parse_interpolation_expr` were renamed to `_file` and `_source` after the `dot_notation_error` function (the sole consumer) was removed. Now all errors from this function use `MdsError::syntax(msg)` instead of `MdsError::syntax_at(msg, file, source, offset, len)`. This is a regression in diagnostic quality for invalid dot-paths like `{a.123.b}`.
- **Impact**: Parse errors lose file:line:column context, making it harder for users to locate mistakes.
- **Fix**: Restore parameter names and use `syntax_at` for errors with offset context:
```rust
fn parse_interpolation_expr(
    content: &str,
    offset: usize,
    file: &str,
    source: &str,
) -> Result<Interpolation, MdsError> {
    // At line 544, use syntax_at instead of syntax:
    return Err(MdsError::syntax_at(
        format!("invalid dot-path in interpolation: '{content}'"),
        file, source, offset, content.len(),
    ));
}
```

**6. Validator skips static type checks for dot-path iterables** - `src/validator.rs:60-83`
- **Severity**: MEDIUM
- **Confidence**: 85%
- **Problem**: The validator skips the static array-type check when `block.iterable.len() > 1` (i.e., for dot paths). Type errors that were caught at validate time now surface only at evaluation time, with less precise span-aware diagnostics.
- **Impact**: Error quality regression; users get generic type errors without source spans for dot-path iteration.
- **Fix**: Document this as an accepted limitation, or attempt static resolution of dot paths in the validator when all intermediate values are known.

---

## Should-Fix Issues (High Priority, Same Session)

### MEDIUM Severity (Same File / Related)

**7. `strip_type_mds` creates per-line format! allocations** - `src/lib.rs:346-351`
- **Severity**: MEDIUM
- **Confidence**: 82%
- **Problem**: `.map(|line| format!("{line}\n"))` allocates a new String for every frontmatter line just to append a newline.
- **Fix**: Use pre-sized buffer with `push_str`:
```rust
fn strip_type_mds(raw: &str) -> Option<String> {
    let mut filtered = String::with_capacity(raw.len());
    for line in raw.lines() {
        if !line.strip_prefix("type:")
            .is_some_and(|v| v.trim() == "mds") {
            filtered.push_str(line);
            filtered.push('\n');
        }
    }
    if filtered.trim().is_empty() {
        None
    } else {
        Some(filtered)
    }
}
```

**8. Inconsistent dot-path representation in AST** - `src/ast.rs:64-66`, `src/ast.rs:81-83`, `src/ast.rs:91`, `src/ast.rs:103`
- **Severity**: MEDIUM
- **Confidence**: 84%
- **Problem**: Dot paths are represented two different ways:
  - `Expr::MemberAccess` and `Arg::MemberAccess`: `{ object: String, fields: Vec<String> }`
  - `IfBlock.condition` and `ForBlock.iterable`: `Vec<String>` (full flat path)
  
  This forces reconstruction overhead in `evaluate_expr` and `resolve_args` (lines 164-166, 207-209).
- **Fix**: Standardize on `Vec<String>` representation. Rename `object` and `fields` to a single `path: Vec<String>` in `MemberAccess` variants.

**9. Unused `_file` and `_source` parameters** - `src/parser.rs:500-501`
- **Severity**: MEDIUM
- **Confidence**: 85%
- **Problem**: Parameters are dead code (unused after removing `dot_notation_error`), creating a trap for future developers.
- **Fix**: Remove the parameters and the underscore prefixes, then use them for `syntax_at` errors (fix #5 above covers this).

**10. `evaluate_for` has near-duplicate loop bodies** - `src/evaluator.rs:338-431`
- **Severity**: MEDIUM
- **Confidence**: 88%
- **Problem**: The function now handles two iteration modes (key-value over objects, standard array iteration) with nearly identical loop bodies. At 94 lines, it exceeds the 50-line warning threshold and creates maintenance risk.
- **Fix**: Extract the shared loop body into a helper function `evaluate_loop_iteration`.

**11. `parse_for_block` has grown to 72 lines** - `src/parser.rs:249-320`
- **Severity**: MEDIUM
- **Confidence**: 82%
- **Problem**: Function crossed the 50-line threshold with multi-phase parsing logic.
- **Fix**: Extract variable-part parsing into `parse_for_vars` helper.

---

## Pre-existing Issues (Not Blocking)

**12. `HashMap<String, Value>` for `Value::Object` loses insertion order** - `src/value.rs:17`
- **Severity**: MEDIUM
- **Confidence**: 85%
- **Problem**: Object field order from YAML is not preserved. While the code sorts for determinism, insertion order is lost. `indexmap::IndexMap` (already a dependency) would preserve order.
- **Note**: Design choice, not a bug. Flagged for future consideration.

**13. No `as_object()` accessor on `Value` -- asymmetry with `as_array()`** - `src/value.rs:114-121`
- **Severity**: MEDIUM
- **Confidence**: 85%
- **Problem**: `Value` provides `as_array()` but not `as_object()`, forcing pattern matching for object access.
- **Fix**: Add `pub fn as_object(&self) -> Option<&HashMap<String, Value>>` to `Value`.

---

## Action Plan

**Priority 1 (Blocking, do not merge without):**
1. Fix `strip_type_mds` to only strip top-level `type: mds` (HIGH - data corruption)
2. Fix the three `debug_assert` + unchecked index patterns (MEDIUM - 4 reviewers)
3. Fix silent YAML key drops (MEDIUM - 3 reviewers)
4. Restore source-location context to parse errors (MEDIUM - 2 reviewers)
5. Fix redundant Vec allocations in MemberAccess evaluation (HIGH - 3 reviewers)

**Priority 2 (High quality, same session):**
6. Optimize `strip_type_mds` to use pre-sized buffer
7. Standardize dot-path representation (or at least document the dual representation)
8. Remove unused parameter prefixes in `parse_interpolation_expr`
9. Extract `evaluate_for` loop bodies into helper
10. Extract `parse_for_block` variable parsing into helper

**Priority 3 (Informational, can defer):**
11. Consider `IndexMap` for `Value::Object` insertion order
12. Add `as_object()` accessor to `Value`

---

## Summary by Reviewer

| Reviewer | Focus | Blocking | Should-Fix | Severity |
|----------|-------|----------|-----------|----------|
| Security | Injection, secrets, data integrity | 2 MEDIUM | 0 | Silent key drop, frontmatter exposure |
| Architecture | Design, coupling, layering | 1 HIGH, 1 MEDIUM | 3 MEDIUM | Validator skips checks, debug_assert, dead params |
| Performance | Allocations, algorithmic complexity | 1 HIGH, 1 MEDIUM | 1 MEDIUM | Vec allocation, format! loops, deep clones |
| Complexity | Function size, cyclomatic complexity | 2 HIGH, 1 MEDIUM | 1 MEDIUM | evaluate_for, parse_for_block, parse_interpolation_expr |
| Consistency | Naming, API patterns, dual representations | 1 HIGH, 2 MEDIUM | 1 MEDIUM | Dot-path representation, reconstruction overhead, dead params |
| Regression | Behavioral changes, error quality | 1 HIGH, 2 MEDIUM | 0 | strip_type_mds corruption, validator type checks, parse spans |
| Testing | Test coverage, unit tests | 4 MEDIUM | 2 MEDIUM | Missing parser unit tests, stale feature knowledge |
| Reliability | Bounds, assertions, resource limits | 2 MEDIUM | 1 MEDIUM | debug_assert patterns, strip_type_mds, depth bounds |
| Rust | Ownership, borrowing, idioms | 3 MEDIUM | 1 MEDIUM | debug_assert, Vec allocation, silent key drops, dead params |

---

## Cross-Reviewer Consolidation

**High-Confidence Duplicate Issues:**
- **Debug assert + unchecked index** (architecture, reliability, rust) → 4 occurrences, 90% confidence → MUST FIX
- **Silent YAML key drops** (security, rust, reliability) → 3 occurrences, 85% confidence → MUST FIX  
- **strip_type_mds issues** (regression, performance, security) → 2 HIGH + 1 MEDIUM → MUST FIX
- **Vec allocation in MemberAccess** (performance, consistency, rust) → 3 occurrences, 85% confidence → MUST FIX
- **Lost source context in parse errors** (architecture, regression, rust) → 2 occurrences, 90% confidence → MUST FIX
- **Dot-path representation inconsistency** (consistency, complexity, rust) → Multiple occurrences → SHOULD FIX

---

## Quality Metrics

- **Test Coverage**: 325 passing tests, 26 new integration tests, strong behavioral coverage
- **Clippy**: Zero warnings
- **Code Style**: Consistent with codebase conventions
- **Architecture**: Clean layering, proper separation of concerns
- **Documentation**: Spec updates comprehensive, commit messages accurate

---

## Conclusion

The branch successfully implements three well-designed features (object/map support, frontmatter preservation, escape documentation fixes) with generally excellent code quality. The blocking issues are fixable without design changes -- mostly guarding against edge cases, fixing data corruption, and restoring diagnostic quality. Once the 7 blocking issues are resolved, this PR will be ready to merge.

**Estimated fix complexity**: 2-3 hours for a Rust-experienced developer familiar with this codebase.

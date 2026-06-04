# Code Review Summary

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16_2309

## Merge Recommendation: CHANGES_REQUESTED

**The PR cannot merge in its current form due to a single blocking consistency issue and multiple high-priority concerns across architecture, complexity, and documentation.**

The three enhancements are well-designed and thoroughly tested. The implementation is fundamentally sound with strong patterns for error handling, resource limits, and test coverage. However, the evaluator's use of panicking `assert!()` macros contradicts the established project principle of "never panic in business logic" and creates an inconsistency with the same branch's validator changes. This issue was identified by 4 independent reviewers (architecture, consistency, regression, rust) and represents a critical pattern violation.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| **Blocking Issues** | 0 | 3 | 6 | 0 | 9 |
| **Should Fix** | 0 | 0 | 5 | 0 | 5 |
| **Pre-existing** | 0 | 0 | 1 | 0 | 1 |
| **Suggestions** | 0 | 0 | 0 | 8 | 8 |

---

## Blocking Issues (Must Fix Before Merge)

### CRITICAL PATTERN VIOLATION: assert!() Panics in Production Code

**Flagged by**: Architecture, Consistency, Regression, Rust (4 reviewers, 85-95% confidence)

**Location**: `src/evaluator.rs:321,339`

**Problem**: The evaluator uses `assert!(!block.condition.is_empty())` and `assert!(!block.iterable.is_empty())` which panic in release builds. This violates the project's core principle of "never panic in business logic" and directly contradicts this same PR's validator changes (commit `72096c1`), which replaced panicking `debug_assert!+index` with `.first().ok_or_else()` "for release safety".

The validator's change explicitly states: "Use .first() with an error return rather than a debug_assert!+index so this holds in release builds too." The evaluator must follow the same pattern for the identical invariant on identical data types.

**Impact**: A future parser bug or direct AST construction in tests/internal code would crash the entire process rather than returning a recoverable error. This is inconsistent error handling within a single PR.

**Fix**:
```rust
// In evaluate_if (line 321-323):
let root = block.condition.first().ok_or_else(|| {
    MdsError::syntax("internal error: @if block has empty condition path")
})?;
let value = resolve_dot_path(root, &block.condition[1..], scope)?;

// In evaluate_for (line 339-341):
let root = block.iterable.first().ok_or_else(|| {
    MdsError::syntax("internal error: @for block has empty iterable path")
})?;
let iterable = resolve_dot_path(root, &block.iterable[1..], scope)?;
```

---

### HIGH PRIORITY: Frontmatter Logic in Wrong Architectural Layer

**Reviewer**: Architecture (82% confidence)

**Location**: `src/lib.rs:254,278` + `src/resolver.rs:43`

**Problem**: The `prepend_frontmatter()` and `strip_type_mds()` functions are defined in `lib.rs` (the public API facade) and operate on data produced by the resolver. This places output composition logic in the wrong layer. The pipeline is clearly: lexer → parser → validator → resolver → evaluator → [render], but frontmatter assembly happens in the API facade after the resolver returns, creating an implicit "render" step outside the pipeline. Additionally, `ResolvedModule` carries `raw_frontmatter` as transient rendering concern mixed with semantic results, creating coupling.

**Impact**: This violates the single-responsibility principle and makes the public API functions less thin wrappers than they should be. The logic duplicates across `compile_collecting_warnings` and `compile_str_collecting_warnings`.

**Fix**: Move `prepend_frontmatter()` and `strip_type_mds()` into the resolver's `process_module()` or into a dedicated `render.rs` module. The `ResolvedModule` could either store already-cleaned frontmatter, or return a richer struct separating semantic content from rendering metadata.

---

### HIGH PRIORITY: Incorrect Function Signature in KNOWLEDGE.md

**Reviewer**: Documentation (95% confidence)

**Location**: `.features/mds-compiler/KNOWLEDGE.md:298`

**Problem**: The documented signature is `resolve_dot_path(path: &[String], scope: &Scope)` (2 params) but the actual implementation is `resolve_dot_path(root: &str, fields: &[String], scope: &Scope)` (3 params). This mismatch will mislead developers maintaining or extending the code.

**Fix**: Update to the correct signature:
```
**`resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError>`**: 
Private function that resolves a dot-path. `root` is looked up in scope as the starting variable, 
then `fields` are traversed into `Value::Object` values.
```

---

### HIGH PRIORITY: Complexity Violations

**Reviewer**: Complexity (88-92% confidence)

**Locations**: 
- `src/evaluator.rs:333` - `evaluate_for` at 96 lines (threshold: 50)
- `src/parser.rs:501` - `parse_interpolation_expr` at 94 lines (threshold: 50)

**Problem**: Both functions exceed the 50-line complexity threshold. They handle multiple distinct logic paths with duplicated boilerplate, making them harder to review, test, and modify safely. `evaluate_for` has two iteration paths (key-value and array) with duplicated resource-limit checks and scope management. `parse_interpolation_expr` has 4 expression types with 3-level nesting in the dot-before-paren case.

**Fix**: Extract key-value iteration into `evaluate_for_key_value()` helper and dot-path handling into `parse_dot_expr()` helper. This would bring both functions under 50 lines.

---

### MEDIUM PRIORITY: Documentation Accuracy Issues (2 items)

**Issue 1 - Falsy Values Incomplete**
- **Reviewer**: Documentation (80% confidence)
- **Location**: `spec.md:92`
- **Problem**: The falsy values list was modified to add "empty object {}" but omits "NaN" (which is falsy per the implementation). KNOWLEDGE.md correctly lists it.
- **Fix**: Change to: "Falsy values: false, null, empty string "", empty array [], empty object {}, 0, NaN"

**Issue 2 - Spec Example Missing Object Variable**
- **Reviewer**: Documentation (82% confidence)  
- **Location**: `spec.md:36-43`
- **Problem**: Section 4.1 now documents object support but the frontmatter example lacks an object variable. Users reading examples first won't see how to define objects.
- **Fix**: Add object variable to example: `config: { debug: true, greeting: Hello }`

---

### MEDIUM PRIORITY: Security & Performance Conditions

**Dot-Path Segment Count** (Security, 80% confidence)
- **Location**: `src/parser.rs:216-226`, `src/parser.rs:271-281`
- **Problem**: No explicit upper bound on dot-separated segments (e.g., `@if a.b.c.d.e.f...`). While implicitly bounded by `MAX_FILE_SIZE`, defense-in-depth suggests explicit check.
- **Fix**: Add maximum segment count (e.g., 32 levels, consistent with `MAX_VALUE_DEPTH = 64`)

**resolve_dot_path Cloning** (Performance, 82% confidence)
- **Location**: `src/evaluator.rs:101-108`
- **Problem**: Clones root value and each intermediate field. For deeply nested objects, this clones full HashMaps unnecessarily.
- **Status**: Acknowledged as acceptable for v0.1 (small configs). Condition: approve with acknowledgment that clone-per-field is intentional for v0.1, or refactor to reference-based traversal.

**resolve_dot_path Depth Bound** (Reliability, 82% confidence)
- **Location**: `src/evaluator.rs:100-123`
- **Problem**: Traversal depth is implicitly bounded by `MAX_VALUE_DEPTH = 64` (values deeper than 64 cannot exist) but lacks explicit guard.
- **Fix**: Add early-exit check against `MAX_VALUE_DEPTH` for defense-in-depth and clarity.

---

## Should-Fix Issues (Recommended Before Merge)

### Testing Gaps (2 items, MEDIUM)

**Missing Test: Runtime-Supplied Objects with Dot-Access**
- **Reviewer**: Testing (82% confidence)
- **Coverage Gap**: `resolve_dot_path` when root comes from runtime vars, not YAML frontmatter
- **Fix**: Add test `runtime_vars_object_dot_access()` exercising `compile_str_with` with object vars

**Missing Test: Key-Value Iteration Over Dot-Path Objects**
- **Reviewer**: Testing (80% confidence)
- **Coverage Gap**: `@for k, v in config.settings:` (key-value iteration on nested object)
- **Fix**: Add test `for_key_value_dot_path_object()` combining both features

---

### Code Quality (Should-Fix)

**Duplicated Loop Body Pattern** (Architecture, 80% confidence)
- **Location**: `src/evaluator.rs:367-381,412-424`
- **Problem**: Key-value and array iteration branches duplicate the push/eval/pop/error-preference pattern
- **Fix**: Extract helper `fn run_loop_body(scope, ctx, body, bindings) -> Result<String, MdsError>`

**Weak Test Assertions** (Testing, 80% confidence)
- **Locations**: `tests/integration.rs:3231,3238,3351`
- **Problem**: Tests use `.contains()` instead of `assert_eq!` for full output; won't catch frontmatter mutations
- **Fix**: Use exact equality assertions in simpler test cases

**strip_type_mds Edge Cases** (Security, 82% confidence)
- **Location**: `src/lib.rs:342-361`
- **Problem**: Doesn't handle quoted YAML variants: `type: "mds"`, `type: 'mds'`, `type:mds` (no space)
- **Fix**: Extend check to handle YAML quoting styles

---

## Key Strengths

1. **Parser Invariant Enforcement** — Validator correctly uses `.first().ok_or_else()` pattern; evaluator must match this.
2. **Resource Limits** — All loops respect `MAX_LOOP_ITERATIONS` (100k) and `MAX_TOTAL_ITERATIONS` (1M); new key-value `@for` correctly enforces both.
3. **Depth Bounds** — `MAX_VALUE_DEPTH = 64` properly bounds YAML/JSON nesting; prevents pathological inputs.
4. **Comprehensive Testing** — 44 new tests (25 integration + 7 parser + 12 value) with 336 total tests passing, zero warnings.
5. **Error Propagation** — Consistent use of `?` operator and `prefer_first_error` pattern.
6. **Input Validation** — All new dot-path parsing validates segments with `is_valid_identifier`, preventing injection.

---

## Action Plan

**Must Complete Before Merge:**
1. Replace `assert!()` macros with `.first().ok_or_else()` pattern in evaluator (20 minutes)
2. Fix `resolve_dot_path` signature in KNOWLEDGE.md (5 minutes)
3. Move frontmatter logic to resolver layer or dedicated render module (1-2 hours)
4. Reduce complexity by extracting helpers in `evaluate_for` and `parse_interpolation_expr` (1 hour)
5. Update spec.md falsy values and example (10 minutes)

**Recommended Before Merge:**
6. Add explicit depth check in `resolve_dot_path` against `MAX_VALUE_DEPTH` (10 minutes)
7. Add two missing test cases (20 minutes)
8. Fix `strip_type_mds` to handle quoted YAML variants (20 minutes)

**Total Effort**: ~4-5 hours to address all blocking and should-fix items.

---

## Confidence Levels & Consensus

The **assert!() panic issue** (blocking) was flagged by 4 reviewers independently:
- Architecture: 85% confidence - pattern violation
- Consistency: 95% confidence - direct contradiction in same PR
- Regression: 82% confidence - inconsistent with validator
- Rust: 92% confidence - library code should never panic

This consensus makes it a critical must-fix.

The **frontmatter layering issue** (blocking) is an architectural concern flagged by architecture reviewer with 82% confidence. This is subjective but represents a genuine single-responsibility violation.

The **complexity issues** are objective: two functions are 88-96 lines vs. the 50-line standard, confirmed by both reviewers.


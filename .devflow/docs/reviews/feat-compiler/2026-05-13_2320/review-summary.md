# Code Review Summary

**Branch**: feat/compiler -> main  
**Date**: 2026-05-13  
**Reviewers**: 9 domain specialists (security, architecture, performance, complexity, consistency, testing, regression, reliability, rust)

---

## Merge Recommendation: APPROVED_WITH_CONDITIONS

**Summary**: This is a well-engineered greenfield compiler implementation with comprehensive test coverage (213 tests, 0 failures), zero build warnings, and clean clippy output. The architecture is sound with clear layer separation and no circular dependencies. Core security controls are solid: resource exhaustion guards, path traversal prevention, symlink rejection, and circular import detection.

However, four blocking issues must be addressed before merge:

1. **Duplicate `@define` registration** (Architecture HIGH) - Functions lose lexical closure captures during evaluation
2. **Unbounded nested loop iteration accounting** (Reliability HIGH) - Outer loops can bypass total iteration limit
3. **Error categorization inconsistency** (Consistency HIGH) - Resource limits incorrectly classified as I/O errors
4. **Public API idiom violation** (`as_array` returns `&Vec<T>` instead of `&[T]`)

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| **Blocking** (in your changes) | 0 | 4 | 9 | 1 |
| **Should-Fix** (code you touched) | 0 | 0 | 10 | 2 |
| **Pre-existing** | 0 | 0 | 0 | 0 |
| **Total** | **0** | **4** | **19** | **3** |

---

## Spec Compliance

✅ **COMPLETE**: All spec-required features are implemented and tested:
- Variables, interpolation, conditionals, loops, functions, imports, exports, includes, code blocks, escaped braces
- Error handling: undefined variables, circular imports, arity mismatches, file not found, recursion limits
- Security: file size limits, path traversal prevention, symlink rejection, import depth limits, YAML/JSON depth limits
- CLI: build, check, init, --set, --vars flags, stdin input, quiet mode

**Spec enhancements beyond v0.1 spec** (deliberate, non-blocking):
- Re-exports (`@export name from "path"`, `@export * from "path"`)
- Selective imports (`@import { fn } from "path"`)
- CLI: `--set KEY=VAL`, stdin (`-`), auto-detect single `.mds` file, `mds init`, quiet mode
- API: `compile_str`, `compile_str_with`, warning collection pattern

---

## Cross-Cutting Themes

### 1. **Excessive Deep-Cloning (Performance)**
**Appears in 6+ locations**. The compiler clones entire data structures where shared references would suffice:
- **Module cache hits** clone entire `ResolvedModule` (HIGH priority)
- **Closure captures** clone entire visible scope per `@define` (O(N²) behavior)
- **Validator scope** clones full scope to add one variable (MEDIUM)
- **Loop iterables** clone entire array before iteration (MEDIUM)

**Impact**: Scales poorly for modules with many imports/functions. For typical prompt templates (small files) impact is negligible. For production use cases with 10+ imports and 10+ functions per module, this becomes a bottleneck.

**Solution**: Adopt `Arc<>` for `FunctionDef`, `NamespaceScope`, and `ResolvedModule`. This is a structural refactor that pays dividends as language grows.

---

### 2. **Unbounded or Insufficient Resource Limits**
**Appears in 3 locations** across security, reliability, and architecture reviews:
- **Total iteration accounting** has a logic flaw where outer loops with mixed bodies (`@for` + text) go uncounted
- **`find_project_root`** loop has no explicit iteration cap (relies on filesystem structure)
- **`validate_var_args`** recurses without depth check (implicit bound via parser only)

**Impact**: Enables potential DOS by constructing deeply-nested structures or outer-loop heavy templates.

---

### 3. **Responsibility Clustering**
**Appears in 3 locations** (complexity, reliability, rust):
- **`process_module` (140 lines, 7 responsibilities)** - tokenize → parse → scope-build → frontmatter → imports → exports → evaluate
- **`tokenize` (155 lines)** - frontmatter, code fences, directives, escapes, interpolation, text all interleaved
- **`ModuleCache::resolve` (100 lines, 8 checks)** - symlink validation, canonicalization, cache lookup, cycle detection, file read, size check, UTF-8 validation, file type validation

**Fix**: Extract into named pipeline stages. This becomes increasingly important as the language grows.

---

### 4. **Duplicate Scope Capture & Validation Logic**
**Appears in 2 locations**:
- **`@define` registration** happens in both resolver (with full closure capture) and evaluator (without) - evaluator's version overwrites resolver's richer data
- **`@for` type checking** happens in both validator (cloning scope) and evaluator (full check) - code duplication with expensive deep-clone in validator

**Impact**: Functions could silently lose lexical closure captures if evaluator registration overwrites resolver registration. Validator cloning is expensive but defense-in-depth.

---

### 5. **Error Categorization Inconsistency**
**Appears in 2 locations**:
- **Resource limits classified as `MdsError::Io`** - output size, loop iterations, file size are NOT I/O errors
- **Missing `_at()` constructors** - `CircularImport` missing spanless constructor, `ExportError` missing span-aware constructor, `Recursion` missing span-aware constructor

**Impact**: Programmatic error handling (e.g., distinguishing "disk full" from "template too complex") becomes difficult.

---

## Blocking Issues

### 1. ARCHITECTURE HIGH: Evaluator overwrites resolver's lexical closure capture
**File**: `src/evaluator.rs:80-82` and `src/resolver.rs:228-248`  
**Confidence**: 82%

The resolver captures lexical scope when registering `@define` blocks (with `captured_namespaces`, `captured_functions`, `captured_vars`). The evaluator then *re-registers* the same blocks during its walk, but without the closure capture logic, overwriting the resolver's richer definition.

**Impact**: Functions could silently lose access to sibling functions and imported namespaces defined at their lexical site.

**Fix**: Have evaluator skip `@define` nodes (already handled by resolver) similar to how it skips `Import` and `Export`:
```rust
Node::Define(_) => {
    // Handled by resolver with full lexical capture, skip during evaluation
}
```

---

### 2. RELIABILITY HIGH: Total iteration accounting bypassed for non-leaf outer loops
**File**: `src/evaluator.rs:334-349`  
**Confidence**: 85%

The `is_leaf_loop` optimization only counts iterations for loops whose body contains no nested `@for` blocks. For a three-level nested loop (100 × 100 × 100), only the innermost level is counted. More critically, for a structure like:
```mds
@for a in array:      # NOT a leaf (contains @for) → 0 counted iterations for outer loop
  @for b in items:    # Leaf → 100,000 counted iterations
    text
  @end
@end
```
The outer loop's 100,000 scope operations go uncounted, potentially bypassing `MAX_TOTAL_ITERATIONS`.

**Fix**: Count all loop iterations unconditionally:
```rust
for item in items {
    *total_iterations += 1;
    if *total_iterations > MAX_TOTAL_ITERATIONS {
        return Err(...);
    }
    // ... rest of loop
}
```

---

### 3. CONSISTENCY HIGH: Resource limits misclassified as I/O errors
**File**: `src/evaluator.rs:91`, `src/evaluator.rs:321`, `src/evaluator.rs:341`, `src/resolver.rs:129`  
**Confidence**: 90%

Output size exceeded, array iteration limit exceeded, total iteration limit exceeded, and file too large are all constructed as `MdsError::Io` despite not being filesystem errors. This conflates distinct error categories.

**Fix**: Introduce `MdsError::ResourceLimit { message }` variant with diagnostic code `mds::resource_limit` and use it for all resource-bound violations.

---

### 4. RUST MEDIUM: Public API returns `&Vec<T>` instead of `&[T]`
**File**: `src/value.rs:102`  
**Confidence**: 90%

The `as_array` method returns `Option<&Vec<Value>>` instead of the idiomatic `Option<&[Value]>`. This is a public API that will be harder to change later.

**Fix**: 
```rust
pub fn as_array(&self) -> Option<&[Value]> {
    match self {
        Value::Array(a) => Some(a),
        _ => None,
    }
}
```

---

## Recommended Improvements (Should-Fix)

### PERFORMANCE Issues (5)
1. **Full ResolvedModule clone on every cache hit** (92% confidence) - Wrap in `Arc<ResolvedModule>`
2. **Closure capture clones entire scope per function** (90% confidence) - Use `Arc<FunctionDef>` for captures
3. **Redundant clone before caching** (90% confidence) - Cache first, return from cache
4. **Lexer allocates 2 full-source arrays** (85% confidence) - Operate on byte slices with UTF-8-aware iteration
5. **`get_all_exports()` creates intermediate Vec** (85% confidence) - Return iterator instead

### ARCHITECTURE Issues (4)
1. **Resolver accumulates 7 responsibilities in `process_module`** (85% confidence) - Extract into pipeline stages
2. **Shared constant defined in 2 places** (90% confidence) - Export from resolver, import in main.rs
3. **`clean_output` lives in lib.rs** (80% confidence) - Move to render utility module
4. **Validator duplicates `@for` type checking** (80% confidence) - Document intentional duplication as defense-in-depth

### COMPLEXITY Issues (4)
1. **`process_module` (140 lines, cyclomatic ~15)** (90% confidence) - Extract into named stages
2. **`tokenize` (155 lines, 5-level nesting)** (92% confidence) - Extract handlers into Lexer methods
3. **`parse_interpolation_expr` (89 lines)** (85% confidence) - Extract dot-notation branch
4. **Parameter convoy (5-6 params per evaluator function)** (83% confidence) - Bundle into `EvalCtx` struct

### TESTING Issues (2)
1. **No unit tests for error.rs** (82% confidence) - Add unit tests for constructors and span logic
2. **No unit tests for resolver.rs** (83% confidence) - Add unit tests for validation helpers

### RELIABILITY Issues (2)
1. **`.expect()` in Scope methods can panic** (85% confidence) - Use `debug_assert!` + fallback
2. **`find_project_root` has no explicit iteration bound** (80% confidence) - Add cap at 256 levels

### RUST Issues (1)
1. **`is_truthy` missing `#[must_use]`** (85% confidence) - Add attribute

### CONSISTENCY Issues (3)
1. **Inconsistent error construction (direct variant vs constructor)** (82% confidence) - Use constructors consistently
2. **Token enum uses unnamed tuple fields** (80% confidence) - Convert to named fields for clarity
3. **Missing constructors for error variants** (85% confidence) - Add `circular_import()` and `export_error_at()`

---

## Reviewer Scores & Recommendations

| Reviewer | Focus | Score | Issues | Recommendation |
|----------|-------|-------|--------|-----------------|
| Security | Input validation, path traversal, resource exhaustion | 8/10 | 2 MEDIUM blocking, 3 should-fix | APPROVED_WITH_COMMENTS |
| Architecture | Layer separation, dependencies, SRP | 7/10 | 2 HIGH blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Performance | Allocation, cloning, caching | 5/10 | 4 HIGH blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Complexity | Function size, nesting, cyclomatic complexity | 6/10 | 4 HIGH blocking, 1 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Consistency | Naming, error categories, constructor patterns | 8/10 | 1 HIGH blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Testing | Coverage, assertion strength, edge cases | 8/10 | 2 MEDIUM blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Regression | Behavioral equivalence, refactoring safety | 10/10 | 0 | APPROVED |
| Reliability | Bounded loops, recursion limits, panic safety | 7/10 | 1 HIGH blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Rust | Idioms, API design, safety, warnings | 8/10 | 1 MEDIUM blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |

---

## Action Plan

### Before Merge (BLOCKING)
1. **Evaluator**: Skip `@define` nodes (already handled by resolver with closure capture)
2. **Evaluator**: Count all loop iterations unconditionally (fix `is_leaf_loop` logic)
3. **Error**: Add `ResourceLimit` variant; use for output size, iteration, file size errors
4. **Value API**: Change `as_array()` return type to `Option<&[Value]>`

### Post-Merge Priority (HIGH)
1. **Performance refactor**: Wrap `FunctionDef`, `NamespaceScope`, `ResolvedModule` in `Arc<>`
2. **Complexity refactor**: Extract `process_module` into named pipeline stages
3. **Complexity refactor**: Extract `tokenize` handlers into Lexer struct methods

### Secondary (MEDIUM, next sprint)
- Consistency: Fix error constructor patterns and missing `_at` variants
- Performance: Return iterators instead of collecting temporary Vecs
- Testing: Add unit tests for error.rs and resolver.rs modules
- Reliability: Add explicit iteration bounds to `find_project_root` and `validate_var_args`

---

## Strengths

✅ **Comprehensive test coverage** - 213 tests (56 unit, 144 integration, 13 doc tests) with zero failures  
✅ **Clean compiler output** - 0 warnings on `cargo build` and `cargo clippy`  
✅ **No unsafe code** - All unsafe operations avoided; no panics in business logic except bounded `.expect()`  
✅ **Strong security posture** - 7 separate resource limits, path traversal prevention, symlink rejection  
✅ **Sound architecture** - Clean layer separation, no circular dependencies, clear module responsibilities  
✅ **No regressions** - All 20 commits verified for behavioral equivalence; refactors are safe  
✅ **Spec complete** - All required features implemented and tested; deliberate extensions documented  

---

## Summary

This is production-ready compiler code with excellent test coverage and security fundamentals. The four blocking issues are correctness/safety concerns that must be fixed before merge, but are straightforward to address (each is a small, localized fix). The "should-fix" items are important for performance and maintainability at scale but do not prevent merge if timelines are constrained -- they can be addressed in a post-merge sprint.

The deep-cloning pattern and responsibility clustering will become pain points as the language grows (new directives, new expression types), but the current implementation is sound and the refactoring path is clear.

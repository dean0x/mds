# Code Review Summary

**Branch**: feat/14-error-serialization-dependency-graph -> main
**Date**: 2026-05-19_0002
**Reviewers**: 9 (security, architecture, performance, complexity, consistency, regression, testing, reliability, rust)

## Merge Recommendation: CHANGES_REQUESTED

**Rationale**: One HIGH issue in blocking changes (missing test for NativeFs path) plus three MEDIUM issues blocking on consistency/design concerns. The architectural inconsistency in entry-key exclusion across `_with_deps` functions should be unified before merge to prevent fragility. The missing integration test for `compile_with_deps` with real files is necessary to validate the distinct `split_last()` logic.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 1 | 3 | 0 | **4** |
| Should Fix | 0 | 0 | 3 | 0 | **3** |
| Pre-existing | 0 | 0 | 0 | 0 | **0** |
| **Total** | **0** | **1** | **6** | **0** | **7** |

---

## Blocking Issues (Must Fix)

### 🔴 HIGH: Missing integration test for `compile_with_deps` with real file imports

**File**: `crates/mds-core/tests/api_surface.rs:277`
**Confidence**: 85%
**Reviewer**: testing
**Category**: Blocking (new test code)

**Problem**: `compile_with_deps_exists` only checks that the function exists; it does not validate behavior with actual file imports. The NativeFs-backed `compile_with_deps` uses a distinct entry-key exclusion mechanism (`split_last()` on canonicalized paths from a real filesystem) compared to `compile_virtual_with_deps` (string-equality filter). This distinct code path has zero behavioral test coverage.

**Impact**: If the `split_last()` assumption (that post-order DFS always inserts the entry module last) ever diverges from reality, the test would not catch the silent failure to exclude the entry key from the dependency list.

**Fix**: Add an integration test that creates a tempdir with two `.mds` files, compiles via `compile_with_deps`, and verifies the dependency list is correct:

```rust
#[test]
fn compile_with_deps_native_two_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let lib = dir.path().join("lib.mds");
    std::fs::write(&lib, "@define greet(x):\nHello {x}!\n@end\n").unwrap();
    let main = dir.path().join("main.mds");
    std::fs::write(&main, "@import \"./lib.mds\"\n{greet(\"World\")}\n").unwrap();

    let result = mds::compile_with_deps(&main, None).expect("should compile");
    assert!(result.output.contains("Hello World!"));
    assert_eq!(result.dependencies.len(), 1);
    assert!(result.dependencies[0].contains("lib.mds"));
    // Entry file must NOT appear in dependencies
    assert!(!result.dependencies.iter().any(|d| d.contains("main.mds")));
}
```

---

### 🔴 MEDIUM: Inconsistent entry-key exclusion strategy across `compile_*_with_deps` functions

**Files**: 
- `lib.rs:530-534` (`compile_with_deps`)
- `lib.rs:573` (`compile_str_with_deps`)
- `lib.rs:610` (`compile_virtual_with_deps`)

**Confidence**: 87% (flagged by architecture, consistency, and rust reviewers)
**Reviewers**: architecture (85%), consistency (90%), rust (82%)
**Category**: Blocking (inconsistent design in public API)

**Problem**: The three `compile_*_with_deps` functions use three different strategies to exclude the entry module from dependencies:
1. `compile_with_deps`: Uses `split_last()` on the Vec, relying on post-order DFS insertion order invariant
2. `compile_str_with_deps`: Skips filtering entirely because `resolve_source` does not cache the inline source
3. `compile_virtual_with_deps`: Uses `.filter(|k| k != entry)` for explicit key-based exclusion

The `split_last()` approach couples the public API to an internal cache ordering invariant. If `IndexMap` insertion order changes (e.g., due to optimization in `resolve_by_key`), `split_last()` would silently exclude the wrong dependency.

**Impact**: Three different semantic implementations of the same operation (exclude entry from deps) increases maintenance burden and fragility. The code is correct today but vulnerable to silent breakage during refactoring.

**Fix**: Unify on a single, explicit key-based exclusion strategy. Recommended approach for `compile_with_deps`:

```rust
pub fn compile_with_deps(
    path: impl AsRef<Path>,
    runtime_vars: Option<HashMap<String, Value>>,
) -> Result<CompileOutput, MdsError> {
    let path = path.as_ref();
    let vars = runtime_vars.unwrap_or_default();
    let mut cache = ModuleCache::new();
    let mut warnings = vec![];
    let resolved = cache.resolve_path(path, &vars, &mut warnings)?;
    let output = build_output(&resolved);
    let deps = cache.dependencies();
    // Use the last key (entry) for value-based filtering, consistent
    // with compile_virtual_with_deps.
    let entry_key = deps.last().cloned();
    let dependencies = deps.into_iter()
        .filter(|k| entry_key.as_ref() != Some(k))
        .collect();
    Ok(CompileOutput { output, warnings, dependencies })
}
```

This makes the entry-key filtering explicit, key-based (not position-based), and consistent across all three variants.

---

### 🔴 MEDIUM: `error.rs` exceeds critical file length threshold (1077 lines)

**File**: `error.rs:1-1077`
**Confidence**: 85%
**Reviewer**: complexity
**Category**: Blocking (code organization)

**Problem**: The file now contains MdsError enum definition (227 lines), 27 constructor methods (290 lines), serialize() method (43 lines), and 500+ lines of tests. At 1077 lines total, it exceeds the "Critical" complexity threshold (>500 lines). The test module alone is over 500 lines, pushing the source file to an unmaintainable size.

**Impact**: Large files reduce readability, increase cognitive load during reviews, and make locating definitions harder. The test module should live in a dedicated file.

**Fix**: Move the `#[cfg(test)] mod tests` block to a separate integration test file or use `#[path = "error_tests.rs"] mod tests;` to split the module while keeping `pub(crate)` access:

```rust
// At the end of error.rs
#[path = "error_tests.rs"]
#[cfg(test)]
mod tests;
```

This would bring the source file to ~570 lines, still above threshold but much more manageable.

---

### 🔴 MEDIUM: `#[must_use]` message style inconsistency

**Files**: 
- `lib.rs:102` (existing `compile` function)
- `lib.rs:516` (new `compile_with_deps` function)

**Confidence**: 92%
**Reviewer**: consistency
**Category**: Blocking (API surface consistency)

**Problem**: The existing `compile` function uses `#[must_use = "the compiled Markdown output should be used"]` (type-focused), while the new `compile_with_deps` uses `#[must_use = "the compiled output, warnings, and dependencies should be used"]` (content-focused). The commit message (4d2f097) explicitly states the change was intentional, but only the three new `_with_deps` functions follow the new style while the 16 existing functions still use the old style.

**Impact**: Two coexisting `#[must_use]` message styles in the same API surface creates cognitive inconsistency and inconsistent guidance to callers.

**Fix**: Choose one style for the entire public API surface. Either:
- *Option A* (forward-going): Keep new functions with content-focused style, update existing functions in a separate PR
- *Option B* (consistent now): Revert new functions to type-focused style to match existing functions

**Recommendation**: Option B is preferred for consistency. If content-focused messages are desired, update all functions together in a separate architectural pass.

---

## Should-Fix Issues (Recommended)

### ⚠️ MEDIUM: `CompileOutput.warnings` field never tested with actual warning content

**Files**: 
- `crates/mds-core/tests/api_surface.rs:252`
- `crates/mds-core/tests/virtual_fs.rs:294-417`

**Confidence**: 82%
**Reviewer**: testing
**Category**: Should-Fix (test coverage gap)

**Problem**: All `_with_deps` tests assert synthetic warning constructions or use inputs that produce no warnings. No test verifies that `CompileOutput.warnings` captures real compiler warnings from the compilation pipeline. If warning-threading from `process_module` to `CompileOutput` were broken, no test would catch it.

**Impact**: The new `CompileOutput.warnings` field is untested against real compiler warnings, creating a coverage gap in the new public API.

**Fix**: Create a test case that triggers a real compiler warning and asserts it appears in `result.warnings`. If no warning is easily triggerable via `compile_virtual_with_deps`, document the gap.

---

### ⚠️ MEDIUM: `compile_str_with_deps` with file imports is untested

**File**: `crates/mds-core/tests/virtual_fs.rs:387-401`
**Confidence**: 80%
**Reviewer**: testing
**Category**: Should-Fix (test coverage gap)

**Problem**: The test comment on line 389-391 explicitly acknowledges this gap: "compile_str_with_deps: inline source that imports a virtual module. Use a base_dir so the import resolution works; but with NativeFs that would look for real files. Skip this variant here -- covered in api_surface.rs." However, api_surface.rs also only tests the no-import case. The `compile_str_with_deps` function with imports (where `base_dir` matters and `dependencies` should be populated) has zero test coverage.

**Impact**: One of three `_with_deps` variants lacks any test that exercises import resolution.

**Fix**: Add a test using tempdir that creates a lib file, then calls `compile_str_with_deps` with a source string containing `@import "./lib.mds"` and `base_dir` pointing to the tempdir:

```rust
#[test]
fn compile_str_with_deps_with_import() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("lib.mds"),
        "@define greet(x):\nHello {x}!\n@end\n",
    ).unwrap();
    let result = mds::compile_str_with_deps(
        "@import \"./lib.mds\"\n{greet(\"World\")}\n",
        Some(dir.path()),
        None,
    ).expect("should compile");
    assert!(result.output.contains("Hello World!"));
    assert!(!result.dependencies.is_empty());
}
```

---

### ⚠️ MEDIUM: Dependency order assertion comment contradicts actual test

**File**: `crates/mds-core/tests/virtual_fs.rs:328-345`
**Confidence**: 80%
**Reviewer**: testing
**Category**: Should-Fix (documentation clarity)

**Problem**: The test comment says "deps of a = ['b.mds', 'c.mds'] in DFS order" but the assertion checks `["c.mds", "b.mds"]` (post-order DFS). The comment text contradicts the assertion. While the assertion itself is correct (matching actual post-order DFS), the comment is misleading and could cause confusion during maintenance.

**Impact**: Incorrect documentation within a test reduces confidence in the assertion's intent.

**Fix**: Update the comment on line 328 from:
```
deps of a = ["b.mds", "c.mds"] in DFS order
```
to:
```
deps of a = ["c.mds", "b.mds"] in post-order DFS
```

---

## Score Breakdown by Reviewer

| Reviewer | Score | Issues Found | Recommendation |
|----------|-------|--------------|-----------------|
| Security | 9/10 | 0 blocking, 0 should-fix | APPROVED |
| Architecture | 8/10 | 1 HIGH blocking | APPROVED_WITH_CONDITIONS |
| Performance | 9/10 | 0 blocking, 0 should-fix | APPROVED |
| Complexity | 7/10 | 0 blocking, 2 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Consistency | 8/10 | 1 MEDIUM blocking, 1 MEDIUM should-fix | APPROVED_WITH_CONDITIONS |
| Regression | 9/10 | 0 blocking, 0 should-fix | APPROVED |
| Testing | 7/10 | 1 HIGH blocking, 2 MEDIUM should-fix | CHANGES_REQUESTED |
| Reliability | 9/10 | 0 blocking, 0 should-fix | APPROVED |
| Rust | 9/10 | 1 MEDIUM blocking, 0 should-fix | APPROVED_WITH_CONDITIONS |

---

## Merge Readiness Assessment

**Current Status**: ❌ Not Ready

**Blocking Path to Merge**:
1. **Fix inconsistent entry-key exclusion** (MEDIUM, blocking on design) — Unify `compile_with_deps`, `compile_str_with_deps`, and `compile_virtual_with_deps` on a single key-based exclusion strategy
2. **Add integration test for NativeFs `compile_with_deps`** (HIGH, blocking on test coverage) — Test real file imports to validate `split_last()` logic
3. **Resolve `#[must_use]` message inconsistency** (MEDIUM, blocking on API consistency) — Standardize to either type-focused or content-focused style across all public functions
4. **Split error.rs test module** (MEDIUM, blocking on file organization) — Move 500+ lines of tests to dedicated file to bring source under 1000 lines

**Recommended Follow-up** (not blocking):
- Add test for `compile_str_with_deps` with file imports
- Add test for `CompileOutput.warnings` with real warning content
- Fix comment/assertion alignment in `deps_three_file_chain` test

---

## Key Strengths

- **Security**: Strong posture. Path traversal defenses intact, no information disclosure, resource limits properly maintained, no deserialization surface.
- **Architecture**: Well-layered. Drift-proof serialization using `miette::Diagnostic` trait methods. Trait-based canonicalization correctly extends abstraction. IndexMap choice is correct for ordered dependency tracking.
- **Regression Testing**: Comprehensive. All 14 existing functions retain signatures. No exports removed. FileSystem trait backward-compatible with default implementation.
- **Reliability**: Excellent. All loops bounded, assertions dense, allocation disciplined, indirection minimal. Resource cleanup is correct.
- **Rust idioms**: Well-written. Proper use of `thiserror` + `miette`, exhaustive matches, `#[non_exhaustive]` on enums, all tests pass, Clippy clean.

---

## Action Plan

1. **Before re-review**: Implement all four blocking fixes (entry-key unification, NativeFs test, `#[must_use]` consistency, error.rs split)
2. **Before merge**: Address should-fix issues or document gaps (warning test, import test, comment fix)
3. **After merge**: Consider architectural passes to reduce API surface width (builder pattern) and file sizes

---

## Detailed Issue References

For full context on each issue, see the individual reviewer reports:
- **security.md** — Path traversal, error disclosure, resource limits, deserialization surface
- **architecture.md** — Entry-key inconsistency, CompileOutput placement, code duplication
- **performance.md** — compute_line_column efficiency, dependencies() allocation
- **complexity.md** — Constructor pairs, file sizes, API surface width
- **consistency.md** — Entry-key strategies, `#[must_use]` styles, derive consistency
- **regression.md** — Full regression checklist passed
- **testing.md** — NativeFs test gap, warning content gap, import test gap
- **reliability.md** — Bounded loops, assertions, allocation, no issues found
- **rust.md** — Entry-key fragility, CRLF handling, diagnostic code unwrap


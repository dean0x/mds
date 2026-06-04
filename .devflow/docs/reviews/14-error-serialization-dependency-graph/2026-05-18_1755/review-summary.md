# Code Review Summary

**Branch**: HEAD -> main
**PR**: #22 — SerializedError/SerializedSpan, CompileOutput with dependency tracking, FileSystem::canonicalize()
**Date**: 2026-05-18
**Timestamp**: 2026-05-18_1755

## Merge Recommendation: **CHANGES_REQUESTED**

### Summary

This PR demonstrates strong architectural judgment and Rust idioms but contains several blocking and should-fix issues that must be resolved before merge. The most critical issue — the `compile_with_deps` bypass of the `FileSystem::canonicalize()` trait — appears in 6 of 9 reviews and directly undermines the PR's stated goal of fixing issue #21. Additionally, missing test coverage for 4 error variants and 1 missing `# Examples` doc section create consistency gaps.

The core implementation (serialization, dependency tracking, IndexMap swap) is sound. All existing tests pass. Fixing the identified issues is straightforward.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| **Blocking** | 0 | 2 | 4 | 0 | **6** |
| **Should Fix** | 0 | 0 | 2 | 0 | **2** |
| **Pre-existing** | 0 | 0 | 0 | 0 | **0** |
| **Suggestions** | - | - | - | 8 | **8** |

---

## Blocking Issues (Must Fix)

### Issue 1: `compile_with_deps` bypasses FileSystem::canonicalize() trait
**Flagged by**: Security, Architecture, Complexity, Consistency, Reliability, Rust (6/9)
**Severity**: HIGH
**Confidence**: 85% (avg)
**Location**: `crates/mds-core/src/lib.rs:521-524`

**Problem**:
`compile_with_deps` calls `std::path::Path::canonicalize()` directly to compute the entry key for filtering, while `resolve_source` was specifically updated in this PR to use `self.fs.canonicalize()` (fixing issue #21). This creates an inconsistency:

- The PR explicitly added `FileSystem::canonicalize()` to abstract path resolution
- `resolve_source` correctly routes through the trait
- `compile_with_deps` bypasses it, using `std::fs` directly

If a custom `FileSystem` implementation overrides `canonicalize()` with different semantics, the entry-key filtering will diverge from the resolver's normalization, causing the entry module to incorrectly appear in the `dependencies` list (correctness bug, not a crash).

**Fix**:
Route through the cache's filesystem. Two options:

```rust
// Option A: Expose canonicalize_key on ModuleCache
let entry_key = cache.canonicalize_key(&path.display().to_string())?;
let dependencies = cache.dependencies().into_iter().filter(|k| k != &entry_key).collect();

// Option B: Use insertion order — entry is last key after resolve_path
let all_deps = cache.dependencies();
let entry_key = all_deps.last().cloned().unwrap_or_default();
let dependencies = all_deps.into_iter().filter(|k| k != &entry_key).collect();
```

---

### Issue 2: `resolve_source` parameter type couples to OS semantics
**Flagged by**: Architecture
**Severity**: HIGH
**Confidence**: 85%
**Location**: `crates/mds-core/src/resolver.rs:238`

**Problem**:
`resolve_source` accepts `base_dir: &Path` (an OS-specific type) and immediately converts it to a string via `base_dir.display().to_string()` to pass through the `FileSystem` abstraction. This violates the Interface Segregation Principle: callers using custom `FileSystem` implementations must construct a `std::path::Path` from a string, only for it to be immediately converted back.

**Fix**:
Accept `base_dir: &str` instead to align with the string-key model used by the rest of the trait:

```rust
pub fn resolve_source(
    &mut self,
    source: &str,
    base_dir: &str,  // Changed from &Path
    runtime_vars: &HashMap<String, Value>,
    warnings: &mut Vec<String>,
) -> Result<Arc<ResolvedModule>, MdsError> {
    let canonical_str = self.fs.canonicalize(base_dir)?;
    self.fs.set_root(&canonical_str)?;
    // ...
}
```

This is a breaking change to a `pub` method, but the `#[non_exhaustive]` enum and pre-1.0 versioning indicate the API is still stabilizing.

---

### Issue 3: Missing test coverage for 4 MdsError variants in serialize()
**Flagged by**: Testing
**Severity**: HIGH
**Confidence**: 89% (avg)
**Location**: `crates/mds-core/src/error.rs:528`

**Problem**:
The `serialize()` method explicitly matches all 11 span-bearing variants. Tests exist for only 8 of these 11 variants:

**Missing**:
- `UndefinedFunction`
- `ImportError`
- `NameCollision`
- `ExportError`

Each variant carries distinct `code` and `help` attributes derived from `miette::Diagnostic`, so variant-specific assertions are required. The PR description explicitly states coverage of "all 16 error variants in serialize()" as a goal.

**Fix**:
Add 4 tests following the existing pattern (see testing report for exact code).

---

### Issue 4: NativeFs::canonicalize() follows symlinks without rejection
**Flagged by**: Security
**Severity**: HIGH
**Confidence**: 85%
**Location**: `crates/mds-core/src/fs.rs:343-348`

**Problem**:
The new `NativeFs::canonicalize()` calls `std::fs::canonicalize()` which silently resolves symlinks. This is used in `resolve_source()` to canonicalize the `base_dir`. If `base_dir` is a symlink pointing outside the project root, the resolved path becomes the symlink target, and `set_root()` then establishes that target as the trusted root. Subsequent imports would be checked against the symlink-target root, not the original project directory — potentially allowing access to files outside the intended project boundary.

In contrast, the existing `normalize()` path uses `check_symlink()` which explicitly detects and rejects symlinks.

The practical risk is bounded because `resolve_source()` is only called for `compile_str_with` / `check_str_with` (the caller controls `base_dir`), but the inconsistency with the existing symlink-rejection policy warrants a fix before merge.

**Fix**:
Apply symlink detection to `canonicalize()` to mirror the protection in `check_symlink()`:

```rust
fn canonicalize(&self, path: &str) -> Result<String, MdsError> {
    let p = Path::new(path);
    // Reject if path itself is a symlink (mirrors check_symlink logic)
    let meta = std::fs::symlink_metadata(p)
        .map_err(|e| MdsError::io(format!("cannot stat {path}: {e}")))?;
    if meta.file_type().is_symlink() {
        return Err(MdsError::import_error(format!(
            "symlinks are not allowed: {path}"
        )));
    }
    p.canonicalize()
        .map(|p| p.display().to_string())
        .map_err(|e| MdsError::io(format!("cannot resolve path {path}: {e}")))
}
```

---

### Issue 5: Missing # Examples sections on _with_deps functions
**Flagged by**: Consistency
**Severity**: MEDIUM
**Confidence**: 90%
**Location**: `crates/mds-core/src/lib.rs:498-504, 529-536, 555-562`

**Problem**:
Every existing public function in `lib.rs` includes a `# Examples` section in its doc comment (15 total). All three new `_with_deps` functions lack this section. This is a documentation style inconsistency that violates the established pattern.

**Fix**:
Add `# Examples` sections following the established pattern (see consistency report for exact code).

---

### Issue 6: Inconsistent naming: _with_deps vs _collecting_warnings
**Flagged by**: Consistency
**Severity**: MEDIUM
**Confidence**: 85%
**Location**: `crates/mds-core/src/lib.rs:506, 538, 564`

**Problem**:
The existing API surface establishes the naming convention `{base_fn}_collecting_warnings` (e.g., `compile_collecting_warnings`, `compile_str_collecting_warnings`, `compile_virtual_collecting_warnings`). The new functions use `_with_deps` (e.g., `compile_with_deps`, `compile_str_with_deps`, `compile_virtual_with_deps`). This creates a bifurcated naming idiom.

**Fix**:
Choose one approach:

1. **Rename to match existing pattern**: `compile_collecting_deps`, `compile_str_collecting_deps`, `compile_virtual_collecting_deps`
2. **Document the intentional divergence**: Accept `_with_deps` as justified by the fact that these functions return a `CompileOutput` struct rather than a tuple, making them semantically different. Add a module-level doc note explaining this.

---

## Should-Fix Issues (Recommended Improvements)

### Issue 7: Missing serialize() edge case test: span without src
**Flagged by**: Testing
**Severity**: MEDIUM
**Confidence**: 85%
**Location**: `crates/mds-core/src/error.rs:549-560`

**Problem**:
The `serialize()` doc comment explicitly describes the behavior when "span is Some but src is None" (line/column should be None while offset/length are populated). No test covers this documented edge case.

**Fix**:
Add test (see testing report for exact code).

---

### Issue 8: Missing compute_line_column boundary test
**Flagged by**: Testing
**Severity**: MEDIUM
**Confidence**: 82%
**Location**: `crates/mds-core/src/error.rs:40-55`

**Problem**:
The boundary condition is `offset > source.len()` returns None, meaning `offset == source.len()` is explicitly valid (zero-width span at EOF). This off-by-one boundary is tested for `offset > len` and `offset == 0`, but not for `offset == source.len()`.

**Fix**:
Add test:
```rust
#[test]
fn line_col_at_end_of_source() {
    // offset == source.len() is valid (zero-width span at EOF).
    assert_eq!(compute_line_column("abc", 3), Some((1, 4)));
}
```

---

## Additional Notes

### Issue 9: #[must_use] message inconsistency
**Flagged by**: Consistency
**Severity**: MEDIUM
**Confidence**: 82%
**Location**: `crates/mds-core/src/lib.rs:505, 537, 563`

**Problem**:
The existing `#[must_use]` messages describe content: "the compiled Markdown output should be used", "the compiled Markdown output and warnings should be used". The new functions use "the CompileOutput should be used" which references the type name instead.

**Fix**:
Change to: "the compiled output, warnings, and dependencies should be used"

---

## Pre-Existing Issues Found

**None**. All flagged issues are in changed or newly added code.

---

## Positive Highlights

1. **Error serialization is well-designed**: The `SerializedError`/`SerializedSpan` types are clean, follow "convert at the boundary" pattern via `MdsError::serialize()`, and use `miette::Diagnostic` for drift-proof code extraction.

2. **FileSystem abstraction intent is sound**: The fix for issue #21 correctly routes `resolve_source` through the trait (except for the one bypass identified). The default `canonicalize()` implementation (identity for virtual FS) is sensible.

3. **IndexMap swap is well-motivated**: Changing from `HashMap` to `IndexMap` for `modules` adds ordered dependency tracking with negligible overhead (already a workspace dependency via `IndexSet`).

4. **Strong Rust practices**:
   - Proper `Result` types with `?` propagation throughout
   - No `unsafe`, `panic!`, or `expect()` in production code
   - `#[must_use]` on all three new functions
   - Clean ownership patterns: borrows where possible, `Arc` for shared data

5. **Comprehensive regression testing**: 253 tests pass. Explicit regression tests verify existing function signatures are preserved and that `compile_with_deps` output matches `compile_virtual`.

6. **Dependency graph coverage**: Tests cover single file, two-file import, three-file chain, diamond dependency (deduplication), and error propagation.

---

## Reviewer Score Summary

| Reviewer | Focus | Score | Recommendation | Key Concern |
|----------|-------|-------|-----------------|-------------|
| Security | Symlinks, secrets, boundaries | 8/10 | CHANGES_REQUESTED | Symlink bypass in canonicalize(); compile_with_deps abstraction leak |
| Architecture | DIP, layer boundaries, coupling | 8/10 | APPROVED_WITH_CONDITIONS | &Path parameter couples to OS; compile_with_deps bypass |
| Performance | Allocations, I/O, algorithms | 8/10 | APPROVED_WITH_CONDITIONS | dependencies() clones all keys (minor, acceptable) |
| Complexity | Cyclomatic, nesting, API surface | 7/10 | APPROVED_WITH_CONDITIONS | API surface combinatorial growth (pre-existing pattern); path.canonicalize() abstraction leak |
| Consistency | Naming, patterns, documentation | 7/10 | CHANGES_REQUESTED | _with_deps naming divergence; missing # Examples; #[must_use] message inconsistency |
| Regression | API compatibility, behavior preservation | 9/10 | APPROVED | IndexMap swap clean; resolve_source behavior preserved; no breaking changes detected |
| Testing | Coverage, edge cases, assertions | 7/10 | CHANGES_REQUESTED | Missing 4 variant tests; missing edge case tests |
| Reliability | Bounds, panics, allocation limits | 8/10 | CHANGES_REQUESTED | compile_with_deps abstraction inconsistency; allocation patterns acceptable |
| Rust | Idioms, ownership, safety | 8/10 | APPROVED_WITH_CONDITIONS | compile_with_deps bypass is consistency gap, not safety issue |

**Average Score**: 7.8/10

---

## Action Plan (Priority Order)

1. **Fix Issue 1** (HIGH, 6/9 reviewers): Route `compile_with_deps` canonicalization through `FileSystem` trait
2. **Fix Issue 4** (HIGH, security): Add symlink rejection to `NativeFs::canonicalize()`
3. **Fix Issue 3** (HIGH): Add missing serialize() tests for 4 variants + 2 edge cases
4. **Fix Issue 2** (HIGH): Change `resolve_source` parameter from `&Path` to `&str`
5. **Fix Issue 5** (MEDIUM): Add `# Examples` doc sections to all three `_with_deps` functions
6. **Fix Issue 6** (MEDIUM): Choose and apply consistent naming pattern (or document divergence)
7. **Fix Issue 9** (MEDIUM): Align `#[must_use]` messages with existing style

**Estimated effort**: 2-3 hours for all fixes.

---

## Merge Decision

**DO NOT MERGE** until blocking issues are resolved. All fixes are straightforward additions/adjustments with clear guidance. No architectural rework required. Estimated time to fix and re-review: 1-2 days.

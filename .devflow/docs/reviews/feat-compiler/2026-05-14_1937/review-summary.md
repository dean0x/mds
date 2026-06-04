# Code Review Summary

**Branch**: feat/compiler -> main
**Date**: 2026-05-14_1937
**Reviewers**: 11 (security, architecture, performance, complexity, consistency, regression, testing, reliability, rust, dependencies, documentation)

## Merge Recommendation: CHANGES_REQUESTED

This PR represents a significant architectural improvement across the compiler codebase (2475+ lines added/887 removed) with strong refactoring of the resolver, evaluator, and lexer. The overall quality is high (average score: 7.8/10), but **three HIGH-severity issues block approval**. All three relate to the same root cause: **file I/O performed before cache lookup**, compounded by **missing security guard on config file reads**. These are straightforward fixes that will unblock the PR.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| **Blocking** | 0 | 3 | 4 | 0 | **7** |
| **Should Fix** | 0 | 0 | 5 | 0 | **5** |
| **Pre-existing** | 0 | 0 | 5 | 1 | **6** |

---

## Blocking Issues (Must Fix Before Merge)

### 1. **File I/O Before Cache Check in `resolve()` — HIGH**

**Location**: `src/resolver.rs:162`
**Confidence**: 92% (flagged by 4 reviewers: performance, reliability, rust, consistency)

**Problem**: 
`validate_and_read_file(path)` is called unconditionally on line 162, performing:
- Two `canonicalize()` syscalls
- Full symlink detection
- File read (up to 10 MB)
- UTF-8 validation

Only *after* all this I/O does the code check the module cache on line 165. On cache hits, the file content is read and discarded. This is a regression — the previous code checked the cache immediately after `canonicalize()`, before the read.

**Impact**: 
For projects with many imports referencing the same module, cache hits now trigger O(file-size) I/O instead of O(1) lookup. In a project with 100 imports of the same module, the file is read 99 times unnecessarily.

**Fix** (straightforward):
Split `validate_and_read_file` into two phases:

```rust
pub fn resolve(&mut self, path: &Path, ...) -> Result<Arc<ResolvedModule>, MdsError> {
    // 1. Canonicalize + security checks (no file read yet)
    let canonical = self.canonicalize_and_check_security(path)?;

    // 2. Check cache BEFORE reading file
    if let Some(cached) = self.modules.get(&canonical) {
        return Ok(Arc::clone(cached));
    }

    // 3. Check for circular imports
    if self.resolving.contains(&canonical) { ... }

    // 4. Now read the file
    let (source, is_md) = self.read_validated_file(&canonical)?;
    // ... rest of resolution
}
```

The helper methods can remain mostly unchanged; just reorder the resolve logic to cache-check before file-read.

---

### 2. **`load_config` Lacks File Size Guard — HIGH**

**Location**: `src/main.rs:51`
**Confidence**: 82% (flagged by 3 reviewers: security, rust, consistency)

**Problem**:
`load_config` calls `std::fs::read_to_string(&candidate)` without any size check. The rest of the codebase consistently enforces `MAX_FILE_SIZE` guards:
- `validate_and_read_file` checks before reading `.mds` files
- `load_vars_file` checks before reading vars JSON

A maliciously large `mds.json` (or symlinked to `/dev/zero` on Unix) could cause unbounded memory allocation before the JSON parser runs. While `mds.json` is a local config file (limiting exposure), the pattern violation is inconsistent with security discipline established elsewhere.

**Impact**:
- Low risk for typical users (configs are usually small)
- Pattern violation: all other file reads have guards

**Fix** (from security review, applied consistently):

```rust
let bytes = std::fs::read(&candidate).map_err(|e| {
    miette::miette!("cannot read {}: {e}", candidate.display())
})?;
if bytes.len() > 1_048_576 {  // 1 MB — generous for a config file
    return Err(miette::miette!(
        "mds.json too large ({} bytes): {}",
        bytes.len(),
        candidate.display()
    ));
}
let raw = String::from_utf8(bytes).map_err(|e| {
    miette::miette!("invalid UTF-8 in {}: {e}", candidate.display())
})?;
```

---

### 3. **`mds.json` `output_dir` Lacks Path Traversal Validation — MEDIUM (HIGH severity)**

**Location**: `src/main.rs:136`
**Confidence**: 85% (flagged by security review)

**Problem**:
The `output_dir` field from `mds.json` is joined directly to `config_dir` without path traversal validation:

```rust
let dir = config_dir.join(output_dir);
```

A malicious or misconfigured `mds.json` with `"output_dir": "../../sensitive_dir"` would direct output outside the project boundary. Import paths have multiple layers of defense (`validate_import_path`, `root_dir` boundary checks), but the output path has none.

**Impact**:
An attacker could craft a `mds.json` in a parent directory to cause the compiler to write files outside the intended project, potentially overwriting sensitive files if permissions allow.

**Fix**:

```rust
let dir = config_dir.join(output_dir);
let canonical_dir = dir.canonicalize().unwrap_or(dir.clone());
if !canonical_dir.starts_with(&config_dir) {
    return Err(miette::miette!(
        "mds.json output_dir escapes project directory: {}",
        output_dir
    ));
}
```

Alternatively, reject `output_dir` values containing `..` components, matching the pattern used for `mds init` filenames.

---

### 4. **Missing Exit Code 3 (Resource Limit) Integration Test — HIGH**

**Location**: `tests/integration.rs`
**Confidence**: 90% (flagged by testing review)

**Problem**:
The new `exit_code()` function in `src/main.rs:329` maps `MdsError::ResourceLimit` to exit code 3. Integration tests cover exit codes 0 (success), 1 (syntax), and 2 (file not found), but code 3 is never tested end-to-end. This is a new behavioral contract introduced in this PR.

**Impact**:
A future change could silently break resource limit exit code reporting without detection.

**Fix**:
Add integration test:

```rust
#[test]
fn exit_code_resource_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge_loop.mds");
    let items: Vec<String> = (0..100_001).map(|i| i.to_string()).collect();
    let source = format!(
        "---\nitems: [{}]\n---\n@for item in items:\n{{item}}\n@end\n",
        items.join(", ")
    );
    std::fs::write(&path, &source).unwrap();
    let status = mds_bin()
        .args(["build"])
        .arg(&path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("failed to run mds");
    assert_eq!(status.code(), Some(3), "expected exit code 3 for resource limit");
}
```

---

## Should Fix Issues (Recommended, Lower Priority)

These issues are not blocking but should be addressed in the same change set:

### 5. **`run()` Function Too Large — 150 Lines, HIGH Complexity**
**Location**: `src/main.rs:405`
- Extract `run_build()`, `run_check()`, `run_init()` handler functions
- Reduces `run()` to a thin 15-line dispatcher
- Improves readability and testability

### 6. **`collect_definitions_and_imports` — 93 Lines with Deep Nesting**
**Location**: `src/resolver.rs:272`
- Extract `process_export()` handler for the `Node::Export` arm
- Moves nesting from 3-4 levels down to 2
- Keeps `collect_definitions_and_imports` as a loop-and-dispatch orchestrator

### 7. **`CollectedDefs` Tuple Type Alias — Opaque Fields**
**Location**: `src/resolver.rs:512`
- Convert to named struct or document fields in doc comment
- Fields: `functions`, `has_explicit_exports`, `explicit_exports`
- Improves code clarity without changing behavior

### 8. **`validate_and_read_file` Mixed Concerns**
**Location**: `src/resolver.rs:71-153`
- This function is 88 lines performing symlink detection, security checks, file I/O, and validation
- Consider splitting into `canonicalize_and_check_symlink()` + `read_and_validate_file()`
- Will naturally flow from fixing Issue #1

### 9. **Error Message Version Reference Inconsistency**
**Location**: `src/value.rs:60,92` vs `src/parser.rs:217,474`
- Either restore `"in MDS v0.1"` in value.rs or remove from parser.rs
- Consistency: all user-facing version-scoped errors should follow the same format

---

## Pre-existing Issues (Not Blocking, Informational)

These issues exist in code not modified by this PR. The review methodology ("Iron Law") states these should not block approval but can be noted for future work:

1. **`shift_remove` on `IndexSet` is O(n)** — Should use `pop()` for LIFO pattern (performance optimization)
2. **Symlink detection only covers final path component** — `root_dir` check catches escapes, but policy is incompletely enforced (minor)
3. **Lexer allocates 120 MB for 10 MB file** — Pre-existing architecture (acceptable for v0.1)
4. **`error.rs` has 290 lines of constructor boilerplate** — Known Rust pattern, acceptable trade-off
5. **Hardcoded `/tmp` in test** — Windows-incompatible; should use tempfile (minor fix)

---

## Positive Findings

This PR demonstrates several architectural improvements that merit highlight:

### Strong Refactoring
- **`EvalContext` struct**: Excellent bundling of `call_stack`, `total_iterations`, `warnings` into a single parameter. Reduces function arity from 5-7 to 3 across the evaluator.
- **Lexer decomposition**: Monolithic `tokenize()` closure converted to `Lexer<'a>` struct with focused `scan_*` methods. Clean, testable, textbook "Deep Module" design.
- **`Arc<FunctionDef>` + `Arc<ResolvedModule>`**: O(1) cloning at storage layers. Owned `FunctionDef` in `CapturedScope` correctly breaks reference cycles.
- **`IndexSet` replacement**: Eliminates redundant `HashSet + Vec` pair for cycle detection. Simpler, less error-prone.
- **`CapturedScope` struct**: Three separate `captured_*` fields consolidated into one struct with `Default` impl.
- **`process_module` decomposition**: 80-line monolith → ~25-line orchestrator calling focused helpers (`build_scope_from_frontmatter`, `collect_definitions_and_imports`, `validate_exports`).

### Comprehensive Testing
- Test count grew from 245 to 276 (31 new tests)
- All 276 tests pass
- New features (exit codes, file output, mds.json, --out-dir, stdin + output, export visibility) have thorough integration coverage
- Tests use proper temp directories and clear assertions

### API Consistency
- `serde_yaml` → `serde_yml` migration: complete and clean, zero stale references
- `Arc<FunctionDef>` adoption: consistent across all storage layers
- `EvalContext` parameter bundling: all evaluator functions consistently use it
- `IndexSet` for cycle detection: consistent replacement throughout
- `CapturedScope` struct: all access patterns uniform
- `mds_bin()` test helper: consistently used across all CLI tests
- `#[must_use]` attributes: new API functions properly marked

### Breaking Changes (Intentional, Documented)
- **Default output changed from stdout to file**: Tested in `build_stdin_defaults_to_file`, CLI help updated to document `-o -` for stdout
- **`to_namespace()` export visibility fix**: Module with explicit exports no longer exposes prompt body via `@include` unless "prompt" is listed. This is a bug fix (tested in `include_respects_export_visibility_for_prompt`).

### Release Readiness
- Exit codes now structured (0 success, 1 syntax error, 2 file not found, 3 resource limit) for programmatic CLI integration
- CLI help text comprehensive and up-to-date
- New `--out-dir` flag fully integrated with precedence rules documented
- `mds.json` config feature well-designed with upward directory walk discovery

---

## Recommendations for Approval

### Before Merge
1. ✅ Fix file I/O before cache check (Issue #1) — performance regression
2. ✅ Add size guard to `load_config` (Issue #2) — security pattern consistency
3. ✅ Add path traversal validation to `output_dir` (Issue #3) — security control
4. ✅ Add exit code 3 integration test (Issue #4) — test coverage gap

### Highly Recommended (Same Commit)
5. Extract handler functions from `run()` to reduce complexity (Issue #5)
6. Extract `process_export()` handler (Issue #6)
7. Convert `CollectedDefs` to named struct or document (Issue #7)

### Before Release
- Add CHANGELOG or release notes documenting the two breaking changes:
  - Default output to file (use `-o -` for stdout)
  - Export visibility bug fix (audit `@include` of modules with explicit exports)

---

## Confidence Summary

| Issue | Type | Reviewers | Avg Confidence |
|-------|------|-----------|----------------|
| File I/O before cache | HIGH | 4 | 92% |
| `load_config` size guard | HIGH | 3 | 82% |
| `output_dir` traversal | MEDIUM (HIGH) | 1 | 85% |
| Exit code 3 test | HIGH | 1 | 90% |
| `run()` complexity | HIGH | 1 | 90% |
| `collect_definitions` complexity | HIGH | 1 | 85% |
| `CollectedDefs` tuple | MEDIUM | 2 | 85% |

---

## Action Plan

1. **Address blocking issues (Issues #1-4)**
   - Refactor `validate_and_read_file` + `resolve` to cache-check before file-read
   - Add size guard to `load_config` with 1 MB limit
   - Add path traversal validation to `output_dir`
   - Add exit code 3 integration test

2. **Address high-complexity issues (Issues #5-7)**
   - Extract `run_build()`, `run_check()`, `run_init()` 
   - Extract `process_export()`
   - Convert `CollectedDefs` to named struct

3. **Plan release notes**
   - Document `-o -` migration path for stdout users
   - Document export visibility bug fix

4. **Merge checklist**
   - All 4 blocking issues fixed
   - All tests passing (should still be 276+)
   - Code review approval from architecture reviewer (complexity reduction is good)

---

## Score Summary

| Domain | Score | Status |
|--------|-------|--------|
| **Security** | 8/10 | APPROVED_WITH_CONDITIONS (2 fixes) |
| **Architecture** | 8/10 | APPROVED_WITH_CONDITIONS (3 fixes) |
| **Performance** | 7/10 | CHANGES_REQUESTED (file I/O regression) |
| **Complexity** | 7/10 | CHANGES_REQUESTED (2 large functions) |
| **Consistency** | 9/10 | APPROVED_WITH_CONDITIONS (1 fix) |
| **Regression** | 8/10 | APPROVED_WITH_CONDITIONS (breaking changes documented) |
| **Testing** | 8/10 | CHANGES_REQUESTED (1 test gap) |
| **Reliability** | 8/10 | CHANGES_REQUESTED (file I/O + debug_assert issue) |
| **Rust** | 8/10 | CHANGES_REQUESTED (file I/O regression + expect in library) |
| **Dependencies** | 7/10 | APPROVED_WITH_CONDITIONS (pre-release pinning) |
| **Documentation** | 8/10 | APPROVED_WITH_CONDITIONS (2 minor issues) |
| **Average** | **7.8/10** | **CHANGES_REQUESTED** |

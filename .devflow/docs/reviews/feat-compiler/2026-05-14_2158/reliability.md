# Reliability Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### HIGH

**`debug_assert` used for LIFO invariant in resolver while `assert` used in evaluator -- inconsistent safety posture** - `src/resolver.rs:204`
**Confidence**: 90%
- Problem: In `invoke_function` (evaluator.rs:196), the call_stack LIFO invariant was correctly promoted from `debug_assert!` to `assert!` with a clear comment explaining why: "Safety-critical LIFO invariant... Enforce in release mode -- cost is negligible at MAX_CALL_DEPTH = 128." However, in `resolver.rs:204`, the analogous LIFO invariant for the `resolving` IndexSet still uses `debug_assert_eq!`. The comment at line 201-202 says "resolve/unmark is strictly LIFO", yet this check is stripped in release builds. If the LIFO property were violated in production, `pop()` would silently remove the wrong path from `resolving`, corrupting cycle detection and potentially allowing circular imports to loop indefinitely or valid imports to be falsely rejected.
- Fix: Promote to `assert_eq!` for consistency with the evaluator pattern, or at minimum document why the weaker assertion is acceptable here (e.g., because `resolving` is bounded by MAX_IMPORT_DEPTH=64 and the cost is acceptable):
```rust
// resolver.rs:203-204
let popped = self.resolving.pop();
assert_eq!(popped.as_ref(), Some(&canonical), "resolving unmark must be LIFO");
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`read_validated_file` reads entire file into memory before checking size** - `src/resolver.rs:145-157`
**Confidence**: 85%
- Problem: `read_validated_file` calls `std::fs::read(canonical)` which loads the entire file into a `Vec<u8>` before checking `bytes.len() as u64 > MAX_FILE_SIZE`. The comment says "avoids TOCTOU race between a separate metadata call and the actual read", which is true -- but with MAX_FILE_SIZE at 10 MB, a malicious user could place a multi-GB file and cause the process to allocate gigabytes before the check fires. The refactoring that split `validate_and_read_file` into two methods (`canonicalize_and_check` + `read_validated_file`) removed the opportunity to add a pre-read size check at the call site.
- Fix: Add a pre-read metadata size check as a fast-reject before the full read. The TOCTOU race is benign here (file could grow between metadata and read), but it prevents obvious multi-GB allocation:
```rust
fn read_validated_file(canonical: &Path) -> Result<String, MdsError> {
    // Fast-reject obviously oversized files before allocating.
    // The post-read check below still catches TOCTOU races.
    if let Ok(meta) = std::fs::metadata(canonical) {
        if meta.len() > MAX_FILE_SIZE {
            return Err(MdsError::resource_limit(format!(
                "file too large ({} bytes, max {} bytes): {}",
                meta.len(), MAX_FILE_SIZE, canonical.display()
            )));
        }
    }
    let bytes = std::fs::read(canonical)
        .map_err(|e| MdsError::io(format!("cannot read {}: {e}", canonical.display())))?;
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(MdsError::resource_limit(format!(
            "file too large ({} bytes, max {} bytes): {}",
            bytes.len(), MAX_FILE_SIZE, canonical.display()
        )));
    }
    String::from_utf8(bytes).map_err(|e| {
        MdsError::io(format!("invalid UTF-8 in {}: {e}", canonical.display()))
    })
}
```

**Config size check has metadata TOCTOU gap followed by unbounded `read_to_string`** - `src/main.rs:55-70`
**Confidence**: 82%
- Problem: `load_config` checks `metadata(&candidate).map(|m| m.len())` against MAX_CONFIG_SIZE, then calls `read_to_string(&candidate)`. Between the metadata check and the read, the file could be replaced with a larger one (TOCTOU). Unlike `read_validated_file` in resolver.rs which reads as bytes first and then checks length, `load_config` uses `read_to_string` which could allocate beyond MAX_CONFIG_SIZE. The `unwrap_or(0)` on metadata failure also means a file whose metadata cannot be read (e.g., permissions issue on stat but not read) would bypass the size check and proceed to read_to_string.
- Fix: Use the same read-then-check pattern as `read_validated_file`:
```rust
let bytes = std::fs::read(&candidate).map_err(|e| {
    miette::miette!("cannot read {}: {e}", candidate.display())
})?;
if bytes.len() as u64 > MAX_CONFIG_SIZE {
    return Err(miette::miette!(
        "mds.json at {} is too large ({} bytes; maximum is 1 MB)",
        candidate.display(),
        bytes.len()
    ));
}
let raw = String::from_utf8(bytes).map_err(|e| {
    miette::miette!("invalid UTF-8 in {}: {e}", candidate.display())
})?;
```

## Pre-existing Issues (Not Blocking)

No critical pre-existing reliability issues found in the reviewed files.

## Suggestions (Lower Confidence)

- **`collect_all` clones all values across all frames without size bound** - `src/scope.rs:172-180` (Confidence: 65%) -- Closure capture via `get_all_vars`/`get_all_functions`/`get_all_namespaces` flattens the entire scope chain into a new HashMap. With deeply nested scopes and many captured variables, this could produce large allocations. However, scope depth is bounded by MAX_CALL_DEPTH (128) and iteration bounds, so practical risk is low.

- **`exit_code_resource_limit` test generates ~1M iterations in CI** - `tests/integration.rs:3027-3067` (Confidence: 62%) -- This test intentionally exercises the MAX_TOTAL_ITERATIONS limit, which means it runs the binary through approximately 1M iterations before it stops. While correctness requires this, it may be slow in CI. Consider a `#[ignore]` attribute with a CI-specific runner, or lowering the constants in a test-only configuration.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The PR demonstrates strong reliability awareness overall: bounded iteration limits (MAX_CALL_DEPTH, MAX_LOOP_ITERATIONS, MAX_TOTAL_ITERATIONS, MAX_OUTPUT_SIZE), promotion of the call_stack LIFO assert to release mode, proper double-fault error-preservation patterns in both `invoke_function` and `evaluate_for`, config size limits, and path traversal guards. The primary concern is the inconsistent assertion strength between the evaluator (release assert) and resolver (debug-only assert) for analogous LIFO invariants. The TOCTOU gaps in size checking are lower severity but worth addressing for defense-in-depth.

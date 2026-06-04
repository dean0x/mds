# Testing Review Report

**Branch**: HEAD -> main
**Date**: 2026-05-18
**PR**: #22 — SerializedError/SerializedSpan, CompileOutput with dependency tracking, FileSystem::canonicalize()

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing serialize() tests for 3 of 16 MdsError variants: UndefinedFunction, ImportError, NameCollision** - `crates/mds-core/src/error.rs:528`
**Confidence**: 90%
- Problem: The `serialize()` method explicitly matches all 11 span-bearing variants in its match arm (lines 538-548). Tests exist for 8 of these 11 variants (Syntax, UndefinedVariable, ArityMismatch, TypeError, CircularImport, FileNotFound, Recursion, ExportError via the wildcard no-span tests), but `UndefinedFunction`, `ImportError`, and `NameCollision` have no serialize-specific test. While the match arm is shared, each variant has distinct diagnostic attributes (`code`, `help`) that should be verified. The PR description explicitly states coverage of "all 16 error variants in serialize()" as a goal.
- Fix: Add three tests following the existing pattern:
```rust
#[test]
fn serialize_undefined_fn_with_span() {
    let e = MdsError::undefined_fn_at("greet", "f.mds", "{{ greet() }}", 3, 5);
    let s = e.serialize();
    assert_eq!(s.code, "mds::undefined_fn");
    assert!(s.help.is_some(), "UndefinedFunction should have help text");
    assert!(s.span.is_some());
}

#[test]
fn serialize_import_error_with_span() {
    let e = MdsError::import_error_at("bad path", "f.mds", "source", 0, 6);
    let s = e.serialize();
    assert_eq!(s.code, "mds::import");
    assert_eq!(s.help, None);
    assert!(s.span.is_some());
}

#[test]
fn serialize_name_collision_with_span() {
    let e = MdsError::name_collision_at("foo", "f.mds", "source", 0, 3);
    let s = e.serialize();
    assert_eq!(s.code, "mds::name_collision");
    assert_eq!(s.help, None);
    assert!(s.span.is_some());
}
```

**Missing serialize() test for ExportError variant** - `crates/mds-core/src/error.rs:528`
**Confidence**: 88%
- Problem: `ExportError` is listed in the span-bearing match arm (line 548) but has no dedicated serialize test. While `export_error_at` is `#[allow(dead_code)]` (suggesting it is not called in production today), the serialize match arm handles it, and correctness should still be verified since the code is explicitly included.
- Fix: Add a test:
```rust
#[test]
fn serialize_export_error_with_span() {
    let e = MdsError::export_error_at("bad export", "f.mds", "source", 0, 6);
    let s = e.serialize();
    assert_eq!(s.code, "mds::export");
    assert_eq!(s.help, None);
    assert!(s.span.is_some());
}
```

### MEDIUM

**Missing edge case: serialize() with span=Some but src=None** - `crates/mds-core/src/error.rs:549-560`
**Confidence**: 85%
- Problem: The `serialize()` doc comment (line 525-527) explicitly describes the behavior when "span is Some but src is None" -- that line/column should be None while offset/length are still populated. No test covers this edge case. This is a documented contract that should be verified.
- Fix: Construct a variant directly with span set but src as None and verify the behavior:
```rust
#[test]
fn serialize_span_without_src_gives_no_line_col() {
    let e = MdsError::Syntax {
        message: "test".into(),
        span: Some(SourceSpan::from((5, 3))),
        src: None,
    };
    let s = e.serialize();
    let span = s.span.expect("span should be Some");
    assert_eq!(span.offset, 5);
    assert_eq!(span.length, 3);
    assert_eq!(span.line, None, "line should be None when src is missing");
    assert_eq!(span.column, None, "column should be None when src is missing");
}
```

**Missing edge case: compute_line_column at offset == source.len()** - `crates/mds-core/src/error.rs:40-55`
**Confidence**: 82%
- Problem: The boundary condition is `offset > source.len()` returns None, meaning `offset == source.len()` is explicitly valid. This off-by-one boundary is tested for `offset > len` (line_col_out_of_bounds) and `offset == 0` (line_col_first_byte), but there is no test for `offset == source.len()`. This is a common source of off-by-one bugs in span computation.
- Fix:
```rust
#[test]
fn line_col_at_end_of_source() {
    // offset == source.len() is valid (zero-width span at EOF).
    assert_eq!(compute_line_column("abc", 3), Some((1, 4)));
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing CRLF test for compute_line_column** - `crates/mds-core/src/error.rs:40` (Confidence: 65%) — The function only checks `b'\n'` for line breaks; a `\r\n` source would count `\r` as incrementing the column. This is consistent with the stated byte-offset convention, but a test documenting this intentional behavior (or documenting that CRLF sources are unsupported) would clarify the contract.

- **No test for dependency ordering stability in diamond pattern** - `crates/mds-core/tests/virtual_fs.rs:349` (Confidence: 70%) — `deps_diamond_no_duplicates` asserts that all 3 deps are present and count is 3, but does not assert the exact DFS ordering `["shared.mds", "a.mds", "b.mds"]`. The comment on line 354 documents the expected order. Since `IndexMap` preserves insertion order and this is a documented contract, pinning the exact order would catch future regressions if the resolution algorithm changes.

- **compile_str_with_deps lacks an import-based dependency test** - `crates/mds-core/tests/virtual_fs.rs:388` (Confidence: 62%) — `deps_str_with_deps_basic` only tests the no-import case. The comment on line 391 acknowledges this limitation ("would look for real files... skip this variant here"). A tempfile-based integration test verifying that `compile_str_with_deps` correctly tracks dependencies from file imports would close this gap.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The test suite is well-structured overall. Tests follow clear Arrange-Act-Assert patterns, are behavior-focused (asserting serialized output rather than internal state), and cover the key paths: per-variant serialization, compute_line_column edge cases, dependency graph topologies (single, chain, diamond, error), API surface regression, and JSON round-tripping.

The main gap is incomplete variant coverage in serialize() tests -- 3 of the 11 span-bearing variants and 1 additional variant (ExportError) lack dedicated tests. While the shared match arm reduces the risk, each variant carries unique `code` and `help` attributes derived from `miette::Diagnostic` macros, so variant-specific assertions are warranted. The documented edge case of span-without-source also lacks verification. These are straightforward additions that would bring coverage from ~75% to 100% of the serialize() contract.

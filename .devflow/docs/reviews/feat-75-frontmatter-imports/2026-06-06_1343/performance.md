# Performance Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Extra allocation in `strip_reserved_keys`** - `crates/mds-core/src/lib.rs:437` (Confidence: 65%) -- `format!("{trimmed}\n")` allocates a third string after building `filtered` and slicing `trimmed`. Could return `filtered` directly with in-place truncation to avoid one allocation. Negligible impact for typical frontmatter sizes (< 1KB).

- **Double YAML parse in `scan_imports` path** - `crates/mds-core/src/lib.rs:805` (Confidence: 60%) -- `parse_frontmatter_imports(&fm.raw)` parses the full frontmatter YAML to extract only the `imports` key. This is a standalone utility path (not the compile hot path), and the parse is required since `scan_imports` does not have access to a pre-parsed YAML value. Acceptable design tradeoff.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 0 | - |
| Should Fix | - | 0 | 0 | - |
| Pre-existing | - | - | 0 | 0 |

**Performance Score**: 9
**Recommendation**: APPROVED

### Rationale

The implementation demonstrates strong performance-aware design throughout:

1. **Single-parse frontmatter** -- `build_scope_from_frontmatter` parses YAML once and routes the `imports` value directly to `parse_frontmatter_imports_from_yaml`, avoiding double-parsing. The comment "Parse YAML once to avoid double-parsing" at `resolver.rs:779` confirms this was an intentional design choice.

2. **Defense-in-depth limits** -- `MAX_FRONTMATTER_IMPORTS = 256` (limits.rs:60) caps the number of import entries, preventing adversarial inputs from triggering unbounded file resolutions. This correctly complements the existing `MAX_IMPORT_DEPTH` guard.

3. **Existing guards honored** -- Frontmatter imports flow through `resolve_import_from` -> `resolve_by_key`, inheriting cache hits (O(1) Arc clone), cycle detection, and depth guards. No new resolution paths bypass these protections.

4. **Pre-sized allocations** -- `Vec::with_capacity(seq.len())` at resolver.rs:1037 and resolver.rs:1106, `String::with_capacity(raw.len())` at lib.rs:400. Allocations are right-sized.

5. **Efficient string scanning** -- `strip_reserved_keys` uses a single-pass line scan with state tracking (`in_imports_block`) rather than regex or YAML re-parsing. `has_type_mds_frontmatter_raw` is a lightweight line scan that short-circuits via `.any()`.

6. **IndexSet for deduplication** -- `scan_imports` uses `IndexSet` for O(1) insertion-order-preserving dedup, consistent with the existing pattern.

No blocking or should-fix performance issues found. The two suggestions are below the 80% confidence threshold and represent minor optimization opportunities with negligible real-world impact.

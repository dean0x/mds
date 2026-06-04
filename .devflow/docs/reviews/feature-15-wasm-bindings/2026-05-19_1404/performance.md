# Performance Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19T14:04
**Scope**: Incremental review (420e2259...HEAD) -- 7 commits

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Redundant source copy before size check** - `crates/mds-wasm/src/lib.rs:370-373`
**Confidence**: 82%
- Problem: `check_source_size(source)` runs on the `&str` reference (good), but then `source.to_string()` on line 373 unconditionally copies the entire source into an owned `String` for `UnwindSafe` compliance. This copy happens even for the `check()` path (line 410) which only validates and never needs the rendered output. For large inputs approaching the 10 MiB limit, this is a non-trivial allocation inside WASM linear memory.
- Fix: This is an inherent constraint of `catch_unwind` requiring `UnwindSafe` data. The copy is necessary for correctness. However, the `check()` function could potentially use a separate code path that avoids the full compilation pipeline if `mds-core` exposed a validation-only API accepting `&str` directly. This is a design-level optimization opportunity, not a bug. No immediate code change required -- flag for future consideration if profiling shows this matters.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Serializer allocation per call** - `crates/mds-wasm/src/lib.rs:337` (Confidence: 65%) -- `serde_wasm_bindgen::Serializer::json_compatible()` is constructed on every `compile()` and `check()` invocation. If the serializer carries allocation overhead, a `thread_local!` or `static` instance could avoid repeated setup. In WASM single-threaded context this would be safe. Likely negligible impact for typical template sizes.

- **`wasm-opt` disabled in release profile** - `crates/mds-wasm/Cargo.toml:31` (Confidence: 70%) -- The `wasm-opt = false` setting skips Binaryen optimization passes that typically yield 10-20% binary size reduction and can improve runtime performance through dead code elimination and instruction folding. The comment explains the rationale (no CI guarantee of Binaryen), which is reasonable for now. Re-enabling once CI is configured would benefit both binary size and execution speed.

- **`format!` in error paths** - `crates/mds-wasm/src/lib.rs:312-316` and similar (Confidence: 60%) -- Several error construction paths use `format!` to build error messages (e.g., `check_source_size`, `build_modules`). These allocate only on the error path, so impact is minimal for happy-path performance. Mentioned only for completeness -- error paths are cold by definition.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 8/10
**Recommendation**: APPROVED

## Rationale

The incremental changes are overwhelmingly performance-positive:

1. **Clone elimination via ownership destructuring** -- The refactor from `map.get()` + `.clone()` to `map.remove()` in `parse_filename`, `parse_modules`, and `parse_vars` eliminates unnecessary clones of every string value in the options object. This is the correct Rust pattern: take ownership when the source is consumed. For options objects with many modules, this avoids O(n) redundant allocations.

2. **`check_source_size` guard before allocation** -- The size check now happens on the borrowed `&str` before `source.to_string()` copies it. This means oversized inputs are rejected without the allocation cost of cloning them. Good fail-fast pattern.

3. **`MAX_SOURCE_SIZE` derived from `mds::MAX_FILE_SIZE`** -- Single source of truth prevents drift between the core file-size limit and the WASM boundary limit. No performance impact directly, but eliminates a class of bugs where mismatched limits could allow oversized inputs through one path.

4. **`load_vars_str` size guard in mds-core** -- The new size check at line 760 prevents `serde_json::from_str` from attempting to parse an arbitrarily large string. JSON parsing has O(n) memory overhead for the parsed tree, so guarding before parse is correct.

5. **Release profile is well-tuned** -- `opt-level = "z"` (size), LTO, `strip = true`, and `codegen-units = 1` are the standard WASM optimization settings. The 455 KB binary size (within the 500 KB budget) reflects these choices.

The one MEDIUM finding (redundant copy for `check()`) is an inherent `catch_unwind` constraint, not a regression. No blocking performance issues were introduced.

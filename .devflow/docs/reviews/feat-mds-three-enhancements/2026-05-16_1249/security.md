# Security Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Silent data loss: YAML non-string keys silently dropped** - `src/value.rs:65-70`
**Confidence**: 85%
- Problem: In `from_yaml_inner`, when processing a `Mapping`, entries with non-string keys are silently skipped (`if let serde_yml::Value::String(key) = k`). YAML allows integer, boolean, and null keys. A user who provides a YAML frontmatter with integer keys (e.g., `42: value`) will have those entries silently disappear with no warning or error. While MDS reasonably requires string keys, the silent discard could mask user mistakes or lead to data integrity surprises.
- Fix: Return an error (or at minimum push a warning) when non-string keys are encountered instead of silently dropping them:
```rust
serde_yml::Value::Mapping(mapping) => {
    let mut map = HashMap::new();
    for (k, v) in mapping {
        if let serde_yml::Value::String(key) = k {
            let value = Self::from_yaml_inner(v, depth + 1)?;
            map.insert(key, value);
        } else {
            return Err(MdsError::yaml_error(format!(
                "object keys must be strings, got: {k:?}"
            )));
        }
    }
    Ok(Value::Object(map))
}
```

**Frontmatter preservation may expose sensitive variables in output** - `src/lib.rs:342-372`
**Confidence**: 80%
- Problem: The `prepend_frontmatter` function preserves raw frontmatter content (minus `type: mds`) in the compiled output. If users place sensitive data in frontmatter (API keys, secrets used as template variables), the raw frontmatter is now emitted in the output file. Before this change, frontmatter was consumed during compilation but never written to output. This is a behavior change that could inadvertently leak secrets if users relied on frontmatter being an internal-only mechanism.
- Fix: This is an intentional design decision, but it should be documented as a behavioral change. Consider adding a `type: private` or similar mechanism to allow users to opt out of frontmatter preservation. At minimum, the CHANGELOG should warn users about this behavioral change. The `strip_type_mds` function currently only strips `type: mds` -- it could be extended to strip any `type:` key, which would give users a way to prevent frontmatter emission.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`strip_type_mds` only matches exact `type: mds` -- case-sensitive and whitespace-sensitive** - `src/lib.rs:342-358`
**Confidence**: 82%
- Problem: The `strip_type_mds` function filters lines matching `type:` prefix with value trimmed to `mds`. However, it only matches the exact lowercase string `mds`. Variations like `type: MDS`, `type: Mds`, or `type:  mds  ` (extra internal whitespace) would be stripped by `.trim()` correctly, but `type: MDS` would not be filtered. While the compiler likely rejects non-lowercase `type: mds` elsewhere, if it does not, this could result in `type: MDS` leaking into output frontmatter.
- Fix: Use case-insensitive comparison:
```rust
.is_some_and(|v| v.trim().eq_ignore_ascii_case("mds"))
```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **No depth bound on dot-path traversal** - `src/evaluator.rs:99-124` (Confidence: 65%) -- The `resolve_dot_path` function traverses object fields without a depth limit. While the parser's `MAX_NESTING_DEPTH` bounds the number of segments in a dot path at parse time (each segment must be a valid identifier separated by dots), deeply nested YAML objects (up to `MAX_VALUE_DEPTH=64`) combined with long dot paths could lead to deep traversal. In practice the parser limits the path length to the interpolation content, which is bounded by `MAX_FILE_SIZE`, so this is low risk. However, an explicit depth check would be defense-in-depth.

- **`HashMap` key collision via YAML aliases/anchors** - `src/value.rs:63-72` (Confidence: 60%) -- When constructing `HashMap` from YAML mappings, duplicate string keys silently overwrite earlier values via `map.insert(key, value)`. YAML aliases and anchors can produce duplicate keys. This is standard `HashMap` behavior but could surprise users expecting an error.

- **`debug_assert` for path emptiness is not a release-mode guard** - `src/evaluator.rs:100`, `src/validator.rs:25,49` (Confidence: 70%) -- The `debug_assert!(!path.is_empty())` / `debug_assert!(!block.condition.is_empty())` calls only fire in debug builds. If a parser bug produces an empty path, the release build would panic on `path[0]` index access. Consider using a regular `if path.is_empty() { return Err(...) }` guard for defense-in-depth, consistent with the project's philosophy of using `assert!` (not `debug_assert!`) for safety-critical invariants (as done for the call_stack LIFO check in the evaluator).

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

The changes are generally well-secured. The existing security architecture -- depth limits, path traversal prevention, symlink rejection, resource caps, identifier validation -- has been properly extended to cover the new object/map type and dot-notation access features. Key security positives:

1. **Depth limits preserved**: `MAX_VALUE_DEPTH=64` is correctly enforced for both YAML mapping and JSON object nesting, preventing stack overflow from deeply nested input data.
2. **Identifier validation intact**: All dot-path segments are validated via `is_valid_identifier()` at parse time, preventing injection of arbitrary strings.
3. **Resource limits extended**: `MAX_LOOP_ITERATIONS` and `MAX_TOTAL_ITERATIONS` are correctly enforced for the new key-value object iteration path.
4. **Objects cannot be directly interpolated**: The evaluator correctly rejects attempts to interpolate `Value::Object` directly, preventing accidental data structure dumps.
5. **Import security unchanged**: The existing symlink, path traversal, depth, and file size checks are untouched and continue to function correctly.

The two blocking MEDIUM issues (silent key dropping, frontmatter exposure) are not exploitable vulnerabilities but represent data integrity and information disclosure risks that should be addressed before merge.

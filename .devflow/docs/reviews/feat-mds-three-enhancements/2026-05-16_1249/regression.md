# Regression Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`strip_type_mds` strips indented `type: mds` lines from nested YAML objects** - `src/lib.rs:342-358`
**Confidence**: 95%
- Problem: The `strip_type_mds` function performs a line-by-line text scan using `line.trim().strip_prefix("type:").is_some_and(|v| v.trim() == "mds")`. Because it trims leading whitespace, it matches `  type: mds` inside nested YAML objects, not just the top-level `type: mds` key. For example, frontmatter like `nested:\n  type: mds` will have the nested `type: mds` line stripped from the output frontmatter, corrupting the output YAML structure. The `nested:` key will be preserved but its child will be removed.
- Impact: Output frontmatter is silently corrupted when any nested mapping contains a `type: mds` key-value pair. This is a data loss issue for templates that use `type: mds` as an application-specific key inside nested objects.
- Fix: Only strip the top-level `type: mds` line by checking that the line has no leading whitespace before the `type:` prefix:
```rust
fn strip_type_mds(raw: &str) -> Option<String> {
    let filtered: String = raw
        .lines()
        .filter(|line| {
            // Only strip top-level type: mds (no leading whitespace)
            let trimmed = line.trim_start();
            !(trimmed == line.trim() 
              && line.len() == trimmed.len() + line.len() - line.trim_start().len()
              && trimmed.strip_prefix("type:")
                  .is_some_and(|v| v.trim() == "mds")
              && !line.starts_with(' ') && !line.starts_with('\t'))
        })
        // Simpler alternative:
        // .filter(|line| {
        //     !(line.strip_prefix("type:")
        //           .is_some_and(|v| v.trim() == "mds"))
        // })
        .map(|line| format!("{line}\n"))
        .collect();
    if filtered.trim().is_empty() {
        None
    } else {
        Some(filtered)
    }
}
```
  Simplest correct fix: replace `line.trim().strip_prefix("type:")` with `line.strip_prefix("type:")` (no trim) to only match unindented lines.

### MEDIUM

**Validator skips static type check for dot-path iterables in `@for` loops** - `src/validator.rs:60-83`
**Confidence**: 82%
- Problem: The validator previously caught non-array iterables at validate time with a precise source-span error. With the dot-path change, the condition `block.iterable.len() == 1` means the validator now skips the static type check entirely when the iterable is a dot path (e.g., `@for item in config.items:`). The error is still caught at evaluation time by the evaluator, but without source-span information from the validator's `type_error_at` constructor.
- Impact: Error quality regression for dot-path iterables: users get a generic `type error: expected array for @for loop, got string` instead of a source-span-annotated error pointing to the exact `@for` directive. This is a diagnostic quality regression, not a correctness issue -- the error is still raised.
- Fix: Consider resolving dot-path values in the validator for static type checking when the root variable exists and all path segments can be statically resolved. Alternatively, document this as a known limitation of dot-path validation.

**`parse_interpolation_expr` lost source-location-aware error reporting** - `src/parser.rs:500-501`
**Confidence**: 85%
- Problem: The `file` and `source` parameters were renamed to `_file` and `_source` (prefixed with underscores), indicating they are now unused. Previously, the `dot_notation_error` function used these parameters to produce `syntax_at` errors with precise source spans for unsupported dot notation. Now that dot notation is supported, the error path was removed, but ALL error messages from `parse_interpolation_expr` (including "invalid dot-path in interpolation", "unclosed parenthesis in function call", "invalid interpolation") now use `MdsError::syntax(msg)` instead of `MdsError::syntax_at(msg, file, source, offset, len)`. These parameters could be used to produce better errors for the remaining failure paths.
- Impact: Diagnostic quality regression -- parse errors from interpolation expressions lack source-span context that was previously available for the dot-notation error case. Users see error messages without file:line:column information for malformed interpolations.
- Fix: Remove the underscore prefixes and use `file` and `source` to produce `syntax_at` errors for the remaining error paths in the function (invalid dot-path, unclosed parenthesis, invalid interpolation).

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`@for key, value` validator does not validate key_var uniqueness against var** - `src/parser.rs:269-293` (Confidence: 65%) -- The parser accepts `@for x, x in obj:` where the key and value variable names are identical. This would cause the key to be shadowed by the value in the loop body. Consider adding a check that `key_var != var`.

- **`from_yaml` silently skips non-string YAML keys** - `src/value.rs:66-70` (Confidence: 62%) -- When converting YAML mappings with non-string keys (e.g., numeric keys), the converter silently skips them. A warning or error might be more appropriate to avoid confusion when users have YAML with integer keys.

- **`resolve_dot_path` error message always references root variable name** - `src/evaluator.rs:110-112` (Confidence: 68%) -- The error `"field '{field}' not found on object '{root}'"` uses `root` (the top-level variable name) even for deep paths. For `a.b.c.missing`, the error says "not found on object 'a'" rather than "not found on object at 'a.b.c'", which could be confusing for nested access.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The branch introduces three well-implemented features (object/map support, frontmatter preservation, escape doc fixes) with strong test coverage (298 new/modified test lines, 325 tests passing). The AST breaking changes (`IfBlock.condition: String -> Vec<String>`, `ForBlock.iterable: String -> Vec<String>`, new `Expr::MemberAccess` and `Arg::MemberAccess` variants) are correctly scoped to `pub(crate)` modules, so there is no public API breakage. The `Value::Object` variant addition is protected by `#[non_exhaustive]`, making it semver-compatible.

The blocking HIGH issue is the `strip_type_mds` function stripping `type: mds` lines from nested YAML objects. This is a data corruption bug that silently produces invalid frontmatter output. The fix is straightforward (check for no leading indentation before stripping).

The two MEDIUM issues are diagnostic quality regressions: dot-path iterables lose static type-check source spans, and `parse_interpolation_expr` no longer uses `file`/`source` for source-located errors. Both are quality-of-life regressions for error messages, not correctness issues.

No removed public exports. No removed CLI options. No changed return types in public API. No incomplete migrations. Commit messages accurately describe the implemented changes.

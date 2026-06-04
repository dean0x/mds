# Regression Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27T01:00:00Z

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

(none)

### MEDIUM

**Unknown directive error message does not mention @elseif** - `crates/mds-core/src/parser.rs:212`
**Confidence**: 82%
- Problem: The unknown directive error message lists valid directives as `@if, @else:, @end, @for, @define, @import, @export, @include` but does not include `@elseif`. If a user writes `@elseif` at the top level (outside an `@if` block), they get a generic "unknown directive" error with no mention of `@elseif`, when a more targeted hint (similar to the `@else` without colon hint at line 205) would be more helpful.
- Fix: Add `@elseif` to the valid directives list, or add a targeted error before the generic catch-all:
```rust
// Before the generic "unknown directive" error:
if trimmed.starts_with("@elseif ") || trimmed == "@elseif" {
    return Err(MdsError::syntax(
        "@elseif can only appear inside an @if block — did you forget the @if?",
    ));
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Deep nesting tests need thread with larger stack -- increased recursion from `parse_body` signature change** - `crates/mds-core/src/parser.rs:1556`, `crates/mds-cli/tests/security.rs:243`
**Confidence**: 80%
- Problem: The `parse_body` method now accepts an additional `prefix_terminators` parameter, adding one more slice reference to each recursive call frame on the stack. Two nesting-depth tests (`parse_nesting_depth_limit_rejected` and `parser_nesting_depth_limit_rejects_deep_nesting`) were updated to spawn threads with 8 MB stacks. While this works, it is a signal that the parser's stack consumption per nesting level increased. The `parse_body -> parse_directive -> parse_if_block -> parse_body` recursion chain now carries more data per frame. If MAX_NESTING_DEPTH (256) is ever increased, or additional parameters are added to `parse_body`, these tests (and production use) could hit stack overflow before the depth limit fires.
- Fix: This is adequately handled for now by the thread stack workaround, but the root issue (deep recursion vs. iterative parsing) should be tracked. No immediate action required -- the MAX_NESTING_DEPTH at 256 with 8 MB stack is a safe margin. Consider adding a comment noting the relationship between MAX_NESTING_DEPTH and minimum stack size.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`_esmImport` wrapper is accessible in ESM build** - `packages/webpack-loader/src/index.ts:10` (Confidence: 65%) -- The `new Function('id', 'return import(id)')` wrapper is compiled into both ESM and CJS builds, but is only needed for CJS. In the ESM build it adds a minor indirection compared to a direct `import()`. Not harmful, but could be conditionally compiled or split into separate entry points for ESM/CJS if build separation is desired in the future.

- **`find_unquoted_operator` does not handle escaped closing quotes adjacent to backslashes in multi-escape edge cases** - `crates/mds-core/src/parser.rs:493-503` (Confidence: 62%) -- The string tracking logic in `find_unquoted_operator` checks for the closing quote before checking for escape sequences. For the sequence `\"`, this works correctly because `\` is not the quote character. However, the ordering means that if a future change introduced additional escape handling (e.g., for `\n`, `\t`), the close-then-escape ordering could interact subtly. The current code handles all test cases correctly.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 9/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Regression Analysis Summary

**AST Breaking Change -- Fully Migrated**: The `IfBlock.condition` field changed from `Vec<String>` to `Condition` enum. All three consumer sites (parser, evaluator, validator) plus unit tests have been updated. No external crate references the old type. Migration is complete.

**`parse_body` Signature Change -- Fully Migrated**: All 7 call sites updated from 1-parameter to 2-parameter signature. No missed call sites.

**Behavioral Change -- Intentional**: `@if !var:` was previously a parse error and is now supported (negation). The old test `if_negation_error_message_is_actionable` was correctly renamed to `if_negation_supported` and assertions updated. Double negation (`!!`) remains a parse error. This is an intentional feature addition, not a regression.

**No Removed Exports**: No public exports were removed from any Rust crate or JS package. Package.json `exports` fields were extended (added `require` path) without removing the existing `import` path.

**No Removed Files**: No files were deleted in this PR.

**Spec Updated**: The specification at `spec.md` was updated to remove the "No @elseif in v0.1" line and add comprehensive documentation for negation, equality, inequality, and @elseif. Spec matches implementation.

**Test Coverage**: 47 new test cases added across language.rs, errors.rs, and security.rs, plus CJS compatibility tests for both bundler-utils and webpack-loader packages.

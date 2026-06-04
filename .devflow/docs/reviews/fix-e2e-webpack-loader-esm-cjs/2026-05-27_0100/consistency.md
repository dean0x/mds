# Consistency Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### HIGH

**Unknown directive error message does not list `@elseif`** - `crates/mds-core/src/parser.rs:212`
**Confidence**: 90%
- Problem: The "unknown directive" error message lists valid directives but omits the newly added `@elseif`. When a user mistypes an `@elseif` directive (e.g., `@elseif` without a space or condition), the error hints at valid options but fails to mention `@elseif`, which is now a valid directive.
- Fix: Update the error message to include `@elseif`:
```rust
"unknown directive: {trimmed}. Valid directives: @if, @elseif, @else:, @end, @for, @define, @import, @export, @include"
```

**Dead comment block in `parse_condition`** - `crates/mds-core/src/parser.rs:539-543`
**Confidence**: 85%
- Problem: A 4-line comment block reads "Check for bare `=` (not `==`) -- common mistake / We check this after the equality operator scan, but we do a quick pre-check here..." followed by "This is handled below after the equality check." The actual bare `=` check appears at line 588-599, NOT here. These orphaned lines are a draft remnant that was never cleaned up, creating misleading documentation for future readers.
- Fix: Delete lines 539-543. The actual bare-`=` check at line 588 already has its own clear comment.

### MEDIUM

**Missing `@elseif` colon hint matching `@else` colon hint** - `crates/mds-core/src/parser.rs:204-209`
**Confidence**: 82%
- Problem: The parser has a targeted hint for `@else` without a trailing colon (line 204-208: "found '@else' without colon -- use '@else:'"). No equivalent hint exists for `@elseif` written without a proper space or colon (e.g., `@elseif:` or bare `@elseif`). The `@elseif` directive is matched via prefix `"@elseif "` (with trailing space), so `@elseif:` or `@elseif` alone would fall through to the generic "unknown directive" error rather than giving a helpful format hint. This is an inconsistency in the error-hint pattern between `@else` and `@elseif`.
- Fix: Add a parallel hint block before the unknown-directive catch-all:
```rust
if trimmed == "@elseif" || trimmed == "@elseif:" {
    return Err(MdsError::syntax(
        "found '@elseif' — use '@elseif <condition>:' (with condition and trailing colon)",
    ));
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`find_unquoted_operator` escape check runs after closing-quote detection** - `crates/mds-core/src/parser.rs:493-503`
**Confidence**: 80%
- Problem: Inside the `if in_string` block, when `ch == string_char` the code sets `in_string = false` but does NOT immediately `continue`. It falls through to the escape check (`if ch == b'\\'`), which means a closing quote is also tested as a potential escape character. In practice this is safe because string delimiters (`"` and `'`) are never `\\`, but the control flow is fragile: an `else if` or early `continue` after the close-string branch would be more defensive. If a future change adjusted quoting behavior, this ordering could become a real bug.
- Fix: Add a `continue` or restructure with `else if`:
```rust
if in_string {
    if ch == string_char {
        in_string = false;
        i += 1;
        continue;
    }
    if ch == b'\\' && i + 1 < len {
        i += 2;
        continue;
    }
    i += 1;
    continue;
}
```

## Pre-existing Issues (Not Blocking)

No critical pre-existing consistency issues found in unchanged code.

## Suggestions (Lower Confidence)

- **CJS build script duplication** - `packages/bundler-utils/package.json:26`, `packages/webpack-loader/package.json:22` (Confidence: 65%) -- The build scripts for both packages contain identical inline `node -e` commands to write `dist-cjs/package.json`. As more packages adopt CJS builds, this could be extracted into a shared script or workspace-level postbuild step, matching how `tsconfig.base.json` is already shared.

- **Vite and Rollup plugins lack CJS builds** - `packages/vite-plugin/package.json`, `packages/rollup-plugin/package.json` (Confidence: 65%) -- The PR adds dual ESM+CJS builds to `bundler-utils` and `webpack-loader` but not to `vite-plugin` or `rollup-plugin`. This may be intentional (Vite/Rollup are ESM-native), but the package export shapes are now inconsistent across the bundler plugin family. If CJS consumers import from `vite-plugin` or `rollup-plugin`, they would get no `require` entry.

- **`CondValue::Bool` vs `Value::Boolean` naming asymmetry** - `crates/mds-core/src/ast.rs:23` (Confidence: 70%) -- The existing `Value` enum uses `Boolean(bool)` while the new `CondValue` enum uses `Bool(bool)`. This is a minor naming divergence between the runtime value type and the AST condition value type. Not strictly a bug (they are distinct types), but the inconsistent abbreviation could confuse future contributors.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The PR introduces two well-structured feature sets (CJS dual builds and `@if` condition enhancements) that are internally consistent. The new Rust code follows existing patterns well: `Condition` enum mirrors the existing `Expr`/`Arg` enum style, the `parse_condition` / `evaluate_condition` / `validate_condition` functions follow the same decomposition pattern as existing helpers, and the CJS build config (`tsconfig.cjs.json`) is consistent across the two packages that adopt it. The two HIGH issues (missing `@elseif` in the error hint list, dead comment) are straightforward cleanup. The MEDIUM issues are minor pattern gaps. Overall, the changes show good pattern discipline.

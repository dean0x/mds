# Documentation Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Missing escape sequence documentation for condition string literals** - `spec.md:128-138`
**Confidence**: 85%
- Problem: Section 4.5 (Functions, line 223) documents that string arguments support `\\`, `\"`, and `\'` escape sequences. The new condition comparison section (lines 128-138) documents single- and double-quoted string literals but does not mention escape sequence support. The implementation (`parse_cond_value` at `parser.rs:459`) calls `unescape_string()` on condition string values, so escapes are supported in practice.
- Fix: Add a note after the single-quoted string examples, e.g.:
  ```markdown
  Escape sequences (`\\`, `\"`, `\'`) are supported inside both single- and double-quoted comparison literals, matching function argument strings.
  ```

**`@elseif` and `@else` missing from editor highlighting keyword list** - `spec.md:662`
**Confidence**: 90%
- Problem: The TextMate injection grammar roadmap lists keywords for highlighting: `@import`, `@if`, `@for`, `@define`, `@end`, `@export`, `@include`. This PR adds `@elseif` as a first-class directive, but the keyword list was not updated. `@else` was also previously absent.
- Fix: Update line 662 to include `@elseif` and `@else`:
  ```
  adding keyword highlighting for `@import`, `@if`, `@elseif`, `@else`, `@for`, `@define`, `@end`, `@export`, `@include`
  ```

**Grammar production `string_chars` referenced but undefined** - `spec.md:729`
**Confidence**: 82%
- Problem: The new `quoted_string` production references `string_chars` but this non-terminal has no definition in the grammar. Unlike pre-existing undefined terminals (`body`, `arguments`, `params`, `yaml_content`, `raw_text`, `path_chars`) which were already absent before this PR, `quoted_string` is a new production added by this branch. The implementation supports escape sequences (`\\`, `\"`, `\'`) within strings, but `string_chars` does not capture this.
- Fix: Add a production for `string_chars`:
  ```
  string_chars    := (escape_seq | [^"\\] | [^'\\])*
  escape_seq      := "\\" | "\\\"" | "\\'"
  ```
  Or, since this is an informal summary grammar, add a comment: `(supports \\, \", \' escapes)`

## Issues in Code You Touched (Should Fix)

### MEDIUM

**CHANGELOG not updated for new language features** - `CHANGELOG.md:8-12`
**Confidence**: 88%
- Problem: The `[Unreleased]` section in CHANGELOG.md does not mention any of the significant user-facing changes in this PR: negation (`!`), equality comparisons (`==`/`!=`), `@elseif` chains, NaN/Infinity rejection in conditions, or webpack CJS compatibility. These are notable features that users tracking the changelog would expect to see. `applies ADR-002` (verify content addresses linked issues).
- Fix: Add entries under `### Added` in the `[Unreleased]` section:
  ```markdown
  ### Added

  - **Negation in conditionals** — `@if !var:` negates truthy checks
  - **Equality comparisons** — `@if var == "value":` / `@if var != "value":` with strict typing (string, number, boolean, null literals)
  - **`@elseif` chains** — `@if`/`@elseif`/`@else`/`@end` multi-branch conditionals with short-circuit evaluation (max 256 branches)
  - **Single-quoted strings in comparisons** — `@if var == 'value':` accepted alongside double-quoted
  - **NaN/Infinity rejection** — non-finite numbers are rejected as condition values at parse time
  - **Webpack CJS compatibility** — `@mds/webpack-loader` now ships a CJS build for Webpack 5 compatibility
  ```

**README Features list outdated for conditionals** - `README.md:39`
**Confidence**: 85%
- Problem: The README Features section describes conditionals as `` `@if`/`@else`/`@end` blocks `` but does not mention `@elseif`, negation, or equality comparisons. Since `@elseif` and comparisons are significant language features, the README's one-line summary underrepresents the conditional system.
- Fix: Update README.md line 39:
  ```markdown
  - **Conditionals** — `@if`/`@elseif`/`@else`/`@end` blocks with negation (`!`) and equality comparisons (`==`/`!=`)
  ```

## Pre-existing Issues (Not Blocking)

### LOW

**Undefined non-terminal productions in grammar** - `spec.md:695-731`
**Confidence**: 80%
- Problem: Several non-terminals referenced in the grammar summary are not defined: `body`, `arguments`, `params`, `yaml_content`, `raw_text`, `path_chars`. While the grammar is labeled a "summary" (not a formal specification), these omissions reduce its value as a reference. This is pre-existing and not introduced by this PR.
- Fix: Either add informal definitions or note that the grammar is a structural overview with some terminals left unspecified.

## Suggestions (Lower Confidence)

- **Condition grammar ambiguity** - `spec.md:713` (Confidence: 65%) — The `condition` production alternatives overlap: `dot_path` is a prefix of `dot_path ("==" | "!=") cond_value`. While this is an informal grammar and the parser resolves via longest-match, a PEG-style ordered choice notation or a comment noting parse priority would improve clarity.

- **Complete example could showcase new features** - `spec.md:539-589` (Confidence: 62%) — The Complete Example in section 8 uses only basic `@if`/`@else` but does not demonstrate any of the new features (negation, equality, `@elseif`). Adding a conditional with comparison or `@elseif` would make the example more representative of the current feature set.

- **Section 12 Status unchanged after feature additions** - `spec.md:737` (Confidence: 70%) — Section 12 says "v0.1 -- Initial release. The compiler is feature-complete as described in this specification." After adding negation, equality, and `@elseif`, this may need a version bump or note indicating these are post-0.1 additions.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 3 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 1 |

**Documentation Score**: 6/10
**Recommendation**: CHANGES_REQUESTED

The spec.md documentation for the new conditional features is thorough and well-structured -- the rules, examples, and grammar updates are internally consistent with the implementation. However, the CHANGELOG lacks entries for these significant user-facing features, the README feature list is stale, the editor highlighting keyword list omits `@elseif`/`@else`, and the new `quoted_string` grammar production references an undefined `string_chars` terminal. Escape sequence support in condition string literals is also undocumented despite being implemented.

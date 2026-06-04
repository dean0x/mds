# Documentation Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicate `### Added` section in CHANGELOG.md** - `CHANGELOG.md:10` and `CHANGELOG.md:24`
**Confidence**: 95%
- Problem: The `[Unreleased]` section now contains two `### Added` subsections. Lines 10-16 add the new features (negation, equality, @elseif, NaN rejection, CJS build) in a new `### Added` block, but lines 24-43 already contain an `### Added` block from previous unreleased work (LazyInit, API surface tests, bundler packages, @mds/mds). Keep a Changelog format requires a single `### Added` subsection per release. This will render confusingly and violates the linked format specification.
- Fix: Merge the two `### Added` sections into one. Move the five new entries (lines 12-16) into the existing `### Added` block (after line 24), or consolidate both into a single `### Added` section under `[Unreleased]`.

### MEDIUM

**`string_chars` grammar rule is ambiguous / incorrect** - `spec.md:732`
**Confidence**: 90%
- Problem: The grammar production `string_chars := (escape_seq | [^"\\] | [^'\\])*` uses alternation between `[^"\\]` and `[^'\\]` in the same choice set. This is ambiguous: the production does not distinguish whether the enclosing delimiter is `"` or `'`. As written, it implies that inside a double-quoted string, single quotes are also excluded (via `[^'\\]`), and vice versa. In practice, the parser allows `'` inside `"..."` and `"` inside `'...'`. The grammar should use separate productions or a note clarifying that the character class depends on the enclosing quote.
- Fix: Split into context-dependent productions or use a parameterized note:
  ```
  quoted_string   := '"' dq_chars '"' | "'" sq_chars "'"
  dq_chars        := (escape_seq | [^"\\])*
  sq_chars        := (escape_seq | [^'\\])*
  ```

**`TypeScript/JS integration or runtime bindings` still listed as NOT in v0.1** - `spec.md:683`
**Confidence**: 85%
- Problem: Section 10 ("What's NOT in v0.1") still lists "TypeScript/JS integration or runtime bindings" as deferred, but the CHANGELOG already documents `@mds/mds` npm package with full JS/TS bindings (compile, check, compileFile, etc.) and bundler integration packages (@mds/vite-plugin, @mds/rollup-plugin, @mds/webpack-loader). The spec contradicts the actual shipped feature set. Since the entire spec body was authored on this branch, this is an inconsistency within the PR's own documentation.
- Fix: Remove or update the line. Either delete it entirely, or update it to reflect what remains unimplemented (e.g., "Structured JSON output (chat message arrays)") and note that JS/TS bindings now exist.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Section 12 "Status" still says v0.1 is feature-complete "as described in this specification"** - `spec.md:741`
**Confidence**: 82%
- Problem: The status line reads "v0.1 -- Initial release. The compiler is feature-complete as described in this specification." However, this PR adds @elseif, negation, and equality comparisons to the spec, making the spec describe features beyond the original v0.1 release. The status section does not clarify that these are unreleased additions. A reader following the spec may believe these features shipped in v0.1, while they are actually part of the unreleased work on this branch.
- Fix: Either update the status to reference the unreleased additions, or add a note like: "v0.1 -- Initial release. Unreleased additions: negation, equality comparisons, @elseif chains (see CHANGELOG)."

## Pre-existing Issues (Not Blocking)

No pre-existing documentation issues of CRITICAL severity were identified.

## Suggestions (Lower Confidence)

- **Complete example does not demonstrate new features** - `spec.md:541-591` (Confidence: 65%) -- Section 8 ("Complete Example") uses only `@if premium:` / `@else:` but does not demonstrate any of the newly added features (@elseif, negation, equality comparison). Adding a small example using these would help users discover them from the spec alone.

- **`number` production may be incomplete for negative decimals** - `spec.md:717` (Confidence: 70%) -- The grammar `number := "-"? [0-9]+ ("." [0-9]+)?` does not mention the NaN/Infinity rejection documented in the CHANGELOG and enforced in the parser (`parse_cond_value`). A brief note or a "not: NaN, Infinity" constraint would make the grammar self-documenting.

- **CHANGELOG new `### Added` entries could mention spec section** - `CHANGELOG.md:12-16` (Confidence: 60%) -- The equality comparison and @elseif entries describe syntax but do not cross-reference spec.md sections (4.3) where the full rules are documented. This would help consumers navigate to the detailed rules.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The documentation additions are thorough and well-structured -- the spec updates for @elseif, negation, and equality comparisons are detailed with clear examples and edge-case rules. The CHANGELOG, README, and grammar summary are all updated. However, the duplicate `### Added` section in the CHANGELOG violates Keep a Changelog format (applies ADR-002 -- PR content should accurately reflect the state of changes before merge), and the spec contains an internal contradiction where section 10 lists JS/TS bindings as deferred while the project already ships them. The `string_chars` grammar ambiguity is a technical inaccuracy that could mislead implementers. None of these are difficult to fix.

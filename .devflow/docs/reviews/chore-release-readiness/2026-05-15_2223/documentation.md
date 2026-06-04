# Documentation Review Report

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15

## Issues in Your Changes (BLOCKING)

### CRITICAL

**CHANGELOG escape syntax contradicts spec and code** - `CHANGELOG.md:22`
**Confidence**: 98%
- Problem: The CHANGELOG states "Escaped braces (`{{` produces `{`)" but the actual escape syntax is `\{` (backslash-brace). The spec section 4.2 correctly documents this as "Escaping: `\{` produces a literal `{` in output" and the lexer's `scan_escape` method implements `\{`. The CHANGELOG will actively mislead users copying the syntax from the release notes.
- Fix: Change line 22 of CHANGELOG.md from:
  ```
  - Escaped braces (`{{` produces `{`)
  ```
  to:
  ```
  - Escaped braces (`\{` produces `{`)
  ```

### HIGH

**CHANGELOG Library API section omits public functions** - `CHANGELOG.md:49-53`
**Confidence**: 85%
- Problem: The "Library API" section lists 7 functions but omits 4 public API functions that are part of the v0.1.0 release: `compile_str_with()`, `check_str_with()`, `compile_str_collecting_warnings()`, and `check_str_collecting_warnings()`. For a release changelog that explicitly sections out the library API, omitting these creates an incomplete public API record. Users looking at the changelog to understand what's available will miss the configurable string-based variants.
- Fix: Expand the Library API section:
  ```markdown
  **Library API** (`mds` crate)
  - `compile()`, `compile_str()`, `compile_str_with()`, `compile_file()` -- render to String
  - `check()`, `check_str()`, `check_str_with()` -- validate without rendering
  - `compile_collecting_warnings()`, `compile_str_collecting_warnings()`, `check_collecting_warnings()`, `check_str_collecting_warnings()` -- for callers who want structured warning output
  - `load_vars_file()` -- load runtime variables from JSON
  ```

### MEDIUM

**README lists --quiet under "Build options" but it is a global CLI flag** - `README.md:59`
**Confidence**: 82%
- Problem: The README's CLI Reference section lists `-q, --quiet` under "Build options:" but in the code it is defined as `#[arg(long, short = 'q', global = true)]` on the `Cli` struct (main.rs:242), meaning it applies to all commands including `check` and `init`. Placing it under "Build options" implies it only works with `mds build`.
- Fix: Either move `--quiet` to a separate "Global options" section above the command-specific options, or add a note: "Available for all commands."

**README --quiet description is less specific than spec** - `README.md:59`
**Confidence**: 80%
- Problem: The README says "Suppress status messages" while the spec (line 374) says "Suppress status messages and warnings on stderr." The README omits that warnings are also suppressed, which is an important behavioral detail for users piping output.
- Fix: Change to "Suppress status messages and warnings" to match the spec.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Spec does not document single-quote string literal syntax** - `spec.md` (Section 4.5 / Section 11 Grammar)
**Confidence**: 82%
- Problem: The CHANGELOG (line 24) lists "String literal arguments with single-quote delimiters" as a feature, and the parser (parser.rs:634) accepts both `'string'` and `"string"` as delimiters for function arguments. However, the spec section 4.5 only shows double-quoted examples (`{greet("Alice")}`), and the grammar section 11 does not define single-quote string literals. Since the spec is being updated in this branch (section 7 expansion), this is a good time to align.
- Fix: Add to spec section 4.5 Rules: "String arguments accept both double-quote and single-quote delimiters: `{greet('Alice')}` or `{greet(\"Alice\")}`" and update the grammar to include `single_quoted_path`.

**Spec section 7.3 omits --quiet availability for check command** - `spec.md:394`
**Confidence**: 80%
- Problem: Section 7.3 says "Same `--vars`/`--set` options as `mds build`" but `--quiet` is also available for `mds check` as a global flag. Since section 7.2 documents `--quiet` explicitly, users may incorrectly assume it doesn't work with `check`.
- Fix: Change to "Same `--vars`/`--set`/`--quiet` options as `mds build`" or add a note about the global `--quiet` flag.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Spec grammar omits `\}` escape** - `spec.md:621`
**Confidence**: 85%
- Problem: The grammar defines `escaped_brace := "\{"` but the lexer also handles `\}` (lexer.rs:228-231), producing a literal `}` as a Text token. This escape is undocumented in both the spec grammar and the prose in section 4.2.
- Fix: Update grammar to `escaped_brace := "\{" | "\}"` and mention `\}` in section 4.2 Rules.

### LOW

**README Library Usage section omits re-exported types** - `README.md:68-83`
**Confidence**: 65% (moved to Suggestions)

## Suggestions (Lower Confidence)

- **README omits re-exported types in Library Usage** - `README.md:68-83` (Confidence: 65%) -- The public API also re-exports `MdsError`, `Value`, and `MAX_FILE_SIZE`, which are needed for proper error handling and variable construction. A brief mention or import line in the examples would help library users.

- **Cargo.toml repository URL points to "mdl" not "mds"** - `Cargo.toml:9` (Confidence: 62%) -- The repository URL is `https://github.com/deanshrn/mdl` while the crate name is `mds`. This may be intentional (repo name != crate name), but could confuse users looking for the crate by name on GitHub.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 1 | 1 | 2 | - |
| Should Fix | - | - | 2 | - |
| Pre-existing | - | - | 1 | - |

**Documentation Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The documentation package is substantial and well-structured for a v0.1.0 release. The README, spec expansion, CHANGELOG, LICENSE, and Cargo.toml metadata are all present and largely accurate. The critical issue is the CHANGELOG's incorrect escape syntax (`{{` vs `\{`) which directly contradicts both the spec and the implementation -- this will mislead users copying from release notes. The remaining issues are completeness gaps (omitted API functions, undocumented global flag scope) rather than accuracy problems. Fixing the CRITICAL and HIGH items is straightforward and should not delay the release.

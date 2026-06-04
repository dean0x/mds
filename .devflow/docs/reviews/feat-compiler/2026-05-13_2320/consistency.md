# Consistency Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13

## Issues in Your Changes (BLOCKING)

### HIGH

**Inconsistent error variant usage for resource limit errors (4 occurrences)** - Confidence: 90%
- `src/evaluator.rs:91`, `src/evaluator.rs:321`, `src/evaluator.rs:341`, `src/resolver.rs:129`
- Problem: Resource limit errors (output size exceeded, array iteration limit, total iteration limit, file too large) are all constructed as `MdsError::Io { message }` despite not being I/O errors. The `Io` variant is used as a catch-all for operational limit violations, which is semantically inconsistent with actual I/O errors (e.g., file read failures at `resolver.rs:125`). Meanwhile, other domain-specific errors like recursion limits, circular imports, and arity mismatches each have their own dedicated variant. Resource limits are a distinct error class and should not masquerade as I/O errors.
- Fix: Introduce a dedicated `MdsError::ResourceLimit { message }` variant (with diagnostic code `mds::resource_limit`) and use it for all resource-bound violations. This aligns with the existing pattern where each distinct error category has its own variant.

### MEDIUM

**Missing `_at` constructor for `Recursion` variant** - Confidence: 85%
- `src/error.rs:317-323`
- Problem: The `MdsError` type follows a consistent dual-constructor pattern: a spanless constructor (e.g., `syntax()`, `undefined_var()`, `file_not_found()`) paired with a span-aware variant (e.g., `syntax_at()`, `undefined_var_at()`, `file_not_found_at()`). The `Recursion` variant has only `recursion()` (spanless) but no `recursion_at()`, despite the variant having `span` and `src` fields. By contrast, `CircularImport` has a `circular_import_at()` but no spanless `circular_import()`. Both break the dual-constructor pattern established by all other variants.
- Fix: Add `recursion_at()` to match the pattern, and add a spanless `circular_import()` constructor. This makes the error API uniformly dual-form.

**Missing `_at` constructor for `ExportError` variant** - Confidence: 85%
- `src/error.rs:386-392`
- Problem: `ExportError` has `span` and `src` fields in the enum variant, but only a spanless `export_error()` constructor. No `export_error_at()` exists, despite the variant supporting source spans. This is inconsistent with the pattern established by `import_error` / `import_error_at`, `name_collision` / `name_collision_at`, etc.
- Fix: Add an `export_error_at()` constructor following the same signature pattern as `import_error_at()`.

**Missing `#[must_use]` on `check` function** - Confidence: 82%
- `src/lib.rs:139`
- Problem: The `check()` function has `#[must_use = "errors should be handled"]` which is correct. However, `compile_file()` (line 314) also has `#[must_use]` while it merely delegates to `compile()` which already has `#[must_use]`. This is fine but creates a minor inconsistency: `check_str_with()` at line 194 and `check_str()` at line 167 both have `#[must_use]` while `load_vars_file()` at line 332 does not, despite returning a `Result` that should also not be silently discarded. The pattern is applied inconsistently across the public API.
- Fix: Add `#[must_use = "the loaded variables should be used"]` to `load_vars_file()` to match the pattern used by the other public functions.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Inconsistent error construction: direct variant vs constructor** - Confidence: 82%
- `src/resolver.rs:99-103` vs `src/resolver.rs:108`
- Problem: Within the same function (`ModuleCache::resolve`), circular import errors are constructed by directly instantiating the enum variant (`MdsError::CircularImport { cycle, span: None, src: None }`) while the adjacent import-depth error uses the constructor function (`MdsError::import_error(format!(...))`) and file-not-found at line 83 uses `MdsError::file_not_found(...)`. Mixing raw variant construction with constructor functions in the same function body is inconsistent. The constructors exist precisely to avoid this boilerplate.
- Fix: The `CircularImport` variant at line 99 should use a spanless constructor. After adding `circular_import()` as suggested above, replace lines 99-103 with: `return Err(MdsError::circular_import(cycle));`

**Token enum uses unnamed tuple fields while AST types use named struct fields** - Confidence: 80%
- `src/lexer.rs:5-22` vs `src/ast.rs:1-134`
- Problem: The `Token` enum uses unnamed tuple variants (e.g., `Text(String, usize)`, `Interpolation(String, usize)`) where the `usize` represents a byte offset. The AST types use named fields (e.g., `IfBlock { condition, then_body, else_body, offset }`). This creates an inconsistency between the two layers of the compiler: tuple indexing (`.0`, `.1`) in lexer-consumer code vs named field access (`.offset`) in parser code. The semantic meaning of the `usize` field in Token variants is unclear without context.
- Fix: Consider converting Token variants to named fields (e.g., `Text { content: String, offset: usize }`) for self-documenting code. This is a low-urgency improvement that would improve readability across the lexer/parser boundary.

## Pre-existing Issues (Not Blocking)

(None -- all code is new on this branch.)

## Suggestions (Lower Confidence)

- **`find_project_root` loop has no depth bound** - `src/resolver.rs:16-28` (Confidence: 65%) -- The `loop` walks the filesystem upward until it finds `.git`/`.mdsroot` or runs out of parent directories. While `Path::pop()` will eventually return `false` at the root, adding an explicit iteration bound (e.g., 256 levels) would align with the project's general bounded-resource approach (MAX_IMPORT_DEPTH, MAX_CALL_DEPTH, MAX_NESTING_DEPTH, etc.).

- **`parse_cli_value` bracket-list parsing doesn't coerce element types** - `src/main.rs:144-154` (Confidence: 62%) -- The `[a, b, c]` syntax always produces `Value::String` elements even when values like `true`, `42`, or `null` appear inside brackets. The function's own doc comment says it matches YAML frontmatter semantics, but YAML would coerce these. This is a minor semantic inconsistency between `--set items=[1,2,3]` (all strings) and YAML `items: [1, 2, 3]` (all numbers).

- **Inconsistent use of `eprintln!` for warnings vs structured approach** - `src/lib.rs:209-213` (Confidence: 60%) -- Warnings are collected as `Vec<String>` and printed via `eprintln!`. Some code paths (like `evaluate_include`) create warning strings with a `"warning: "` prefix baked in, while the `emit_warnings` function adds no prefix. The warning strings are not structured, so downstream consumers cannot distinguish severity or type.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 3 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Consistency Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The codebase demonstrates strong overall consistency: Rust naming conventions are followed throughout (snake_case for functions/variables, CamelCase for types/enums), the error handling pattern is almost uniformly Result-based with no panics in business logic, and the public API style is coherent across `compile`, `compile_str`, `check`, and `check_str` function families. The main consistency gaps are (1) the `MdsError::Io` variant being reused as a catch-all for resource limits, which muddies error categorization, and (2) a few missing `_at` constructors that break the otherwise clean dual-constructor pattern in the error module. These should be addressed before merge to keep the error API clean as the codebase grows.

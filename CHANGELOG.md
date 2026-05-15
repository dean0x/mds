# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-15

Initial release of the MDS (Markdown Script) compiler.

### Added

**Language features**
- Variable interpolation from YAML frontmatter (`{name}`)
- `@if`/`@else`/`@end` conditionals with full MDS truthiness rules
- `@for item in list:` loops over arrays
- `@define` function definitions with parameters and lexical scoping
- `@import` directives: alias (`as ns`), merge, and selective (`{ a, b }`)
- `@export` directives: named, re-export from module, wildcard re-export
- `@include ns` to inline the prompt body of an imported module
- Escaped braces (`{{` produces `{`)
- Frontmatter `type: mds` marker to allow `.md` files as MDS sources
- String literal arguments with single-quote delimiters

**Compiler pipeline**
- Lexer with token types for all MDS syntax elements
- Recursive-descent parser producing a typed AST
- Module resolver with `Arc<ResolvedModule>` caching and cycle detection
- Semantic validator (undefined variables/functions, arity, type checks)
- Evaluator with `EvalContext` threading (call stack, iteration counting, warnings)
- `mds.json` project config with `build.output_dir`

**CLI** (`mds` binary)
- `mds build` — compile `.mds` to Markdown with auto-detection, `--out-dir`, `--set`, `--vars`
- `mds check` — validate without rendering
- `mds init` — create a starter template
- Stdin mode (`mds build -`)
- Categorized exit codes (0/1/2/3)
- Rich miette diagnostics with source spans

**Security**
- Path traversal prevention for imports and config `output_dir`
- Symlink rejection in import paths
- File size limits (10 MB per file, 1 MB for `mds.json`)
- Resource limits: call depth (128), loop iterations (100 K per loop, 1 M total), output size (50 MB), warnings (1000)
- YAML/JSON value nesting depth limit (64 levels)

**Library API** (`mds` crate)
- `compile()`, `compile_str()`, `compile_file()` — render to String
- `check()`, `check_str()` — validate without rendering
- `compile_collecting_warnings()`, `check_collecting_warnings()` — for callers who want structured warning output
- `load_vars_file()` — load runtime variables from JSON

**Tests**
- 292 tests covering integration, unit, and doc-tests

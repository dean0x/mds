# Architecture Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**PR**: #1

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`CollectedDefs` tuple type alias reduces readability and violates information hiding** - `src/resolver.rs:512`
**Confidence**: 85%
- Problem: `type CollectedDefs = (HashMap<String, Arc<FunctionDef>>, bool, HashSet<String>);` is a positional tuple whose fields are identified only by position. At the single call site in `process_module`, the destructure reads `let (functions, has_explicit_exports, explicit_exports) = ...` which is clear, but the tuple type itself has no field names and the meaning of the `bool` element is opaque from the type signature alone. This is a "shallow module" pattern per Ousterhout -- the abstraction hides less than it exposes.
- Fix: Replace the tuple alias with a named struct:
  ```rust
  struct CollectedDefs {
      functions: HashMap<String, Arc<FunctionDef>>,
      has_explicit_exports: bool,
      explicit_exports: HashSet<String>,
  }
  ```
  This makes `collect_definitions_and_imports` self-documenting without needing to look at the destructure site.

**`load_config` and `resolve_output_path` live in `main.rs` alongside CLI plumbing -- limits reusability** - `src/main.rs:33-157`
**Confidence**: 82%
- Problem: `load_config` (project config discovery) and `resolve_output_path` (output precedence chain) are pure domain logic that could be useful to library consumers or a future language server, but they are private functions in the binary crate. This couples project-config awareness to the CLI entry point. Currently `lib.rs` has no concept of `mds.json` or output path resolution, so any programmatic user would have to reimplement this logic.
- Fix: Consider extracting `load_config`, `MdsConfig`, `BuildConfig`, `derive_output_filename`, and `resolve_output_path` into a `config` module in the library crate (or at minimum a separate file within `src/`). This is not urgent for v0.1 but will become a pain point if you add an LSP, watch mode, or programmatic build API. Marking as MEDIUM because the current scope is CLI-only and the functions are well-tested where they are.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`process_module` still takes 6 parameters despite `ModuleCtx` introduction** - `src/resolver.rs:229-237`
**Confidence**: 83%
- Problem: `ModuleCtx` was introduced to bundle context, yet `process_module` still takes `source`, `file_str`, `base_dir`, `is_md`, `runtime_vars`, `warnings` as separate parameters -- then constructs `ModuleCtx` internally at line 246. The method could instead accept `ModuleCtx` directly (or a slightly expanded version that includes `source`), reducing its arity from 6 to 3-4 parameters. The `is_md` boolean is the only field that does not fit into `ModuleCtx` (it is a derived property of the current file, not reusable context).
- Fix: Extend `ModuleCtx` with a `source` field (already borrows it) and pass it into `process_module`:
  ```rust
  fn process_module(
      &mut self,
      ctx: &ModuleCtx<'_>,
      is_md: bool,
      warnings: &mut Vec<String>,
  ) -> Result<ResolvedModule, MdsError> { ... }
  ```
  This would make the decomposition cleaner and match the intent of the `ModuleCtx` abstraction.

**`resolve_output_path` has a side effect (directory creation) mixed with pure path resolution** - `src/main.rs:97-157`
**Confidence**: 80%
- Problem: `resolve_output_path` performs `std::fs::create_dir_all` in steps 4 and 5 (lines 127 and 140). A function named `resolve_*` conventionally performs pure computation -- resolving a value from inputs. Mixing filesystem side effects into it violates SRP and makes the function harder to test (the existing tests work around this with `tempfile::tempdir`). The `run()` function already has its own `create_dir_all` call at line 456 for the parent directory of the resolved path.
- Fix: Remove the `create_dir_all` calls from `resolve_output_path` and move all directory creation to the caller (`run()`). This makes `resolve_output_path` a pure function, testable without touching the filesystem. The existing `create_dir_all` at line 456 already handles the parent directory case.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`ResolvedModule` has public fields but also accessor methods with filtering logic** - `src/resolver.rs:36-41, 450-509`
**Confidence**: 85%
- Problem: `ResolvedModule` exposes `functions`, `prompt_body`, `has_explicit_exports`, and `explicit_exports` as public fields, yet also provides `get_export`, `get_all_exports`, `get_prompt_value`, and `to_namespace` which apply export-visibility filtering. Consumers can bypass the filtering by directly accessing the public fields. This is a leaky abstraction where the struct's invariant (export visibility) is not enforced by its API. The `to_namespace` bug fix (prompt_body visibility) in this PR demonstrates the risk: code that accessed fields directly would still have the old broken behavior.
- Fix: Make the fields `pub(crate)` and ensure all external access goes through the accessor methods. Since `ResolvedModule` is in a `pub(crate)` module (`resolver`), this is low-impact but would prevent future regressions.

## Suggestions (Lower Confidence)

- **`evaluate_for` pop-before-result pattern** - `src/evaluator.rs:287-289` (Confidence: 70%) -- The `@for` loop calls `scope.pop()?` before unwrapping `rendered?`. This was intentionally reordered to ensure scope cleanup on error, but the error from `scope.pop()` could shadow the original rendering error. Consider whether the `pop` error or the `rendered` error is more important to surface.

- **`MdsError` derives `Clone` with `Arc<NamedSource<String>>` fields** - `src/error.rs:21` (Confidence: 65%) -- Adding `Clone` to `MdsError` is convenient but errors are not typically cloned in the happy path. The `Arc` wrapping of `NamedSource` was likely needed to satisfy the `Clone` bound. If `Clone` was added only for the `modules` cache (which now stores `Arc<ResolvedModule>`), it may no longer be needed since `ResolvedModule` no longer stores `MdsError` values.

- **`validate_and_read_file` initializes `root_dir` but `resolve` also checks cache/cycle after** - `src/resolver.rs:71-153, 155-200` (Confidence: 62%) -- The split between `validate_and_read_file` and `resolve` means that some checks (cache, cycle) happen after file I/O. If a cache hit is found, the file was already read unnecessarily. The ordering is: read file -> check cache -> check cycle. Consider checking cache before I/O. However, this may be intentional because canonicalization (needed for cache key) requires the file to exist.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Rationale

This PR represents a strong architectural improvement across the board:

1. **EvalContext struct** -- Excellent application of the "Parameter Object" pattern. Bundling `call_stack`, `total_iterations`, and `warnings` into `EvalContext` reduces function arity throughout the evaluator from 5-7 parameters to 3, making the code significantly more maintainable. The `Vec<String>` call stack with `debug_assert!` LIFO verification is a clean tradeoff.

2. **Lexer decomposition** -- Converting the monolithic `tokenize()` closure-and-loop into a `Lexer` struct with `scan_*` methods is textbook Deep Module design. Each `scan_*` method has a clear precondition, a focused responsibility, and advances `self.pos`. The public API (`tokenize`) is unchanged -- callers see no difference.

3. **`Arc<FunctionDef>` and `Arc<ResolvedModule>`** -- Good application of shared ownership to eliminate redundant cloning in the module cache and scope system. The deliberate choice to use owned `FunctionDef` (not `Arc`) in `CapturedScope` to break reference cycles shows careful cycle analysis.

4. **`CapturedScope` struct** -- Consolidating three separate `captured_*` fields into one struct with a `Default` impl is clean and reduces the surface area for errors.

5. **`IndexSet` replacing `HashSet + Vec`** -- Eliminating the dual data structure for cycle detection by using `IndexSet` (O(1) lookup + insertion order) is an elegant simplification that reduces the risk of the two structures getting out of sync.

6. **Resolver decomposition** -- `process_module` is now a ~25-line orchestrator calling `build_scope_from_frontmatter`, `collect_definitions_and_imports`, `validate_exports`. Each helper has a single responsibility and is independently testable. The `ModuleCtx` pattern reduces parameter threading.

7. **CLI exit codes** -- Clean separation of error categorization (`exit_code`) from error reporting. The downcast-based approach is idiomatic for miette.

The conditions for approval are minor: the `CollectedDefs` tuple alias should be converted to a named struct (low effort, high readability payoff), and the directory-creation side effects in `resolve_output_path` should be moved to the caller. Both are straightforward changes that would complete the decomposition work already started in this PR.

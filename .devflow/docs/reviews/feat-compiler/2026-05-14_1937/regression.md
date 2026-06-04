# Regression Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14
**Diff**: `git diff 97b478f...HEAD` (15 files, +2475/-887 lines)

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Default output changed from stdout to file -- intentional breaking change for CLI consumers** - `src/main.rs:452-474`
**Confidence**: 95%
- Problem: `mds build foo.mds` previously wrote compiled output to stdout. It now writes to `foo.md` next to the source file by default. Any existing scripts, CI pipelines, or tool integrations that pipe stdout from `mds build` will silently get empty stdout and an unexpected file written to disk. The help text was updated (`-o -` for stdout), but there is no deprecation warning, no `CHANGELOG.md` entry, and no migration guide.
- Impact: Scripts like `result=$(mds build template.mds)` or `mds build template.mds | pbcopy` will break silently -- they receive empty string instead of compiled output.
- Fix: This is an intentional design change (documented in KNOWLEDGE.md gotchas and tested). However, since this is a pre-1.0 CLI, the risk is acceptable. Still, a `CHANGELOG.md` entry or release note documenting the breaking change would prevent user confusion. Consider whether a deprecation warning (e.g., "writing to file by default; use -o - for stdout") for one release cycle is warranted.

**`to_namespace()` export visibility fix changes behavior for alias-imported modules** - `src/resolver.rs:483-508`
**Confidence**: 92%
- Problem: `to_namespace()` previously exposed `prompt_body` unconditionally in the `NamespaceScope`. Now it respects export visibility -- modules with explicit exports that do not list `"prompt"` will no longer expose their body text via `@include alias`. Any `.mds` template that relies on `@include` of an alias-imported module with explicit exports (without listing `"prompt"`) will now get empty output where it previously got the module body.
- Impact: This is a bug fix (documented in the PR and KNOWLEDGE.md), but it changes observable behavior for existing templates.
- Fix: This is correct behavior (tested in `include_respects_export_visibility_for_prompt`). The fix is intentional and the test validates it. No code change needed -- but document this as a behavioral fix in release notes so template authors can audit their imports.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`serde_yaml` replaced with `serde_yml` 0.0.12 -- pre-1.0 dependency** - `Cargo.toml:11`
**Confidence**: 82%
- Problem: The YAML parsing dependency was changed from `serde_yaml` (0.9, widely used) to `serde_yml` (0.0.12, a pre-1.0 crate). While `serde_yaml` is unmaintained, `serde_yml` at version 0.0.12 signals API instability. The API surface used here (`from_str`, `Value::Mapping`, `Value::Tagged`, etc.) is straightforward and unlikely to change, but a version bump could introduce breaking API changes.
- Impact: Low immediate risk since the project pins to `0.0.12` in the lockfile. But future `cargo update` could pull a breaking `0.0.x` release.
- Fix: Consider pinning the version more tightly in `Cargo.toml` with `= "0.0.12"` or `">=0.0.12, <0.1.0"` to prevent unexpected breakage on `cargo update`. Alternatively, verify the crate's release cadence and API stability commitment.

**`MdsError` now derives `Clone` -- enlarges the implicit API contract** - `src/error.rs:21`
**Confidence**: 80%
- Problem: Adding `Clone` to `MdsError` is a new capability that downstream consumers could start relying on. While not breaking per se, removing `Clone` later (e.g., if a non-Clone field is added) would be a semver-breaking change. The `Arc<NamedSource>` fields make `Clone` cheap, but this should be an intentional API decision.
- Impact: Low risk for an internal project, but worth noting as an API contract expansion.
- Fix: Acceptable as-is since all fields use `Arc` for source code references. Just be aware this becomes part of the public API contract.

## Pre-existing Issues (Not Blocking)

(none found at CRITICAL severity in unchanged code)

## Suggestions (Lower Confidence)

- **`evaluate_for` changed error/pop ordering** - `src/evaluator.rs:287-289` (Confidence: 65%) -- The old code propagated evaluation errors immediately (`let rendered = ...?`) before `scope.pop()`, which could leak a scope frame on error. The new code calls `scope.pop()` before `rendered?`, which is actually an improvement for scope cleanup. However, this changes which error is returned if BOTH `evaluate_nodes` and `scope.pop()` fail: now `scope.pop()` error takes priority. In practice `scope.pop()` only fails on the global frame (a compiler bug), so this is unlikely to matter.

- **`call_stack` changed from `HashSet` to `Vec` for recursion detection** - `src/evaluator.rs:159` (Confidence: 65%) -- Recursion detection now uses `Vec::iter().any()` (O(n)) instead of `HashSet::contains` (O(1)). At `MAX_CALL_DEPTH=128` this is negligible. However, `HashSet` rejects duplicate insertions silently while `Vec::push` always succeeds -- the `debug_assert!` on pop verifies LIFO correctness but only in debug builds. The new approach is fine given the depth limit, but the Vec allows the same function name to appear multiple times if direct recursion detection fails. The `iter().any()` check prevents this in practice.

- **Error message changed: "not supported in MDS v0.1" to "not supported"** - `src/value.rs:60,92` (Confidence: 70%) -- The YAML mapping and JSON object rejection messages dropped the "in MDS v0.1" qualifier. If downstream code matches on error message text, this could be a minor issue. Since `MdsError` variants are matched by type (not string), this is very low risk.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Conditions

1. The two HIGH findings are intentional behavioral changes, not accidental regressions. Both are tested. The condition for approval is that these breaking changes are documented in release notes or a changelog entry before merge, so users can discover the migration path (`-o -` for stdout, audit `@include` of modules with explicit exports).

### Positive Findings

- **No removed public API functions**: All 12 public functions in `lib.rs` are preserved with identical signatures.
- **No removed CLI commands or options**: `build`, `check`, `init` all preserved. New `--out-dir` option added.
- **No removed test functions**: All existing tests preserved; 21+ new tests added.
- **All 276 tests pass**: Zero failures.
- **Scope cleanup improved**: `evaluate_for` now pops scope before propagating errors, preventing frame leaks.
- **Migration complete**: `serde_yaml` -> `serde_yml` fully migrated with no stale references.
- **`IndexSet` replaces `HashSet + Vec` pair**: Simpler, correct cycle detection with preserved ordering.
- **`Arc<FunctionDef>` and `Arc<ResolvedModule>`**: O(1) cloning replaces deep clones throughout resolver/evaluator.
- **`EvalContext` bundles threaded state**: Reduces parameter count across all evaluator functions without changing behavior.
- **`CapturedScope` struct**: Clean consolidation of three separate fields into one struct.
- **Exit codes added**: Structured exit codes (0/1/2/3) enable programmatic error handling by callers.
- **`check --quiet` fixed**: `check` command now properly gates warnings on the quiet flag.
- **New `check_collecting_warnings` / `check_str_collecting_warnings` API**: Extends the two-tier warning pattern to validation functions.

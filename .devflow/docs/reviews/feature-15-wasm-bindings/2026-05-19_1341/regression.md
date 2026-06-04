# Regression Review Report

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19

## Issues in Your Changes (BLOCKING)

### HIGH

**`.gitignore` entries for `.memory/` and `.docs/` removed without stated intent** - `.gitignore:1-2`
**Confidence**: 92%
- Problem: The commit removes two `.gitignore` entries (`.memory/` and `.docs/`) that were present on `main`. These directories are local-only tooling artifacts (devflow memory and docs directories). Without the gitignore entries, `.memory/` (which exists in the worktree) will now appear as untracked and could be accidentally committed, leaking local devflow state into the repository. The commit message makes no mention of this change, indicating it was likely unintentional — a side effect of rewriting the `.gitignore` file rather than appending to it.
- Fix: Restore the removed entries while keeping the new `crates/mds-wasm/pkg/` entry:
```gitignore
.memory/
.docs/
/target
crates/mds-wasm/pkg/
```

### MEDIUM

**Workspace-wide `panic = "unwind"` affects all crates, not just mds-wasm** - `Cargo.toml:29-34`
**Confidence**: 85%
- Problem: The `[profile.dev]` and `[profile.release]` sections set `panic = "unwind"` at the workspace level. While this is required for `catch_unwind` in the WASM crate, it changes the panic strategy for `mds-core` and `mds-cli` as well. On `main`, no panic profile was set (Rust defaults to `unwind` for dev and `unwind` for release), so in practice this is a no-op for the current default target. However, explicitly locking `panic = "unwind"` in release prevents future use of `panic = "abort"` for the CLI binary (which is a common release optimization for smaller binary size and faster panics). The change is safe today but subtly constrains future release profile tuning for non-WASM crates.
- Fix: Consider scoping the panic setting to only the mds-wasm package using per-package profiles, which is already partially done for `opt-level`/`strip`/`codegen-units`:
```toml
# Remove workspace-level panic overrides if they match defaults:
# [profile.dev]
# panic = "unwind"   # This is already the default
#
# [profile.release]
# lto = true
# panic = "unwind"   # Only needed for mds-wasm; blocks abort for CLI

# Per-package override (already exists):
[profile.release.package.mds-wasm]
opt-level = "z"
strip = true
codegen-units = 1
# panic = "unwind" is inherited from workspace default
```
Note: Cargo does not currently support per-package `panic` overrides — it is a workspace-global setting. The `[profile.release.package.mds-wasm]` section only accepts `opt-level`, `codegen-units`, and `overflow-checks`. If `panic = "abort"` is ever desired for the CLI, the workaround would be a separate `--config` flag at build time. Given this Cargo limitation, the current approach is acceptable but should be documented with a comment explaining why `unwind` is locked.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`load_vars_str` lacks input size bound** - `crates/mds-core/src/lib.rs:759` (Confidence: 65%) -- Unlike `load_vars_file` which enforces `MAX_FILE_SIZE`, `load_vars_str` accepts unbounded input. In the WASM context, input is bounded by JS/WASM memory limits, so this is unlikely to be exploitable. However, for API consistency with `load_vars_file`, a size check could be added.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Regression Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The core changes are additive and well-structured with no regressions to existing functionality:
- `Value::from_json` visibility widened from `pub(crate)` to `pub` -- backward compatible, no existing callers break
- `load_vars_str` is a new public function -- purely additive
- New `mds-wasm` crate is isolated in its own crate with no changes to existing crate behavior
- All 257 existing mds-core tests pass
- The `api_surface.rs` test registry has been updated to include the new `load_vars_str` function
- No exports removed, no signatures changed, no files deleted

The `.gitignore` entry removal is the only blocking concern (HIGH) as it appears unintentional and could lead to accidental commits of local-only devflow state. The workspace-level panic profile is a minor concern worth documenting.

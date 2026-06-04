# Security Review Report

**Branch**: main (d0624a2...HEAD)
**Date**: 2026-05-15
**Focus**: Security

## Issues in Your Changes (BLOCKING)

### MEDIUM

**TOCTOU in mds.json config size check** - `src/main.rs:55-63`
**Confidence**: 82%
- Problem: `load_config()` checks file size via `std::fs::metadata()` then reads the file content with `std::fs::read_to_string()`. A malicious actor with write access to the project directory could swap a small file for a large one between the metadata check and the read, bypassing the 1 MB size guard. Unlike the resolver's `read_validated_file()` which reads bytes first then checks size (TOCTOU-safe), the config loader uses the vulnerable check-then-read pattern.
- Fix: Read the file content first, then check its length -- matching the pattern already used in `resolver.rs:148-157` and `lib.rs:395-403`:
```rust
let bytes = std::fs::read(&candidate).map_err(|e| {
    miette::miette!("cannot read {}: {e}", candidate.display())
})?;
if bytes.len() as u64 > MAX_CONFIG_SIZE {
    return Err(miette::miette!(
        "mds.json at {} is too large ({} bytes; maximum is 1 MB)",
        candidate.display(),
        bytes.len()
    ));
}
let raw = String::from_utf8(bytes).map_err(|e| {
    miette::miette!("invalid UTF-8 in {}: {e}", candidate.display())
})?;
```

### LOW

**Pre-release YAML dependency (serde_yml 0.0.12)** - `Cargo.toml:12`
**Confidence**: 85%
- Problem: `serde_yml` is at version `0.0.12` -- a pre-release with no stability guarantees. YAML parsers have historically been vectors for denial-of-service attacks (billion laughs / YAML bombs, unbounded alias expansion). While the code constrains value depth to 64 levels via `MAX_VALUE_DEPTH`, the `serde_yml` parser processes the YAML before depth limits are enforced in `Value::from_yaml_inner()`. A malicious YAML payload with alias-based expansion could consume excessive memory within `serde_yml::from_str()` before control returns to MDS.
- Fix: Add a comment in `Cargo.toml` noting the risk and pin to a specific version. Consider switching to `serde_yaml2` or evaluating `serde_yml` for YAML bomb resistance. Alternatively, cap the frontmatter raw size before passing it to `serde_yml::from_str()` (the file is already capped at 10 MB, but frontmatter itself has no separate limit).

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none -- this is a new codebase)

## Suggestions (Lower Confidence)

- **`--out-dir` CLI flag has no path traversal protection** - `src/main.rs:153-155` (Confidence: 65%) -- The `--out-dir` flag accepts any path including `..` components. The `mds.json` `output_dir` correctly rejects `..` (line 163-171), but `--out-dir` does not. This is arguably acceptable since the user explicitly provides the CLI flag (unlike `mds.json` which could be committed by another contributor), but the inconsistency could surprise users.

- **No `#![forbid(unsafe_code)]` crate-level attribute** - `src/lib.rs` (Confidence: 62%) -- The codebase contains zero `unsafe` blocks (verified), but adding `#![forbid(unsafe_code)]` at the crate root would enforce this invariant against future contributions and signal security intent to auditors.

- **Symlink detection inherits unavoidable TOCTOU window** - `src/resolver.rs:87-107` (Confidence: 60%) -- The code acknowledges the residual OS-level TOCTOU race in its comments and correctly minimizes the window by comparing two `canonicalize()` calls. This is the best defense available in userspace Rust. Noted for completeness, not actionable.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 1 |
| Should Fix | - | - | 0 | - |
| Pre-existing | - | - | 0 | 0 |

## Security Controls Assessment

The codebase demonstrates strong security awareness for a template compiler. The following controls are already in place:

**Path Traversal Prevention**
- Import paths must start with `./` or `../` (`validate_import_path`, `resolver.rs:624-635`)
- Resolved paths are checked to stay within the project root via `canonicalize()` + `starts_with()` (`resolver.rs:128-135`)
- Null bytes in import paths are rejected (`resolver.rs:631-633`)
- `mds.json` `output_dir` rejects `..` components (`main.rs:163-171`)
- `mds init` filename rejects `..` components (`main.rs:547-553`)

**Symlink Attack Protection**
- Symlinks are detected by comparing canonical parent + filename against fully-canonicalized path (`resolver.rs:87-107`)
- Rejection is tested (`tests/integration.rs:2207-2234`)

**Resource Exhaustion Prevention**
- File size: 10 MB per file (`resolver.rs:47`, `lib.rs:57`)
- Config size: 1 MB for `mds.json` (`main.rs:26`)
- Stdin size: 10 MB (`main.rs:415-429`)
- Output size: 50 MB (`evaluator.rs:19`)
- Single loop iterations: 100,000 (`evaluator.rs:12`)
- Total loop iterations: 1,000,000 (`evaluator.rs:16`)
- Call depth: 128 (`evaluator.rs:9`)
- Import depth: 64 (`resolver.rs:44`)
- Parser nesting depth: 256 (`parser.rs:12`)
- YAML/JSON value depth: 64 (`value.rs:6`)
- Argument nesting depth: 256 (`parser.rs:12`) and 128 (`evaluator.rs:128`)
- Warning accumulation: capped at 1,000 (`evaluator.rs:22`)
- Directory walk for project root: 256 iterations (`resolver.rs:21`)
- Directory walk for config: 256 iterations (`main.rs:51`)

**Cycle Detection**
- Import cycles detected via `IndexSet` with ordered insertion (`resolver.rs:179-183`)
- Function recursion (direct and mutual) detected via call stack (`evaluator.rs:172-174`)
- Both have dedicated tests

**Input Validation**
- Identifiers restricted to ASCII alphanumeric + underscore (`parser.rs:700-707`)
- YAML and JSON parsing are bounded by depth limits
- File type validation requires `.mds` extension or `type: mds` in frontmatter (`resolver.rs:639-666`)

**Memory Safety**
- Zero `unsafe` blocks in the entire codebase
- `Arc<FunctionDef>` used for shared ownership; owned `FunctionDef` in captures to break reference cycles (`scope.rs:9-11`)
- `Scope::pop()` returns `Result` instead of panicking (`scope.rs:87-95`)
- LIFO invariants enforced with `assert_eq!` in resolver and structured error in evaluator

**Security Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The single MEDIUM finding (TOCTOU in config loading) is a minor inconsistency with the project's own established patterns. The pre-release YAML dependency is worth monitoring. Overall, the security posture is strong -- defense-in-depth controls are comprehensive and well-tested with dedicated security integration tests.

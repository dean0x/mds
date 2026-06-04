# Dependencies Review Report

**Branch**: main (d0624a2...HEAD)
**Date**: 2026-05-15

## Summary

The MDS compiler has a lean and well-chosen dependency set (7 runtime + 1 dev-dependency, 90 total packages in the lock file). All dependencies are actively used, licenses are permissive (MIT/Apache-2.0), and the lockfile is committed. The primary concern is using `serde_yml` at a pre-release version (0.0.12) with no SemVer stability guarantees, and the project is missing its own license declaration and MSRV policy.

## Issues in Your Changes (BLOCKING)

### HIGH

**Pre-release YAML dependency (`serde_yml 0.0.12`)** - `Cargo.toml:12`
**Confidence**: 95%
- Problem: `serde_yml` is at version `0.0.12` -- a `0.0.x` version that signals the crate is still experimental with no SemVer stability commitment. The comment in Cargo.toml acknowledges this ("Pre-release (0.0.x); track for 0.1.x stability milestone"), but the crate has not reached 0.1.x. Its sub-dependency `libyml` is also at `0.0.5`. For a project targeting public release, depending on a crate that could introduce breaking changes in any patch bump is a meaningful risk.
- Impact: Any `cargo update` could pull in a breaking `0.0.13` with API changes. The `serde_yml` crate is a relatively new fork/rewrite; ecosystem adoption is still maturing compared to the well-established `serde_yaml` (now archived) or other alternatives.
- Fix: Consider one of:
  1. Pin the exact version `serde_yml = "=0.0.12"` in Cargo.toml to prevent accidental breakage until the crate stabilizes.
  2. Evaluate alternative YAML crates (e.g., `yaml-rust2` with manual serde integration, or monitor `serde_yml` for a 0.1.x release).
  3. At minimum, the existing comment is good -- ensure CI catches breakage early.

### MEDIUM

**Missing project license declaration** - `Cargo.toml:1-5`
**Confidence**: 92%
- Problem: The `Cargo.toml` has no `license` or `license-file` field, and there is no `LICENSE` file in the repository. All 90 dependencies use permissive licenses (MIT, Apache-2.0, Zlib, ISC, Unlicense), but the project itself has no declared license. This will block `cargo publish` (crates.io requires a license) and creates legal ambiguity for users.
- Impact: Cannot publish to crates.io. Contributors and users have no clarity on usage rights. Default copyright (all rights reserved) applies.
- Fix: Add a `license` field to `Cargo.toml` and create a LICENSE file:
  ```toml
  [package]
  license = "MIT OR Apache-2.0"
  ```

**No MSRV (minimum supported Rust version) declared** - `Cargo.toml:1-5`
**Confidence**: 85%
- Problem: No `rust-version` field in Cargo.toml and no `rust-toolchain.toml` file. Users building from source have no guidance on which Rust version is required. The `edition = "2021"` implies at least Rust 1.56, but dependencies like `thiserror 2.x` require Rust 1.65+, and newer dependencies may require even more recent versions.
- Impact: Users on older Rust toolchains will get confusing compilation errors. CI may not test against a defined minimum version.
- Fix: Add to `Cargo.toml`:
  ```toml
  [package]
  rust-version = "1.75"  # or whatever version CI verifies
  ```

## Issues in Code You Touched (Should Fix)

### LOW

**Version range for `serde_yml` wider than necessary** - `Cargo.toml:12`
**Confidence**: 80%
- Problem: The spec `serde_yml = "0.0.12"` in Cargo's version resolution means `>=0.0.12, <0.0.13` for `0.0.x` crates (Cargo treats the leading zero specially). So this is effectively already pinned to `0.0.12` since there's no `0.0.12.x` patch. This is a minor concern -- the behavior is correct but non-obvious. An explicit `"=0.0.12"` would communicate intent more clearly.

**Duplicate `unicode-width` versions in transitive tree** - `Cargo.lock`
**Confidence**: 80%
- Problem: The dependency tree includes both `unicode-width 0.1.14` (required by `miette`) and `unicode-width 0.2.2` (required by `textwrap`, which is itself required by `miette`). This is a minor binary size inflation and entirely within `miette`'s control -- not actionable by this project.
- Impact: Negligible binary size increase (~20KB). No functional issue.
- Fix: No action needed. This will resolve when `miette` upgrades to `textwrap` version using unified `unicode-width`.

## Pre-existing Issues (Not Blocking)

(none -- this is the initial implementation)

## Suggestions (Lower Confidence)

- **Consider `cargo-deny` for CI** (Confidence: 70%) -- Tools like `cargo-deny` or `cargo-audit` are not currently installed. Adding `cargo-deny` to CI would automate license checking, vulnerability scanning, and duplicate dependency detection as the project grows.

- **`miette` "fancy" feature pulls heavy transitive deps** (Confidence: 65%) -- The `miette` crate with `features = ["fancy"]` brings in `backtrace`, `textwrap`, `terminal_size`, `supports-color`, etc. (11 transitive deps). If the fancy error rendering is only needed for the CLI binary and not the library, consider making it a feature-gated dependency so library consumers don't pull in terminal-related crates.

- **`getrandom` WASI transitive dependencies** (Confidence: 60%) -- The `tempfile` dev-dependency pulls in `getrandom` which brings `wasip2` and `wasip3` with `wit-bindgen` and an entire WASM toolchain (20+ packages). These are only compile-time platform stubs and only in dev-dependencies, so no production impact, but they inflate the lock file significantly (roughly 30 of the 90 packages).

## Dependency Inventory

| Dependency | Version | Purpose | License | Status |
|------------|---------|---------|---------|--------|
| `clap` | 4.6.1 | CLI argument parsing | MIT/Apache-2.0 | Stable, actively maintained |
| `indexmap` | 2.14.0 | Ordered set for cycle detection | Apache-2.0/MIT | Stable, actively maintained |
| `serde` | 1.228 | Serialization framework | MIT/Apache-2.0 | Stable, actively maintained |
| `serde_json` | 1.149 | JSON parsing (config, vars) | MIT/Apache-2.0 | Stable, actively maintained |
| `serde_yml` | 0.0.12 | YAML frontmatter parsing | MIT/Apache-2.0 | **Pre-release (0.0.x)** |
| `miette` | 7.6.0 | Diagnostic error reporting | Apache-2.0 | Stable, actively maintained |
| `thiserror` | 2.0.18 | Error derive macro | MIT/Apache-2.0 | Stable, actively maintained |
| `tempfile` | 3.27.0 | Temp dirs in tests (dev-only) | MIT/Apache-2.0 | Stable, actively maintained |

## Dependency Health Assessment

| Metric | Value | Assessment |
|--------|-------|------------|
| Direct dependencies | 7 runtime + 1 dev | Lean -- appropriate for scope |
| Total packages (lock) | 90 | Reasonable |
| Max tree depth | 9 levels | Acceptable (mostly proc-macro chains) |
| Duplicate crates | 1 (`unicode-width`) | Minor, upstream-controlled |
| Pre-release deps | 1 (`serde_yml 0.0.12`) | Risk -- needs monitoring |
| Unmaintained deps | 0 | Clean |
| License issues | 0 (all permissive) | Clean |
| Known CVEs | Unable to verify (`cargo-audit` not installed) | Unknown |
| Lockfile committed | Yes | Correct |
| Dev/runtime separation | Yes (`tempfile` is dev-only) | Correct |

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 2 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Dependencies Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions:
1. Add a `license` field to `Cargo.toml` and create a LICENSE file before public release.
2. Monitor `serde_yml` for stability (0.1.x milestone) or pin to exact version.
3. Add `rust-version` (MSRV) to `Cargo.toml` before public release.
4. Install `cargo-audit` or `cargo-deny` in CI pipeline to catch future vulnerabilities.

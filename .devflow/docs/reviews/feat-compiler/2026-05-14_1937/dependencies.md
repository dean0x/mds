# Dependencies Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-14

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**`serde_yml` is a `0.0.x` pre-release with pinned exact version** - `Cargo.toml:11`
**Confidence**: 85%
- Problem: `serde_yml = "0.0.12"` resolves to `^0.0.12` in Cargo, which for `0.0.x` versions effectively pins to exactly `0.0.12` (no patch updates). The `0.0.x` version scheme signals the library has not yet committed to a stable API. While the migration away from the deprecated `serde_yaml` (`0.9.34+deprecated`) is correct and necessary, the replacement is itself pre-release software. The `libyml` transitive dependency (the C YAML parser binding) is also at `0.0.5`, compounding the pre-release risk.
- Fix: This is acceptable for v0.1.0 of the `mds` crate since it is itself pre-release. However, consider tracking the `serde_yml` maturity roadmap. If `serde_yml` does not reach `0.1.x` within a reasonable timeframe, evaluate alternative YAML parsers (e.g., `yaml-rust2` for direct parsing without serde, or other maintained forks). Add a comment in `Cargo.toml` noting the pre-release status:
  ```toml
  # Pre-release; pinned to exact patch. Track for 0.1.x stability milestone.
  serde_yml = "0.0.12"
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`indexmap` version range is wide: `"2"`** - `Cargo.toml:9`
**Confidence**: 82%
- Problem: The version specifier `"2"` is equivalent to `^2.0.0`, accepting any `2.x.y`. While `indexmap` is a mature, well-maintained crate (part of the broader Rust ecosystem tooling), this is the widest possible range within the major version. The lockfile pins `2.14.0` which is fine, but the range allows any future 2.x.y that `cargo update` might pull in.
- Fix: Tighten to match the minimum version actually needed. Since `IndexSet` with `shift_remove` and ordered iteration has been stable since early `indexmap 2.x`, a tighter range is optional but recommended for defense-in-depth:
  ```toml
  indexmap = "2.2"
  ```
  This still allows patch updates but constrains to the minor version range you have tested against.

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`clap`, `serde_json`, and `thiserror` use major-version-only ranges** - `Cargo.toml:8,12,14`
**Confidence**: 80%
- Problem: `clap = { version = "4", ... }`, `serde_json = "1"`, and `thiserror = "2"` all use the widest possible range within their major version. While these are well-maintained crates and the lockfile protects against unexpected updates in builds, `cargo update` could pull in any compatible version. For a compiler tool, tighter ranges reduce risk of behavioral changes from minor version updates.
- Fix: Consider tightening to minor version ranges in a future PR:
  ```toml
  clap = { version = "4.6", features = ["derive"] }
  serde_json = "1.0"
  thiserror = "2.0"
  ```

### LOW

**Duplicate `unicode-width` versions (0.1.14 + 0.2.2)** - `Cargo.lock`
**Confidence**: 80%
- Problem: `miette` 7.6.0 depends on `unicode-width` 0.1.14, while its transitive dependency `textwrap` uses `unicode-width` 0.2.2. This results in two versions of the same crate being compiled. This is a pre-existing issue from the `miette` dependency, not introduced by this PR.
- Fix: No action needed now. A future `miette` update may unify these.

## Suggestions (Lower Confidence)

- **Consider `cargo-deny` or `cargo-audit` for CI** - (Confidence: 70%) -- Neither tool is currently configured in the project. For a compiler that processes user-authored template files, having automated supply chain auditing catches CVEs in transitive dependencies like `libyml`, `anyhow`, and `memchr` before they reach production.

- **`serde` with `derive` feature is correctly promoted to direct dependency** - `Cargo.toml:10` (Confidence: 75%) -- Previously, `serde` derive macros were available only transitively through `serde_yaml`. The explicit `serde = { version = "1", features = ["derive"] }` is the correct pattern. However, the `derive` feature is only used in `src/main.rs` for two structs (`MdsConfig`, `BuildConfig`). If the config structures grow, this remains justified. If they are ever removed, the explicit `serde` dependency can be dropped back to implicit.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 1 | 1 |

**Dependency Changes Summary**:
| Change | From | To | Justification |
|--------|------|----|---------------|
| `serde_yaml` removed | `0.9` (deprecated) | -- | Package is deprecated upstream |
| `serde_yml` added | -- | `0.0.12` | Active maintained replacement for serde_yaml |
| `indexmap` added | -- | `2` | Replaces HashSet+Vec pair with IndexSet for cycle detection |
| `serde` added (explicit) | (transitive) | `1` with `derive` | Needed for `#[derive(Deserialize)]` on config structs |
| `unsafe-libyaml` removed | (transitive) | -- | Replaced by `libyml` (transitive via serde_yml) |

**New transitive dependencies**: `libyml` 0.0.5 (MIT), `anyhow` 1.0.102 (MIT/Apache-2.0), `version_check` 0.9.5 (MIT/Apache-2.0). All licenses are permissive (MIT/Apache-2.0), compatible with the project.

**Dependencies Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

Conditions: The HIGH severity finding about `serde_yml` pre-release status should be acknowledged. The migration from deprecated `serde_yaml` is correct, but the replacement's `0.0.x` version warrants tracking. Consider adding a comment in `Cargo.toml` noting the pre-release pin and tightening the `indexmap` version range.

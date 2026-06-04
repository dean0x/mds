# Code Review Summary

**Branch**: feature/15-wasm-bindings -> main  
**Date**: 2026-05-19_1341  
**Reviewers**: 9 (security, architecture, performance, complexity, consistency, regression, testing, reliability, rust)

---

## Merge Recommendation: CHANGES_REQUESTED

**Rationale**: The PR introduces strong WASM bindings with solid architecture and security practices. However, four blocking categories of issues require fixes before merge:

1. **Input size limits missing at WASM boundary** (MEDIUM) — Compile/check accept unbounded source
2. **Error boilerplate and complexity** (HIGH x2) — 11 instances of repeated error construction inflating `parse_options` to 126 lines
3. **`.gitignore` regression** (HIGH) — Removes `.memory/` and `.docs/` entries, risking accidental commits
4. **Consistency gaps** (HIGH x3) — Missing `Cargo.toml` workspace fields, broken error contract in `to_js`, missing `#[must_use]` attribute

No CRITICAL issues identified. Once these are fixed, the PR is approvable.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| **Blocking** (in your changes) | 0 | 5 | 5 | 2 | **12** |
| **Should Fix** (code you touched) | 0 | 0 | 2 | 0 | **2** |
| **Pre-existing** (not blocking) | 0 | 0 | 0 | 0 | **0** |
| **TOTAL ACTIONABLE** | 0 | 5 | 7 | 2 | **14** |

---

## Blocking Issues (Must Fix Before Merge)

### Category: Input Validation (1 HIGH, 1 MEDIUM)

**[HIGH] Repeated JS error construction boilerplate (11 occurrences)**
- **Confidence**: 92% (Architecture) + 90% (Consistency) + cross-validated in Complexity, Rust
- **Files**: `crates/mds-wasm/src/lib.rs:45,49,55,60,66,72,79,106,143,152,166,181,200,217,242,280,312`
- **Problem**: The 4-line pattern of `js_sys::Error::new` + `Reflect::set` for code property + conversion appears 11 times across error construction sites (`parse_options`, `mds_error_to_js`, `catch_panic`, `build_modules`, `to_js`). This is the primary driver of `parse_options` length (126 lines, 95% confidence from Complexity reviewer) and violates DRY.
- **Impact**: 
  - Makes functions harder to maintain and edit
  - Increases chance of inconsistency if error shape changes
  - Every new option validation site requires copying 5 lines of boilerplate
- **Fix**: Extract a single helper function:
  ```rust
  /// Create a JS Error with a `code` property.
  fn js_error(message: &str, code: &str) -> JsValue {
      let err = js_sys::Error::new(message);
      let _ = Reflect::set(&err, &JsValue::from_str("code"), &JsValue::from_str(code));
      err.into()
  }
  
  /// Shorthand for options validation errors.
  fn options_error(message: &str) -> JsValue {
      js_error(message, "mds::invalid_options")
  }
  ```
  Then replace each error site with: `return Err(options_error(&msg));` (1 line instead of 5).
  Collapses ~60 lines of boilerplate across the file into ~10 lines total.

---

**[HIGH] `parse_options` function length and cyclomatic complexity (126 lines)**
- **Confidence**: 95% (Complexity) + 90% (Architecture)
- **File**: `crates/mds-wasm/src/lib.rs:130-256`
- **Problem**: The function exceeds the 50-line CRITICAL threshold with cyclomatic complexity ~12. Contains 3 sequential match blocks (filename, modules, vars) each with 3 arms and nested error construction. Adding a new option field requires copying the same match/error pattern, increasing maintenance burden.
- **Impact**: Costly to modify, hard to test individual field validation
- **Fix**: Extract three focused parser functions (`parse_filename`, `parse_modules`, `parse_vars`), each returning `Result<T, JsValue>`. Reduce `parse_options` to a ~20-line orchestrator:
  ```rust
  fn parse_options(options: JsValue) -> Result<ParsedOptions, JsValue> {
      if options.is_null() || options.is_undefined() {
          return Ok(ParsedOptions::default());
      }
      let opts_json: serde_json::Value = serde_wasm_bindgen::from_value(options)
          .map_err(|e| options_error(&format!("invalid options: {e}")))?;
      let serde_json::Value::Object(map) = &opts_json else {
          return Err(options_error("options must be a plain object"));
      };
      Ok(ParsedOptions {
          filename: parse_filename(map)?,
          extra_modules: parse_modules(map)?,
          vars: parse_vars(map)?,
      })
  }
  ```
  This also makes each field parser independently testable.

---

**[MEDIUM] No input size limits at WASM boundary**
- **Confidence**: 82% (Security)
- **File**: `crates/mds-wasm/src/lib.rs:345,380`
- **Problem**: `compile()` and `check()` accept `source: &str` without enforcing size limits. While `mds-core`'s `load_vars_file` enforces 10 MB `MAX_FILE_SIZE` for file inputs, the WASM boundary bypasses the file layer. A caller could pass an extremely large source string or modules map, causing excessive memory allocation within WASM linear memory and potential tab crash or DoS.
- **Impact**: Resource exhaustion vulnerability at WASM boundary
- **Fix**: Add size check before processing:
  ```rust
  const MAX_SOURCE_SIZE: usize = 10 * 1024 * 1024; // 10 MB, matching MAX_FILE_SIZE
  
  if source.len() > MAX_SOURCE_SIZE {
      return Err(js_error(
          "source exceeds maximum size of 10 MB",
          "mds::resource_limit"
      ));
  }
  ```
  Also consider limiting total size of modules in `parse_options` or `build_modules`.

---

### Category: Gitignore Regression (1 HIGH)

**[HIGH] `.gitignore` removes `.memory/` and `.docs/` entries**
- **Confidence**: 92% (Regression) + 85% (Security) + 85% (Consistency) — triple-validated
- **File**: `.gitignore:1-2`
- **Problem**: The PR removes two entries (`.memory/` and `.docs/`) that were on `main`. These are local-only devflow artifacts. Without them, `.memory/` (which exists in the current working tree with `decisions/` and `knowledge/` subdirs) will appear as untracked and could be accidentally committed via `git add .` or `git add -A`, leaking local devflow state into the repository. The commit message makes no mention of this, indicating it was likely unintentional.
- **Impact**: Risk of unintended commits of project-specific notes and internal decision records
- **Fix**: Restore the removed entries while keeping the new `crates/mds-wasm/pkg/` entry:
  ```
  /target
  crates/mds-wasm/pkg/
  .memory/
  .docs/
  .devflow/
  ```

---

### Category: Workspace Configuration (1 MEDIUM, 1 HIGH)

**[HIGH] Cargo.toml missing workspace fields in mds-wasm crate**
- **Confidence**: 92% (Consistency)
- **File**: `crates/mds-wasm/Cargo.toml`
- **Problem**: Both `mds-core/Cargo.toml` and `mds-cli/Cargo.toml` include `rust-version.workspace = true`, `readme.workspace = true`, and `keywords.workspace = true`. The new `mds-wasm/Cargo.toml` omits all three, deviating from the established workspace convention.
- **Impact**: Inconsistent crate metadata, maintenance burden if workspace fields change
- **Fix**: Add the three missing workspace assignments:
  ```toml
  [package]
  name = "mds-wasm"
  version.workspace = true
  edition.workspace = true
  rust-version.workspace = true
  description = "MDS compiler WebAssembly bindings"
  license.workspace = true
  readme.workspace = true
  repository.workspace = true
  keywords.workspace = true
  ```

---

**[MEDIUM] Workspace-wide `panic = "unwind"` affects all crates**
- **Confidence**: 85% (Regression, Performance, Rust) — triple-validated
- **File**: `Cargo.toml:29-34`
- **Problem**: The `[profile.dev]` and `[profile.release]` sections set `panic = "unwind"` at workspace level. While required for `catch_unwind` in mds-wasm, it now applies to mds-cli and mds-core as well. This prevents use of `panic = "abort"` for CLI (a common release optimization for smaller binaries and faster panics). Note: Cargo does not currently support per-package `panic` overrides.
- **Impact**: Slightly larger CLI binary, reduced compiler optimizations for CLI
- **Fix**: Document the rationale with a comment and/or remove workspace `panic` setting if it matches defaults:
  ```toml
  # Note: panic = "unwind" is required for catch_unwind in mds-wasm
  # This is already the default for dev. For release, using "unwind"
  # workspace-wide is necessary because mds-cli depends on mds-core,
  # which is called from mds-wasm's catch_unwind boundary.
  [profile.release]
  lto = true
  # panic = "unwind" inherited from default
  ```
  Alternatively, document that future per-crate optimization must use `--config` flags at build time.

---

### Category: Error Contract (2 HIGH)

**[HIGH] `to_js` serialization error missing `code` property**
- **Confidence**: 85% (Consistency)
- **File**: `crates/mds-wasm/src/lib.rs:308-315`
- **Problem**: Every other error construction site in mds-wasm sets a `code` property on JS errors. The `to_js` function (line 312) creates an error with only a `message`, breaking the contract documented in `compile()` and `check()` doc comments that state errors always have a `code` property.
- **Impact**: Inconsistent error interface, broken contract for JS consumers
- **Fix**: Add code property using the extracted helper:
  ```rust
  fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
      value
          .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
          .map_err(|e| {
              js_error(
                  &format!("failed to serialize result: {e}"),
                  "mds::internal"
              )
          })
  }
  ```

---

**[HIGH] Error code naming inconsistency and missing documentation**
- **Confidence**: 90% (Consistency)
- **File**: `crates/mds-wasm/src/lib.rs:146, 286`
- **Problem**: mds-core error codes use snake_case, but the new WASM boundary introduces codes (`mds::invalid_options`, `mds::internal`) that don't correspond to any `MdsError` variant. This creates a two-tier error code system that isn't documented. While acceptable for WASM-only boundary errors, the distinction isn't explicit.
- **Impact**: Confusing error interface for JS consumers, no clear documentation of boundary-only codes
- **Fix**: Add a comment at the top of `lib.rs` documenting WASM-only error codes:
  ```rust
  // WASM-only error codes (not in MdsError):
  // - mds::internal         — unrecoverable panic caught at WASM boundary
  // - mds::invalid_options  — malformed JS options object
  // - mds::resource_limit   — input size exceeds maximum
  ```

---

### Category: Nesting and Code Clarity (1 MEDIUM)

**[MEDIUM] `mds_error_to_js` has 4 levels of nesting in span block**
- **Confidence**: 82% (Complexity)
- **File**: `crates/mds-wasm/src/lib.rs:53-80`
- **Problem**: The span serialization block contains 4 levels of nesting (function -> if-let -> Reflect::set calls x5 -> nested if-let for optional line/column). The function is 44 lines (in the warning zone of 30-50 lines).
- **Impact**: Harder to understand and modify span construction logic
- **Fix**: Extract span serialization to dedicated helper:
  ```rust
  fn span_to_js(span: &mds::SerializedSpan) -> js_sys::Object {
      let obj = js_sys::Object::new();
      let _ = Reflect::set(&obj, &JsValue::from_str("offset"), &JsValue::from_f64(span.offset as f64));
      let _ = Reflect::set(&obj, &JsValue::from_str("length"), &JsValue::from_f64(span.length as f64));
      if let Some(line) = span.line {
          let _ = Reflect::set(&obj, &JsValue::from_str("line"), &JsValue::from_f64(line as f64));
      }
      if let Some(column) = span.column {
          let _ = Reflect::set(&obj, &JsValue::from_str("column"), &JsValue::from_f64(column as f64));
      }
      obj
  }
  ```
  Reduces `mds_error_to_js` to ~20 lines and max nesting of 2.

---

### Category: Performance (2 MEDIUM, 2 LOW)

**[MEDIUM] Double serialization in options parsing**
- **Confidence**: 85% (Performance)
- **File**: `crates/mds-wasm/src/lib.rs:141`
- **Problem**: `parse_options` deserializes entire JS object into `serde_json::Value` (line 141), then clones fields again when extracting (`.clone()` on lines 163, 197, 235). For large `modules` maps (many virtual files with large source content), this doubles memory usage and allocation cost.
- **Impact**: Inefficient memory usage for large options objects
- **Fix**: Use ownership destructuring to move values instead of cloning:
  ```rust
  let serde_json::Value::Object(mut map) = opts_json else { ... };
  let filename = match map.remove("filename") {
      Some(serde_json::Value::String(s)) => s,
      None => "input.mds".to_string(),
      ...
  };
  ```
  Using `map.remove()` avoids cloning strings.

---

**[MEDIUM] `val.clone()` in vars iteration forces redundant deep-copy**
- **Confidence**: 82% (Performance)
- **File**: `crates/mds-wasm/src/lib.rs:235`
- **Problem**: Each var value is cloned before passing to `Value::from_json()`. Since `from_json` consumes by value, the clone is necessary only because `vars_map` is borrowed. If destructured by ownership (as above), the clone is eliminated.
- **Impact**: Full deep copy of JSON tree for large nested variable objects
- **Fix**: Addressed by the same ownership destructuring fix above.

---

**[LOW] `wasm-opt = false` disables Binaryen optimization**
- **Confidence**: 85% (Performance)
- **File**: `crates/mds-wasm/Cargo.toml:24`
- **Problem**: Release metadata sets `wasm-opt = false`, disabling Binaryen optimizer. Binaryen typically reduces binary size by 10-20%. Current binary is ~455 KB; with wasm-opt it would be ~370-410 KB.
- **Impact**: Slightly larger download, but still within 500 KB budget
- **Fix**: Enable wasm-opt for release builds:
  ```toml
  [package.metadata.wasm-pack.profile.release]
  wasm-opt = ["-Oz"]
  ```
  If disabled for a reason, document it.

---

**[LOW] `panic = "unwind"` set globally affects CLI binary size**
- **Confidence**: 80% (Performance)
- **Problem**: `panic = "unwind"` at workspace level applies to mds-cli (which doesn't use `catch_unwind`). Abort strategy produces 5-10% smaller binaries.
- **Impact**: Slightly larger CLI binary (already noted in MEDIUM issue above)
- **Fix**: Same as MEDIUM workspace panic issue — document rationale and note Cargo limitation.

---

## Should-Fix Issues (Code You Touched, Not Blocking)

### Category: Boundary Validation (1 MEDIUM)

**[MEDIUM] `load_vars_str` has no input size limit, unlike sibling `load_vars_file`**
- **Confidence**: 85% (Reliability) + 82% (Consistency) — duplicated findings
- **File**: `crates/mds-core/src/lib.rs:759`
- **Context**: This is in mds-core, which you touched to add the new function
- **Problem**: `load_vars_file` (line 711) enforces `MAX_FILE_SIZE` (10 MB). New `load_vars_str` (line 759) parses JSON directly with no size check. A caller passing arbitrarily large string causes unbounded allocation.
- **Impact**: Reliability gap — API inconsistency between siblings
- **Fix**: Add size guard consistent with file-based sibling:
  ```rust
  pub fn load_vars_str(json: &str) -> Result<HashMap<String, Value>, MdsError> {
      if json.len() as u64 > MAX_FILE_SIZE {
          return Err(MdsError::resource_limit(format!(
              "vars JSON string exceeds maximum size of {MAX_FILE_SIZE} bytes"
          )));
      }
      let parsed: serde_json::Value =
          serde_json::from_str(json).map_err(|e| MdsError::json_error(e.to_string()))?;
      // ...
  }
  ```

---

### Category: Rust Practices (1 MEDIUM)

**[MEDIUM] Silently discarded `Reflect::set` return values (16 occurrences)**
- **Confidence**: 82% (Reliability, Rust) — duplicated
- **Files**: `crates/mds-wasm/src/lib.rs:45,49,55,60,66,72,79,106,143,170,204,221,246,283` (and others)
- **Problem**: All `Reflect::set` calls discard `Result` with `let _ =`. While unlikely to fail on freshly-created objects, silent discarding means a failure would produce incomplete error objects (missing `code`, `help`, `span`) with no diagnostic trace.
- **Impact**: Degraded error messages on edge case JS engine behavior
- **Fix**: For defense-in-depth, extract a helper that debug-asserts success:
  ```rust
  /// Set a property on a JS object. Debug-asserts success in dev builds.
  fn set_prop(target: &JsValue, key: &str, value: &JsValue) {
      let result = Reflect::set(target, &JsValue::from_str(key), value);
      debug_assert!(result.is_ok(), "Reflect::set failed for key: {key}");
  }
  ```
  Or add explanatory comment at the top of the file documenting the rationale.

---

### Category: Missing Attributes (1 MEDIUM)

**[MEDIUM] `Value::from_json` promoted to `pub` without `#[must_use]`**
- **Confidence**: 82% (Consistency)
- **File**: `crates/mds-core/src/value.rs:101`
- **Problem**: All other public methods on `Value` that return meaningful values carry `#[must_use]`: `is_truthy()`, `as_array()`, `type_name()`. The newly-public `from_json` returns `Result<Value, MdsError>` but lacks the attribute.
- **Impact**: Inconsistent API, potential for accidental result discarding
- **Fix**: Add attribute:
  ```rust
  /// Convert a serde_json::Value into our Value enum.
  #[must_use = "the converted value should be used"]
  pub fn from_json(json: serde_json::Value) -> Result<Value, MdsError> {
  ```

---

### Category: Security (1 MEDIUM)

**[MEDIUM] Panic message may leak internal details**
- **Confidence**: 80% (Security)
- **File**: `crates/mds-wasm/src/lib.rs:96-113`
- **Problem**: The `catch_panic` function includes panic payload verbatim in JS error message. Panic messages from compiler internals or dependencies can leak file paths, assertion details, or internal state descriptions.
- **Impact**: Information disclosure (low risk for template compiler, but principle of minimal disclosure)
- **Fix**: Sanitize or truncate, or provide generic message with debug-only detail:
  ```rust
  let js_err = js_sys::Error::new("internal compiler error");
  // Optionally set a detail property for debugging:
  let _ = Reflect::set(&js_err, &JsValue::from_str("detail"), &JsValue::from_str(&msg));
  ```

---

## Pre-existing Issues

No pre-existing CRITICAL or HIGH issues identified. No blocking issues outside the scope of this PR.

---

## Positive Findings (What Was Done Well)

### Security
1. **VirtualFs isolation** — Compilation runs entirely in-memory via VirtualFs. WASM boundary never accesses OS filesystem. Path traversal properly guarded (null byte rejection, segment counting, `..` traversal blocked).
2. **Panic containment** — `catch_unwind` prevents panics from aborting the WASM module; converts to structured JS errors.
3. **Input validation** — All options fields type-checked with clear error messages. Filename collisions detected. Null/undefined options gracefully default.
4. **Depth limits** — `Value::from_json` enforces 64-level nesting depth limit, preventing stack overflow.
5. **No unsafe code** — Entire mds-wasm crate uses safe Rust only.
6. **No hardcoded secrets** — No credentials, tokens, or API keys in changed files.
7. **Structured error handling** — Errors converted to `js_sys::Error` with typed `code` properties rather than raw Rust types.

### Architecture
1. **Clean layering** — mds-wasm depends on mds-core and never reaches into internal modules. Dependency direction strictly inward.
2. **Boundary isolation** — WASM boundary is a proper adapter layer. All JS<->Rust conversions in mds-wasm/src/lib.rs.
3. **Additive core changes** — Two mds-core changes minimal, backward-compatible, follow existing patterns exactly.
4. **DIP compliance** — mds-wasm depends on mds-core public API abstractions, not internals.
5. **Error contract** — Structured JS errors with `code`, `help`, `span` provide well-defined interface.

### Testing
1. **Comprehensive coverage** — 21 wasm-bindgen-test cases + 13 mds-core tests for new functions.
2. **Good test structure** — Clear Arrange-Act-Assert, descriptive names, helper functions reduce boilerplate.
3. **Error case coverage** — Tests include input validation, error codes, boundary conditions.

### Code Quality
1. **No `.unwrap()` in library code** — All error paths use `?` or explicit `map_err`.
2. **Proper `#[must_use]` annotations** — Consistent with existing convention (where needed).
3. **Well-documented code** — Clear doc comments on public functions.

---

## Summary by Reviewer Score

| Reviewer | Score | Recommendation | Key Finding |
|----------|-------|-----------------|-------------|
| Security | 8/10 | CHANGES_REQUESTED | Missing input size limits, .gitignore regression |
| Architecture | 8/10 | APPROVED_WITH_CONDITIONS | Error boilerplate and parse_options SRP |
| Performance | 8/10 | APPROVED_WITH_CONDITIONS | Double serialization, wasm-opt disabled |
| Complexity | 6/10 | CHANGES_REQUESTED | parse_options 126 lines, 11x error boilerplate |
| Consistency | 7/10 | CHANGES_REQUESTED | Missing Cargo.toml fields, to_js missing code, .gitignore |
| Regression | 8/10 | CHANGES_REQUESTED | .gitignore removal, workspace panic scope |
| Testing | 7/10 | CHANGES_REQUESTED | Missing span/help assertions, check() under-tested |
| Reliability | 8/10 | APPROVED_WITH_CONDITIONS | load_vars_str size limit, Reflect::set silent failures |
| Rust | 8/10 | APPROVED_WITH_CONDITIONS | Silenced Reflect::set, workspace panic scope |

**Aggregate**: 7.6/10 average. All reviewers except one (Testing: 7/10, Complexity: 6/10) gave 8+/10. Consensus: **approvable with requested changes**.

---

## Action Plan (Priority Order)

### Phase 1: Critical Fixes (Do First)
1. **Extract `js_error` and `options_error` helpers** — Eliminates 11 instances of boilerplate, reduces `parse_options` length, enables Phase 2
2. **Restore `.gitignore` entries** — Prevent accidental commits of .memory/, .docs/
3. **Add `Cargo.toml` workspace fields** — mds-wasm consistency with mds-core, mds-cli

### Phase 2: Code Structure (Depends on Phase 1)
4. **Split `parse_options` into per-field parsers** — Reduces complexity, improves testability
5. **Extract `span_to_js` helper** — Flatten `mds_error_to_js` nesting
6. **Fix `to_js` error code property** — Restore error contract

### Phase 3: Validation & Polish
7. **Add source size limit at WASM boundary** — Match mds-core's 10 MB limit
8. **Fix options destructuring (ownership)** — Eliminate double serialization and clones
9. **Add `#[must_use]` to `Value::from_json`** — Consistency
10. **Add size limit to `load_vars_str`** — Parity with `load_vars_file`

### Phase 4: Testing (Depends on Phase 1-3)
11. **Add span/help assertions to error tests** — Verify structured error properties
12. **Add check() tests for modules, vars, options** — Parity with compile() coverage

### Phase 5: Optimization & Documentation
13. **Enable wasm-opt** — ~10-20% binary size reduction
14. **Document WASM-only error codes** — Add comment explaining boundary-specific codes
15. **Add `Reflect::set` rationale comment or helper** — Explain intentional silencing

---

## Detailed Blocking Issues by Category

### Blocked by Boilerplate
- HIGH: Repeated JS error construction (11x)
- HIGH: parse_options length (126 lines)
- MEDIUM: mds_error_to_js nesting (4 levels)

**Resolution**: Extract 3 helpers (`js_error`, `options_error`, `span_to_js`). Unblocks all three issues.

### Blocked by .gitignore
- HIGH: `.gitignore` entries removed

**Resolution**: Restore `.memory/`, `.docs/` entries.

### Blocked by Consistency
- HIGH: Cargo.toml missing workspace fields
- HIGH: to_js missing code property
- HIGH: Error code naming not documented

**Resolution**: Add 3 Cargo.toml fields, fix `to_js` to use `js_error` helper, add comment.

### Blocked by Workspace Config
- MEDIUM: Workspace panic scope affects all crates

**Resolution**: Document rationale (Cargo limitation prevents per-package override), or remove if it matches defaults.

---

## Confidence Aggregation

Issues with **multiple reviewer agreement** are highest priority:

| Issue | Reviewers | Confidence |
|-------|-----------|------------|
| Error boilerplate (11x) | Architecture, Complexity, Consistency, Rust | 90%+ |
| .gitignore removal | Security, Consistency, Regression | 92%+ |
| parse_options length | Complexity, Architecture | 95%, 90% |
| load_vars_str size limit | Reliability, Regression | 85%, 65% |
| Workspace panic scope | Regression, Performance, Rust | 85%+ |

The top 3 blocking issues have 90%+ confidence across 3+ reviewers.

---

## Next Steps

1. **Assignee**: Review this summary and confirm understanding of blocking categories
2. **Implementation**: Work through Action Plan phases 1-5 in order
3. **Re-review**: Resubmit PR; expect quick approval once blocking issues are resolved
4. **Testing**: Run `cargo test --all` and `wasm-pack test` after each phase

The PR is solid; these are maintainability and completeness fixes, not architectural problems.

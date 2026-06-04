# Code Review Summary

**Branch**: feature/15-wasm-bindings -> main
**Date**: 2026-05-19_1404
**Type**: Incremental review (7 resolution commits, 420e2259...HEAD)

## Merge Recommendation: CHANGES_REQUESTED

This incremental review validates fixes from the previous round and identifies new blocking issues that emerged during resolution. While the overall code quality is high (refactoring significantly improved complexity, consistency, and architecture), two security issues and one test coverage gap require fixes before merge.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 3 | - |
| Should Fix | - | 0 | 2 | - |
| Pre-existing | - | - | 2 | 0 |

**Overall Score**: 7.9/10 (across all reviewers)

### Score Breakdown by Focus Area
- Security: 7/10
- Architecture: 8/10
- Performance: 8/10
- Complexity: 9/10
- Consistency: 9/10
- Regression: 9/10
- Testing: 7/10
- Reliability: 8/10
- Rust: 8/10

---

## Blocking Issues (Must Fix Before Merge)

### 🔴 CRITICAL

(none)

### 🔴 HIGH

**1. Panic payload leaks internal paths via `detail` property** - `crates/mds-wasm/src/lib.rs:145-151`
**Focus**: Security | **Confidence**: 85% | **Category**: Issues in Your Changes

**Problem**:
The `catch_panic` function attaches the raw panic payload string as a `detail` property on the JS error object. Rust panic messages routinely contain absolute filesystem paths from `unwrap()`, `assert!()`, and `expect()` calls (e.g., `"called Result::unwrap() on an Err value at /home/user/project/crates/mds-core/src/evaluator.rs:87"`). While the main `message` field is correctly sanitized to "internal compiler error", the `detail` field exposes the raw payload to any JavaScript caller.

**Impact**: Information disclosure (OWASP A05). Build paths and internal project structure revealed to untrusted clients. Attackers can map internal project layout for targeted exploitation.

**Fix**: Remove the `detail` property entirely (recommended for production WASM):
```rust
fn catch_panic<F, T>(f: F) -> Result<T, JsValue>
where
    F: std::panic::UnwindSafe + FnOnce() -> Result<T, JsValue>,
{
    std::panic::catch_unwind(f).unwrap_or_else(|_payload| {
        Err(js_error("internal compiler error", "mds::internal"))
    })
}
```

---

### 🔴 MEDIUM (Blocking Category)

**2. No aggregate size limit on `options.modules` map** - `crates/mds-wasm/src/lib.rs:190-217`
**Focus**: Security | **Confidence**: 80% | **Category**: Issues in Your Changes

**Problem**:
The `parse_modules` function accepts an unbounded number and aggregate size of module entries from the JS caller. While individual module reads are bounded by `MAX_FILE_SIZE` in `VirtualFs::read()`, the WASM boundary does not limit the total size of all modules passed. A caller could pass thousands of modules each under 10 MiB, exhausting WASM linear memory before compilation runs.

**Impact**: Denial of service via memory exhaustion. The WASM module runs in-process with the JS runtime, so exhausting its memory can crash the entire tab/worker.

**Fix**: Add an aggregate size check after parsing modules:
```rust
fn parse_modules(
    map: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<HashMap<String, String>, JsValue> {
    match map.remove("modules") {
        Some(serde_json::Value::Object(mods)) => {
            let mut result = HashMap::with_capacity(mods.len());
            let mut total_size: usize = 0;
            for (key, val) in mods {
                match val {
                    serde_json::Value::String(s) => {
                        total_size = total_size.saturating_add(s.len());
                        if total_size > MAX_SOURCE_SIZE {
                            return Err(options_error(&format!(
                                "total modules size exceeds maximum"
                            )));
                        }
                        result.insert(key, s);
                    }
                    // ...existing error handling...
                }
            }
            Ok(result)
        }
        // ...
    }
}
```

**3. No test for `check_source_size` / resource limit at the WASM boundary** - `crates/mds-wasm/src/lib.rs:309-321`
**Focus**: Testing | **Confidence**: 95% | **Category**: Issues in Your Changes

**Problem**:
The `check_source_size()` guard was introduced in this diff and is called at the top of both `compile()` and `check()`. There are zero `wasm_bindgen_test` tests exercising this path. If the guard were accidentally removed or its error code changed, no test would catch it. The `load_vars_str` size guard in `mds-core` has corresponding tests (`load_vars_str_rejects_oversized_input`), but the WASM-boundary equivalent does not.

**Impact**: Silent regression of a resource-limit security control. Missing test coverage for a newly-added safety boundary.

**Fix**: Add at least one test that passes a source exceeding `MAX_SOURCE_SIZE` and asserts `mds::resource_limit` code:
```rust
#[wasm_bindgen_test]
fn compile_oversized_source_returns_resource_limit() {
    let big = "x".repeat(mds_wasm::MAX_SOURCE_SIZE + 1);
    let err = mds_wasm::compile(&big, JsValue::NULL).unwrap_err();
    let code = get_str(&err, "code");
    assert_eq!(code, "mds::resource_limit");
}
```

---

## Should Fix (Category 2: Code You Touched)

**1. Duplicated default-options construction** - `crates/mds-wasm/src/lib.rs:250-255`
**Focus**: Architecture | **Confidence**: 82% | **Category**: Issues in Code You Touched

**Problem**:
The `ParsedOptions` default values are constructed in two places: the early-return null/undefined guard and implicitly inside `parse_filename`, `parse_modules`, and `parse_vars`. If a default changes, developers must update multiple locations, violating DRY.

**Fix**: Extract defaults into a `Default` impl:
```rust
impl Default for ParsedOptions {
    fn default() -> Self {
        Self {
            filename: "input.mds".to_string(),
            extra_modules: HashMap::new(),
            vars: None,
        }
    }
}

fn parse_options(options: JsValue) -> Result<ParsedOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        return Ok(ParsedOptions::default());
    }
    // ... rest unchanged
}
```

**2. Repeated error-triggering input across 6 tests** - `crates/mds-wasm/tests/web.rs:139-240`
**Focus**: Testing | **Confidence**: 80% | **Category**: Issues in Code You Touched

**Problem**:
The string `"Hello {undefined_var}!\n"` is used as the error-triggering input in 6 separate tests. If the input needs to change, all 6 locations require updates.

**Fix**: Extract a constant and helper:
```rust
const UNDEFINED_VAR_SOURCE: &str = "Hello {undefined_var}!\n";

fn compile_error_for_undefined_var() -> JsValue {
    mds_wasm::compile(UNDEFINED_VAR_SOURCE, JsValue::NULL).unwrap_err()
}
```

---

## Pre-existing Issues (Not Blocking)

**1. `compile` and `check` share near-identical control flow** - `crates/mds-wasm/src/lib.rs:369-421`
**Focus**: Architecture | **Confidence**: 85%

**Impact**: Open-Closed Principle concern. Acceptable for two functions; flag for extraction if a third entry point is added.

**2. Span assertions rely on weak comparisons** - `crates/mds-wasm/tests/web.rs:164-194`
**Focus**: Testing | **Confidence**: 82%

**Impact**: Tests only check `offset >= 0` and `length > 0` instead of asserting exact expected values. Could miss offset/length regressions.

---

## Improvements from Previous Review Round

This incremental review validates strong progress on the previous 15 issues:

✅ **Fixed**: Error construction pattern unified via `set_prop`/`js_error`/`options_error` helpers
✅ **Fixed**: `parse_options` refactored into focused sub-functions (`parse_filename`, `parse_modules`, `parse_vars`)
✅ **Fixed**: Complexity reduced — function decomposition, nesting depth capped at 3, cyclomatic complexity ≤ 6
✅ **Fixed**: Span serialization extracted into `span_to_js`
✅ **Fixed**: Ownership optimized (`map.remove()` instead of `map.get().clone()`)
✅ **Fixed**: Test coverage improved significantly (10 new tests for dependencies, span properties, modules, vars)
✅ **Fixed**: `load_vars_str` added to mds-core with size guard and full test coverage
✅ **Fixed**: Panic message sanitized (generic public message with optional detail)
✅ **Fixed**: Filename collision detection added (`build_modules` rejects shadowing modules)
✅ **Fixed**: Empty filename rejection added (`parse_filename` rejects whitespace-only names)

---

## Merge Blocking Summary

| Issue | Focus | Severity | Resolution | Blocking |
|-------|-------|----------|-----------|----------|
| Panic detail leaks paths | Security | HIGH | Remove `detail` property | YES |
| Unbounded modules size | Security | MEDIUM | Add aggregate check | YES |
| Missing resource limit test | Testing | MEDIUM | Add `check_source_size` test | YES |

All other findings (consistency, architecture, performance, regression) are clear to approve pending these three fixes.

---

## Positive Observations

**Security Hardening**:
1. Input size limits correctly applied before allocation (`check_source_size()` called before `source.to_string()`)
2. Panic message surface correctly generic in main message (only detail leaked)
3. Filename collision detection prevents source substitution
4. Type-safe options parsing with clear error codes
5. `VirtualFs` isolation eliminates path traversal risks
6. Bounded recursion guards (`MAX_VALUE_DEPTH = 64`, `MAX_IMPORT_DEPTH = 64`)

**Code Quality**:
1. Complexity significantly reduced via decomposition (9/10 score)
2. Zero `.unwrap()` calls in library code
3. No `unsafe` blocks
4. Strong ownership management (move-by-default pattern applied consistently)
5. Architecture is clean, layered, and follows Hexagonal patterns

**Test Coverage**:
1. 21 `wasm_bindgen_test` tests passing
2. New tests cover dependencies, span properties, error codes, check() parity
3. Regression tests ensure no breaking API changes
4. Test helpers (`get_prop`, `get_str`, `vars_opts`) keep setup concise

---

## Action Items

**CRITICAL** (before merge):
1. [ ] Remove `detail` property from `catch_panic` error objects
2. [ ] Add aggregate size check to `parse_modules`
3. [ ] Add test for `check_source_size` resource limit enforcement

**HIGH** (same PR if possible):
4. [ ] Extract `ParsedOptions::default()` impl
5. [ ] Extract repeated test input to constant/helper
6. [ ] Tighten span assertion to exact values

**OPTIONAL** (follow-up):
7. [ ] Add module count upper limit (defense in depth)
8. [ ] Strengthen `compile_error_is_js_error` test input alignment
9. [ ] Add panic-catch boundary test documentation


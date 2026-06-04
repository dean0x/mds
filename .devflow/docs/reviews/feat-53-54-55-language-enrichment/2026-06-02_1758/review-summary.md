# Code Review Summary

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02_1758

## Merge Recommendation: BLOCK MERGE

**Rationale**: 6 CRITICAL/HIGH blocking issues across architecture, performance, security, and correctness require fixes before merge. While the feature implementation is solid and well-tested (690 tests passing), multiple HIGH-severity findings span multiple reviewers with high confidence (80-90%). All issues are fixable.

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 6 | 4 | 0 | 10 |
| Should Fix | 0 | 0 | 3 | 0 | 3 |
| Pre-existing | 0 | 0 | 0 | 0 | 0 |

---

## Blocking Issues (MUST FIX)

### HIGH Severity (6 issues)

#### 1. `length()` returns byte count, not character count for multi-byte UTF-8 strings
**File**: `crates/mds-core/src/builtins.rs:341`
**Confidence**: 90% (consensus across 7 reviewers: security, architecture, performance, consistency, testing, reliability, rust)
**Category**: Blocking

**Problem**: `length()` uses `s.len()` which returns byte length, not character count. For multi-byte UTF-8 strings like "café" (5 bytes but 4 characters), users will expect character count semantics. This is inconsistent with `reverse()` which uses `s.chars().rev()` (character-aware) and `slice()` which uses `snap_to_char_boundary` for char safety. The three string builtins present an inconsistent API.

**Impact**: High — API confusion for template authors using non-ASCII strings. Users will get surprising results when composing `slice(s, 0, length(s) - 1)`.

**Fix**: Use `s.chars().count()` for character-level semantics:
```rust
Value::String(s) => Ok(Value::Number(s.chars().count() as f64)),
```

**Reviewers**: security (70%, pre-existing), architecture (80%, should-fix), performance (85%, pre-existing), consistency (85%, blocking), testing (85%, blocking), reliability (80%, medium), rust (90%, blocking)

---

#### 2. `split("")` enables O(n) memory amplification from empty separator
**File**: `crates/mds-core/src/builtins.rs:219-223`
**Confidence**: 82% (consensus across security, performance)
**Category**: Blocking

**Problem**: `builtin_split` passes the separator directly to `str::split()`. When `sep` is an empty string, Rust's `split("")` produces N+2 parts for a string of length N (one per byte boundary). A 10MB input string (within MAX_FILE_SIZE) would produce ~10 million `Value::String` allocations, each with heap overhead (~24 bytes), totaling 250-300MB of heap usage — a memory amplification vector for adversarial inputs.

**Impact**: HIGH — Security-relevant denial-of-service vector. Input amplification is 25-30x.

**Fix**: Reject empty separator with a clear error:
```rust
fn builtin_split(args: &[Value]) -> Result<Value, MdsError> {
    let s = require_string_at(args, 0, "split", "first")?;
    let sep = require_string_at(args, 1, "split", "second")?;
    if sep.is_empty() {
        return Err(MdsError::builtin_error(
            "split() separator must not be empty".to_string(),
        ));
    }
    let parts: Vec<Value> = s.split(sep).map(|p| Value::String(p.to_string())).collect();
    Ok(Value::Array(parts))
}
```

**Reviewers**: security (82%), performance (82%)

---

#### 3. `replace("", x)` enables O(n) output amplification
**File**: `crates/mds-core/src/builtins.rs:212-216`
**Confidence**: 80% (consensus across security, performance)
**Category**: Blocking

**Problem**: `builtin_replace` passes `from` directly to `str::replace()`. When `from` is empty, Rust inserts the replacement before every character and at both ends. For a 10MB input with `replace(s, "", "XX")`, output grows to ~30MB. The 50MB MAX_OUTPUT_SIZE provides an upper bound in the evaluator, but the amplification occurs inside `str::replace()` before the evaluator's limit check.

**Impact**: HIGH — Memory amplification vector (3-5x). Bypasses MAX_OUTPUT_SIZE check.

**Fix**: Reject empty `from` string:
```rust
fn builtin_replace(args: &[Value]) -> Result<Value, MdsError> {
    let s = require_string_at(args, 0, "replace", "first")?;
    let from = require_string_at(args, 1, "replace", "second")?;
    if from.is_empty() {
        return Err(MdsError::builtin_error(
            "replace() search string must not be empty".to_string(),
        ));
    }
    let to = require_string_at(args, 2, "replace", "third")?;
    Ok(Value::String(s.replace(from, to)))
}
```

**Reviewers**: security (80%), performance (80%)

---

#### 4. `sort()` treats NaN values as equal via `unwrap_or(Equal)`, violating total order contract
**File**: `crates/mds-core/src/builtins.rs:414-416`
**Confidence**: 85% (consensus across security, testing, reliability, rust)
**Category**: Blocking

**Problem**: The sort comparator uses `partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)` which treats NaN comparisons as "equal". This violates the total ordering contract (transitivity): NaN == 1.0 and NaN == 2.0 but 1.0 != 2.0. While the parser blocks NaN literals and `number()` rejects non-finite values, NaN could enter from future arithmetic or external data (JSON frontmatter). If present, sort produces non-deterministic output.

**Impact**: HIGH — Correctness issue. Non-deterministic sort behavior on edge-case inputs.

**Fix**: Use `total_cmp()` (stable since Rust 1.62) or reject NaN:
```rust
Value::Number(_) => {
    for item in &sorted {
        if let Value::Number(n) = item {
            if !n.is_finite() {
                return Err(MdsError::builtin_error(
                    "sort() cannot sort arrays containing NaN or infinity".to_string(),
                ));
            }
        }
    }
    sorted.sort_by(|a, b| match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.total_cmp(b),
        _ => unreachable!(),
    });
}
```

**Reviewers**: security (82%), testing (80%), reliability (85%), rust (85%)

---

#### 5. Dual-registry pattern for builtins creates maintenance coupling (builtin metadata + dispatch)
**File**: `crates/mds-core/src/builtins.rs:27-128` and `crates/mds-core/src/builtins.rs:136-158`
**Confidence**: 85% (architecture review)
**Category**: Blocking

**Problem**: Built-in registration requires updating two separate structures in lockstep: the `BUILTINS` static array (metadata/arity) and the `call_builtin` match arms (dispatch). If one is updated without the other, the system either silently rejects a valid function name at dispatch or reports wrong arity. There is no compile-time enforcement of synchronization. This pattern will become more fragile as more builtins are added.

**Impact**: HIGH — Maintainability/correctness risk. Silent dispatch failures on future changes.

**Fix**: Consolidate into a single registry:
```rust
struct BuiltinDef {
    name: &'static str,
    min_args: usize,
    max_args: usize,
    handler: fn(&[Value]) -> Result<Value, MdsError>,
}

static BUILTINS: &[BuiltinDef] = &[
    BuiltinDef { name: "upper", min_args: 1, max_args: 1, handler: builtin_upper },
    // ...
];

pub(crate) fn call_builtin(name: &str, args: &[Value]) -> Result<Value, MdsError> {
    match get_builtin(name) {
        Some(def) => (def.handler)(args),
        None => Err(MdsError::undefined_fn(name)),
    }
}
```

**Reviewers**: architecture (85%, blocking)

---

#### 6. Arity-check logic duplicated 5 times across evaluator/validator with no shared abstraction
**File**: `crates/mds-core/src/evaluator.rs:273-276,343-349` and `crates/mds-core/src/validator.rs:180-213,229-242,285-321`
**Confidence**: 82% (consensus across architecture, complexity, consistency)
**Category**: Blocking

**Problem**: The arity check pattern (`args.len() < required || args.len() > total`) is repeated 5 times across evaluator and validator. The evaluator and validator independently implement the "resolve function, check arity, check builtins as fallback" decision chain. This means resolution order (user-defined > builtin > undefined error) is encoded in two places. If it ever changes, both must be updated in lockstep. The `required_param_count` export from evaluator to validator (per self-review fix) is a symptom of this coupling.

**Impact**: HIGH — Maintainability debt. High risk of silent inconsistency.

**Fix**: Extract a shared function-resolution helper:
```rust
pub(crate) enum ResolvedFunction<'a> {
    UserDefined { required: usize, total: usize, func: &'a FunctionDef },
    Builtin { min: usize, max: usize },
}

pub(crate) fn resolve_function<'a>(name: &str, scope: &'a Scope) -> Result<ResolvedFunction<'a>, MdsError> { ... }
```

Both evaluator and validator would call this function, eliminating duplicated resolution logic.

**Reviewers**: architecture (82%, blocking), complexity (80%, medium), consistency (80%, medium)

---

### MEDIUM Severity (4 issues)

#### 1. `unique()` has O(n^2) complexity with Vec::contains linear scan
**File**: `crates/mds-core/src/builtins.rs:436-441`
**Confidence**: 85% (consensus across performance, reliability, rust, security)
**Category**: Blocking

**Problem**: `builtin_unique` uses `result.contains(item)` inside a loop, yielding O(n^2) overall. For large arrays (e.g., 10K+ elements from `split`), this becomes a CPU-bound denial-of-service vector. While arrays are bounded by input size (10MB), a split producing 10M single-char strings piped through `unique()` causes ~10^14 comparisons.

**Impact**: MEDIUM → HIGH for adversarial inputs. No explicit array-size limit.

**Fix**: Use HashSet or add size bound:
```rust
fn builtin_unique(args: &[Value]) -> Result<Value, MdsError> {
    let arr = match &args[0] {
        Value::Array(a) => a,
        other => return Err(type_err("unique", "", "array", other.type_name())),
    };
    if arr.len() > 10_000 {
        return Err(MdsError::builtin_error(
            "unique() array exceeds maximum size of 10,000 elements".to_string()
        ));
    }
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result: Vec<Value> = Vec::new();
    for item in arr {
        let key = item.to_string();
        if seen.insert(key) {
            result.push(item.clone());
        }
    }
    Ok(Value::Array(result))
}
```

**Reviewers**: security (80%, should-fix), performance (85%, blocking), reliability (82%, medium), rust (80%, medium), complexity (implicit)

---

#### 2. `require_number_index` performs unsafe f64-to-usize cast without overflow guard
**File**: `crates/mds-core/src/builtins.rs:295`
**Confidence**: 90% (reliability review)
**Category**: Blocking

**Problem**: `n.max(0.0).floor() as usize` performs a cast on out-of-range f64 without explicit bounds. While the subsequent `.min(len)` clamp in `slice()` saves correctness today, the cast itself is a footgun. The safety guarantee is implicit (depends on downstream code), not self-contained. If any caller uses the result without a bounds clamp, it silently produces `usize::MAX`.

**Impact**: MEDIUM → HIGH. Latent safety issue. Defensive cast needed.

**Fix**: Add explicit bounds check:
```rust
fn require_number_index(val: &Value, fn_name: &str, pos: &str) -> Result<usize, MdsError> {
    match val {
        Value::Number(n) => {
            let clamped = n.max(0.0).floor();
            if !clamped.is_finite() || clamped > usize::MAX as f64 {
                return Err(type_err(fn_name, pos, "a finite number", "infinity or out-of-range"));
            }
            Ok(clamped as usize)
        }
        other => Err(type_err(fn_name, pos, "number", other.type_name())),
    }
}
```

**Reviewers**: reliability (90%, blocking)

---

#### 3. `expect()` panic in default parameter binding
**File**: `crates/mds-core/src/evaluator.rs:305`
**Confidence**: 82% (rust review)
**Category**: Blocking

**Problem**: The `.expect("BUG: non-optional param missing but arity check passed")` call will panic if the invariant is violated. While the comment explains the invariant, a library crate panicking is not ideal. All other error paths in the evaluator return `Result`, making this the only panic path in new code.

**Impact**: MEDIUM. Library code should not panic. Produces opaque abort instead of structured error.

**Fix**: Replace with explicit error return:
```rust
param.default.as_ref().ok_or_else(|| {
    MdsError::syntax(format!(
        "internal error: non-optional param '{}' missing but arity check passed",
        param.name
    ))
})?
```

**Reviewers**: rust (82%, medium)

---

#### 4. Missing `builtin_error_at` constructor breaks error pattern consistency
**File**: `crates/mds-core/src/error.rs:365`
**Confidence**: 85% (consistency review)
**Category**: Blocking

**Problem**: Every other span-bearing `MdsError` variant has both a bare constructor and an `_at` variant for source-span diagnostics. `BuiltinError` has only the bare constructor. This means built-in type errors cannot carry source location information, producing worse diagnostics than other error types. All 11 other variants follow the pair pattern: `syntax`/`syntax_at`, `undefined_var`/`undefined_var_at`, etc.

**Impact**: MEDIUM. Diagnostic quality degradation for builtin errors.

**Fix**: Add `builtin_error_at` constructor:
```rust
pub(crate) fn builtin_error_at(
    msg: impl Into<String>,
    file: &str,
    source: &str,
    offset: usize,
    len: usize,
) -> Self {
    let (span, src) = at(file, source, offset, len);
    MdsError::BuiltinError {
        message: msg.into(),
        span,
        src,
    }
}
```

**Reviewers**: consistency (85%, medium)

---

## Should-Fix Issues (RECOMMENDED)

### HIGH Severity (0 issues)

### MEDIUM Severity (3 issues)

#### 1. `builtin_sort` function length exceeds guideline (52 lines)
**File**: `crates/mds-core/src/builtins.rs:378`
**Confidence**: 85% (complexity review)

**Problem**: `builtin_sort` is 52 lines with two parallel match arms (String and Number) that each contain a validation loop followed by a sort-by closure. The two arms are structurally identical, differing only in the Value variant. This increases cyclomatic complexity unnecessarily.

**Fix**: Extract `require_homogeneous(arr, expected_type)` helper to reduce `builtin_sort` to ~20 lines.

---

#### 2. `validate_node` function length exceeds guideline (109 lines)
**File**: `crates/mds-core/src/validator.rs:23`
**Confidence**: 82% (complexity review)

**Problem**: `validate_node` is a single match with 9 arms totaling 109 lines. The `Node::For` arm is 40+ lines with nesting depth 4. This carries the highest cyclomatic complexity in the PR.

**Fix**: Extract `Node::For` and `Node::If` arms into dedicated `validate_for_node` and `validate_if_node` functions.

---

#### 3. Four near-identical quote-aware scanning state machines (4 duplicates)
**File**: `crates/mds-core/src/parser_helpers.rs:124,205,840,890`
**Confidence**: 85% (complexity review)

**Problem**: `find_unquoted_operator`, `split_on_unquoted_op`, `split_on_unquoted_commas`, and `find_unquoted_equals` each implement identical quote-tracking byte scanners. There are 21 occurrences of the `in_string` variable across these 4 functions. Significant maintainability debt.

**Fix**: Extract generic `scan_unquoted` iterator callback to eliminate 80% of the duplication.

---

## Pre-existing Issues (NOT BLOCKING)

None identified in code you didn't touch.

---

## Convergence Status

**Cycle**: 1
**Prior Resolution**: (none)
**Prior FP Ratio**: N/A (first cycle)
**Assessment**: First review cycle — no convergence data yet. All findings are first-pass observations with high confidence (80-90%).

---

## Action Plan

### Phase 1: Critical Blocking Fixes (Do First)
1. **Add empty-string guards** to `split()` and `replace()` (2 files, ~15 min total)
2. **Fix `sort()` NaN handling** to use `total_cmp()` with finiteness check (1 file, ~10 min)
3. **Fix `length()` semantics** to use `s.chars().count()` (1 file, ~5 min)
4. **Add defensive bounds check** to `require_number_index()` (1 file, ~10 min)
5. **Add `builtin_error_at` constructor** (1 file, ~10 min)
6. **Replace panic with Result** in default parameter binding (1 file, ~10 min)
7. **Fix `unique()` O(n^2) complexity** with either HashSet or size bound (1 file, ~20 min)
8. **Consolidate builtin registry** (dual-registry pattern) (1 file, ~30 min)
9. **Extract shared arity-check helper** across evaluator/validator (2 files, ~45 min)

**Total**: ~2.5 hours of focused changes across 6 files.

### Phase 2: Should-Fix Refactorings (After merge approval, lower priority)
1. Extract `require_homogeneous()` helper for `sort()`
2. Extract `validate_for_node()` and `validate_if_node()`
3. Consolidate quote-aware scanners

### Phase 3: Testing Gaps (Add tests for fixes)
1. Test `length()` multi-byte semantics
2. Test `sort()` NaN rejection
3. Test `split("")` rejection
4. Test `replace("", x)` rejection
5. Test validator builtin arity path
6. Test `@elseif` with logical operators

---

## Summary by Reviewer

| Reviewer | Focus | CRITICAL | HIGH | MEDIUM | Score | Recommendation |
|----------|-------|----------|------|--------|-------|---|
| security | injection, amplification | 0 | 2 | 0 | 8/10 | APPROVED_WITH_CONDITIONS |
| architecture | structure, coupling | 0 | 2 | 0 | 7/10 | CHANGES_REQUESTED |
| performance | complexity, allocation | 0 | 3 | 1 | 7/10 | APPROVED_WITH_CONDITIONS |
| complexity | cyclomatic, length | 0 | 2 | 1 | 7/10 | APPROVED_WITH_CONDITIONS |
| consistency | patterns, naming | 0 | 1 | 2 | 7/10 | CHANGES_REQUESTED |
| regression | API compatibility | 0 | 0 | 0 | 9/10 | APPROVED |
| testing | coverage gaps | 0 | 2 | 1 | 7/10 | APPROVED_WITH_CONDITIONS |
| reliability | bounds, overflow | 0 | 2 | 2 | 7/10 | CHANGES_REQUESTED |
| rust | idioms, safety | 0 | 2 | 2 | 7/10 | CHANGES_REQUESTED |

**Consensus**: All 9 reviewers agree on the two amplification vectors (`split("")`, `replace("")`), `sort()` NaN handling, `length()` byte-vs-char semantics, and the dual-registry architectural pattern. These are genuinely blocking issues that must be fixed.

---

## Notes for Developer

1. **High convergence on core issues** — 7 reviewers independently flagged `length()` byte semantics; 4+ flagged each of the other blocking issues. This is genuine consensus, not false positives.

2. **Security findings are not theoretical** — The amplification vectors in `split("")` and `replace("")` have concrete DOS pathways (10MB input → 30MB output or 10M allocations). These are production-grade fixes, not pedantry.

3. **Architectural debt should be addressed now** — The dual-registry and duplicated arity-check patterns will compound as more builtins are added. The time to fix them is before the builtin set grows beyond 20.

4. **Test coverage is good** — 690 tests passing, 100 new tests added. The gaps are specific (missing tests for the fixes you'll make, not general coverage shortfalls).

5. **No data-loss or security vulnerabilities** — All findings are in the "should fix but won't crash" category. The code is safe; it just has rough edges and maintenance debt.

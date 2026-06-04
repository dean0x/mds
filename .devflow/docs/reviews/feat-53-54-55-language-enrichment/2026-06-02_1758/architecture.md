# Architecture Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Dual-registry pattern for builtins creates maintenance coupling** - `crates/mds-core/src/builtins.rs:27-128` and `crates/mds-core/src/builtins.rs:136-158`
**Confidence**: 85%
- Problem: Built-in function registration requires updating two separate structures in lockstep: the `BUILTINS` static array (for metadata/arity) and the `call_builtin` match arms (for dispatch). Adding a new built-in requires coordinated changes in both places. If one is updated without the other, the system either silently rejects a valid function name at dispatch or reports wrong arity at the metadata layer. There is no compile-time enforcement that these two structures stay synchronized.
- Fix: Consider a registry pattern where each builtin registers both its metadata and handler function together, eliminating the possibility of drift:
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
  This eliminates the match arm entirely and makes it impossible to register metadata without a handler or vice versa.

**Arity checking logic duplicated across evaluator and validator with no shared abstraction** - `crates/mds-core/src/evaluator.rs:273-276,343-349` and `crates/mds-core/src/validator.rs:183-194,196-206,229-231,286-298,301-311`
**Confidence**: 82%
- Problem: The arity check pattern (`args.len() < required || args.len() > total` for user-defined, `args.len() < meta.min_args || args.len() > meta.max_args` for builtins) is repeated 5 times across evaluator.rs and validator.rs. The evaluator and validator independently implement the same "resolve function, check arity, check builtins as fallback" decision chain. This means the resolution order (user-defined > builtin > undefined error) is encoded in two places — if it ever changes, both must be updated. The `required_param_count` export from evaluator to validator (per the self-review fix commit) is a symptom of this coupling.
- Fix: Extract a shared function-resolution helper (e.g., in a new `resolution.rs` or in the existing `builtins.rs`) that encapsulates the lookup order and arity range computation:
  ```rust
  pub(crate) enum ResolvedFunction<'a> {
      UserDefined { required: usize, total: usize, func: &'a FunctionDef },
      Builtin { min: usize, max: usize },
  }

  pub(crate) fn resolve_function<'a>(name: &str, scope: &'a Scope) -> Result<ResolvedFunction<'a>, MdsError> { ... }
  ```
  Both evaluator and validator would call `resolve_function` and then apply their respective actions, eliminating the duplicated resolution logic.

### MEDIUM

(none)

## Issues in Code You Touched (Should Fix)

### HIGH

(none)

### MEDIUM

**`length()` uses byte length for strings, conflicting with `slice()` char-boundary snapping** - `crates/mds-core/src/builtins.rs:341` and `crates/mds-core/src/builtins.rs:255-268`
**Confidence**: 80%
- Problem: `length()` returns `s.len()` (byte count) while `slice()` uses `snap_to_char_boundary` to correct byte indices that fall inside multi-byte characters. This means `length("cafe\u{0301}")` returns 6 (bytes), but `slice("cafe\u{0301}", 0, length("cafe\u{0301}"))` would work fine (end clamps). However, the spec says `length(s_or_arr)` returns "String byte length or array count" — this is an intentional design choice but creates a conceptual mismatch where `slice` thinks in char boundaries but `length` reports bytes. Users composing `slice(s, 0, length(s) - 1)` to "trim the last character" would instead trim the last byte, potentially breaking a multi-byte character. This is a design-level tension rather than a bug.
- Fix: Either document this limitation clearly (that `length` returns byte length and `slice` indices are byte-based with char-boundary snapping), or consider adding a `chars` function or changing `length` for strings to return `s.chars().count()`. Given this is v0.2, documenting the semantics clearly may be sufficient.

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`call_function` return type change creates implicit String wrapping** - `crates/mds-core/src/evaluator.rs:339,375` (Confidence: 70%) — `invoke_function` returns `Result<String, MdsError>`, then `call_function` wraps it as `Value::String`. This means user-defined functions always produce `Value::String` while builtins can return any `Value` type. This asymmetry is fine for now (user functions are text templates) but may become a constraint if user-defined functions ever need to return non-string types.

- **`#[allow(clippy::too_many_arguments)]` on `arity_at`** - `crates/mds-core/src/error.rs:343` (Confidence: 65%) — The split of `expected` into `expected_min`/`expected_max` pushed `arity_at` to 8 parameters (triggering clippy). A builder pattern or struct parameter would be more ergonomic but given the crate-internal visibility and infrequent call sites, the suppression is pragmatic.

- **`unique()` uses O(n^2) linear search for deduplication** - `crates/mds-core/src/builtins.rs:436-441` (Confidence: 60%) — `result.contains(item)` is O(n) per element making `unique()` O(n^2). For a template language with typically small arrays this is unlikely to be a problem, but if large arrays are ever processed it could become a bottleneck. A `HashSet`-based approach or `IndexSet` would be O(n) amortized.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 0 | - |
| Should Fix | - | 0 | 1 | - |
| Pre-existing | - | - | 0 | 0 |

**Architecture Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The PR implements three language features cleanly with good separation of concerns. The new `builtins.rs` module is well-contained, the AST extensions are minimal and backward-compatible (applies ADR-008 — bundling related features into one PR), and the parser/evaluator/validator changes follow the existing architectural patterns consistently.

The two HIGH findings are about maintainability coupling rather than correctness:
1. The dual-registry pattern for builtins (metadata array + dispatch match) has no compile-time safety net and will become more fragile as more builtins are added.
2. The arity check + function resolution logic is duplicated across evaluator and validator, creating a maintenance burden where changes must be mirrored.

Neither finding blocks correctness today — the current code is correct and well-tested. But both represent architectural debt that should be addressed before the builtin set grows further. The `length()` byte-vs-char semantic tension is a design consideration worth documenting.

# Architecture Review Report

**Branch**: feat/53-54-55-language-enrichment -> main
**Date**: 2026-06-02

## Issues in Your Changes (BLOCKING)

### MEDIUM

**`required_param_count` is misplaced in `evaluator.rs` — belongs with `Param`/`FunctionDef` in `ast.rs` or `scope.rs`** - `crates/mds-core/src/evaluator.rs:253-256`
**Confidence**: 82%
- Problem: `required_param_count` operates solely on `&[Param]` and has no dependency on the evaluator module. It is a pure data-query function on a type defined in `ast.rs` (`Param`). Placing it in the evaluator creates a cross-layer import: the validator must `use crate::evaluator::required_param_count` to use it, creating a dependency from validator to evaluator. In a strictly layered pipeline (parser -> validator -> evaluator), the validator should not depend on the evaluator.
- Fix: Move `required_param_count` to `ast.rs` as a `pub(crate)` function (or as a method on `Param` / a free function near `Param`). Both evaluator and validator can then import from `ast` — which they already depend on — eliminating the layering inversion. The feature knowledge file itself documents this as a gotcha: "required_param_count is defined in evaluator.rs and imported by validator.rs — not in scope.rs where FunctionDef lives."

```rust
// In crates/mds-core/src/ast.rs, near the Param struct:
impl Param {
    pub(crate) fn count_required(params: &[Param]) -> usize {
        params.iter().filter(|p| p.default.is_none()).count()
    }
}
// Or as a free function:
pub(crate) fn required_param_count(params: &[Param]) -> usize {
    params.iter().filter(|p| p.default.is_none()).count()
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Duplicated function-lookup-and-arity-check pattern across `validate_expr` and `validate_var_args`** (2 occurrences) - `crates/mds-core/src/validator.rs:201-236`, `crates/mds-core/src/validator.rs:306-343`
**Confidence**: 85%
- Problem: The exact same three-branch "try user-defined, try builtin, else undefined" lookup pattern with arity range checking is duplicated in `validate_expr` (for top-level `Expr::Call`) and again in `validate_var_args` (for nested `Arg::Call`). Both branches construct identical `MdsError::arity_at` / `MdsError::undefined_fn_at` calls. This violates DRY and SRP — adding a new function resolution source (e.g., namespace-scoped builtins) would require updating both sites.
- Fix: Extract a helper function like `validate_call_arity(name, arg_count, scope, file, source, offset, len) -> Result<(), MdsError>` that encapsulates the three-branch lookup. Both `validate_expr` and `validate_var_args` call it.

```rust
fn validate_call_arity(
    name: &str,
    arg_count: usize,
    scope: &Scope,
    file: &str,
    source: &str,
    offset: usize,
    len: usize,
) -> Result<bool, MdsError> {
    // Returns Ok(true) if it's a builtin (for caller to decide on validate_var_args path)
    if let Some(func) = scope.get_function(name) {
        let required = required_param_count(&func.params);
        let total = func.params.len();
        if arg_count < required || arg_count > total {
            return Err(MdsError::arity_at(name, required, total, arg_count, file, source, offset, len));
        }
        Ok(false)
    } else if let Some(meta) = crate::builtins::get_builtin(name) {
        if arg_count < meta.min_args || arg_count > meta.max_args {
            return Err(MdsError::arity_at(name, meta.min_args, meta.max_args, arg_count, file, source, offset, len));
        }
        Ok(true)
    } else {
        Err(MdsError::undefined_fn_at(name, file, source, offset, len))
    }
}
```

**Double arity check for builtins: validator AND evaluator both guard independently** - `crates/mds-core/src/evaluator.rs:343-351`, `crates/mds-core/src/validator.rs:217-229`
**Confidence**: 80%
- Problem: The evaluator's `call_function` performs its own arity range check on builtins (`args.len() < meta.min_args || args.len() > meta.max_args`) even though the validator has already checked this for all statically-known calls. The validator is the "trust boundary" — the evaluator should trust validated code. This is a minor redundancy and defense-in-depth concern. The evaluator check adds safety for dynamic dispatch paths (e.g., if the evaluator is ever called without validation), but the architecture explicitly documents "calling evaluate before validate" as an anti-pattern. If that invariant is enforced, the evaluator arity check is dead code for builtins in the normal path.
- Fix: This is acceptable as defense-in-depth if the project wants belt-and-suspenders safety. Consider adding a short comment in the evaluator noting it is a redundant guard for robustness, so future maintainers do not attempt to deduplicate and accidentally remove the safety net for unusual code paths.

## Pre-existing Issues (Not Blocking)

No pre-existing issues found at CRITICAL severity.

## Suggestions (Lower Confidence)

- **`condvalue_to_value` could be a method on `CondValue`** - `crates/mds-core/src/evaluator.rs:244-251` (Confidence: 70%) — This function converts `CondValue` to `Value` but is a free function in the evaluator rather than a method on `CondValue` in `ast.rs`. Moving it to `impl CondValue` would improve discoverability and keep the conversion near the type definition, following the "behavior belongs to the data owner" principle.

- **`BuiltinMeta.handler` uses function pointer, not trait object** - `crates/mds-core/src/builtins.rs:30` (Confidence: 65%) — Using `fn(&[Value]) -> Result<Value, MdsError>` as a function pointer is simple and efficient for the current 18 builtins. If builtins ever need to carry state (e.g., configuration, locale), this would need to change to a trait or closure. For now the function pointer approach is the right call — this is informational.

- **`#[allow(clippy::too_many_arguments)]` on `arity_at`** - `crates/mds-core/src/error.rs:343` (Confidence: 60%) — The `arity_at` constructor now takes 7 arguments (name, expected_min, expected_max, got, file, source, offset, len). This is at the boundary of what is ergonomic. A builder pattern or a params struct could improve readability, but the `_at` pattern is consistent across all error constructors in this module, so changing just this one would break consistency.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

### Overall Assessment

This PR delivers three tightly-coupled language features (applies ADR-008) across ~2800 lines in a well-structured manner. The architectural highlights:

**Strengths:**
- The `builtins.rs` module is a textbook example of a deep module: simple 2-function interface (`get_builtin`/`call_builtin`), rich internal implementation with 18 functions. The `BuiltinMeta` struct with handler function pointer eliminates the need for a separate dispatch match, making the registry the single source of truth.
- The `Param` struct with `Option<CondValue>` default cleanly extends the existing AST type system. Using `CondValue` (already defined for condition literals) as the default-value type is a good reuse of existing abstractions.
- Logical operators are implemented with proper precedence parsing (`||` < `&&`) and the `Condition::And`/`Or` variants integrate cleanly into the existing recursive condition model across validator and evaluator.
- The `MAX_LOGICAL_OPERANDS` limit follows the established defense-in-depth pattern in `limits.rs`.
- Module boundaries are respected: `builtins` is `pub(crate)`, the public API surface (`lib.rs`) is unchanged, and `api_surface.rs` regression tests are updated.
- Validator-evaluator separation is maintained: validation is conservative (all branches checked), evaluation short-circuits.

**Issues:**
- The `required_param_count` placement in `evaluator.rs` creates a validator-to-evaluator dependency that violates the pipeline's layering direction. This is a MEDIUM blocking issue.
- Function resolution logic (user-defined vs. builtin lookup + arity check) is duplicated in the validator. This is a maintainability concern.

The conditions for approval: relocate `required_param_count` to eliminate the layering violation.

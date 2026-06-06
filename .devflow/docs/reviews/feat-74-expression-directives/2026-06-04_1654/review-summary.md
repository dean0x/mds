# Code Review Summary

**Branch**: feat-74-expression-directives -> main
**Date**: 2026-06-04_1654

## Merge Recommendation: CHANGES_REQUESTED

The PR introduces a powerful feature (expressions in directives) with generally sound architecture and comprehensive test coverage. However, it carries three clusters of blocking issues that should be resolved before merge:

1. **Code duplication** (quote/paren scanning state machines replicated 5x) creates high maintainability risk
2. **Missing documentation** (module docstring, function docstrings stale) breaks API clarity
3. **Test gaps** (NotEq operator coverage missing) leave asymmetric validation

Additionally, a memory-efficiency issue in `split()` should be fixed to avoid transient over-allocation in constrained environments.

---

## Issue Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW | Total |
|----------|----------|------|--------|-----|-------|
| Blocking | 0 | 3 | 6 | 0 | 9 |
| Should Fix | 0 | 0 | 4 | 0 | 4 |
| Pre-existing | 0 | 0 | 3 | 0 | 3 |

---

## Blocking Issues (Category 1)

### HIGH Severity

**Duplicated quote/paren scanning state machine (5+ occurrences)** — `crates/mds-core/src/parser_helpers.rs` (95% confidence)
- **Location**: lines 30-85, 93-115, 328-390, 439-489, 607-645
- **Problem**: Five functions (`strip_trailing_directive_colon`, `has_unterminated_string`, `find_unquoted_operator`, `split_on_unquoted_op`, and the inline bare-`=` scanner in `parse_simple_condition`) implement nearly identical byte-level state machines that track `in_string`, `string_char`, `paren_depth` with identical escape handling. This PR added 2 new instances and expanded 2 existing ones. A future change to quoting rules (e.g., backtick strings, bracket notation) would require synchronized updates across all 5+ locations — a classic SRP violation and maintainability trap.
- **Impact**: HIGH — Future grammar extensions become risky; bug fixes in one copy may not propagate to others.
- **Fix**: Extract a reusable `ScanState` struct:
  ```rust
  struct ScanState { in_string: bool, string_char: u8, paren_depth: usize }
  impl ScanState {
      fn advance(&mut self, ch: u8, next: Option<u8>) -> bool { /* skip_next */ }
      fn is_bare(&self) -> bool { !self.in_string && self.paren_depth == 0 }
  }
  ```
  Refactor each scanner to use `ScanState::advance()` + `is_bare()`, reducing each to 5-10 lines of unique logic.

**Duplicated type hierarchy: CondValue and Expr literal variants** — `crates/mds-core/src/ast.rs` (85% confidence)
- **Location**: lines 12-31 (CondValue), 116-123 (Expr)
- **Problem**: The PR adds `StringLiteral`, `NumberLiteral`, `BooleanLiteral`, `NullLiteral` to `Expr` — semantically identical to `CondValue::String/Number/Boolean/Null`. This creates two parallel type hierarchies for the same concept with separate parsing (`parse_cond_value` vs `parse_expr_inner` literal branches), separate conversion functions (`condvalue_to_value` vs `evaluate_expr` literal arms), and no compile-time guarantee they stay in sync.
- **Impact**: HIGH — Violates SRP; increases risk of divergence if literal representation changes.
- **Fix**: Unify by removing `CondValue` and using `Expr` literal variants throughout. Change `Param.default` from `Option<CondValue>` to `Option<Expr>` (restricting to literal variants at parse time). Remove `condvalue_to_value` and `parse_cond_value`, reusing `evaluate_expr` and `parse_expr_inner` paths.

**Module docstring references removed function `parse_dot_path`; omits new functions** — `crates/mds-core/src/parser_helpers.rs:7` (95% confidence)
- **Problem**: Still lists `parse_dot_path` (removed), omits `strip_trailing_directive_colon`, `has_unterminated_string`, `parse_expr_inner`. Readers navigating the module API will miss the new entry points.
- **Fix**: Update module docstring:
  ```rust
  //! - **Condition parsing** — `parse_condition`, `parse_negation_condition`,
  //!   `find_unquoted_operator`, `parse_cond_value`, `parse_expr_inner`,
  //!   `strip_trailing_directive_colon`, `has_unterminated_string`
  ```

**`parse_condition` docstring not updated for expression support** — `crates/mds-core/src/parser_helpers.rs:520-522` (92% confidence)
- **Problem**: Still documents only `var == "value"` forms, but implementation now accepts `func(a) == func(b)` and other arbitrary expressions.
- **Fix**: Update to:
  ```rust
  /// - `var` / `config.debug` / `func(args)` → `Condition::Truthy`
  /// - `!var` / `!func(args)` → `Condition::Not`
  /// - `expr == expr` / `expr != expr` → `Condition::Eq` / `Condition::NotEq`
  ```

**Missing NotEq (!=) operator tests with expression-based conditions** — `crates/mds-core/src/parser_tests.rs` (90% confidence)
- **Problem**: The PR adds `Condition::NotEq(Expr, Expr)` with full expression support. Zero tests for `!=` with expression-based operands (e.g., `@if lower(name) != "bob":`). The `==` path has thorough coverage but `!=` has none.
- **Impact**: HIGH — Asymmetric validation leaves the NotEq code path untested against the new expression grammar.
- **Fix**: Add minimum:
  1. Parser: `@if lower(name) != "alice":` → `Condition::NotEq(Expr::Call, Expr::StringLiteral)`
  2. Evaluator: `@if lower(name) != "bob":` with `name: Alice` → truthy branch

### MEDIUM Severity

**split() allocates full Vec before checking MAX_ARRAY_ELEMENTS limit** — `crates/mds-core/src/builtins.rs:260-269` (95% confidence, cross-confirmed: Security 85% + Performance 85%)
- **Problem**: `.collect()` materializes the entire `Vec<Value>` before checking if element count exceeds 100,000. Input of 10 MB (MAX_FILE_SIZE) split on 1 byte produces ~10 million `Value::String` allocations (~240+ MB peak) before the guard discards it. In WASM with limited memory, this could OOM before the guard fires.
- **Fix**: Use iterator with early-exit count:
  ```rust
  let mut parts: Vec<Value> = Vec::new();
  for p in s.split(sep) {
      if parts.len() >= MAX_ARRAY_ELEMENTS {
          return Err(MdsError::resource_limit(...));
      }
      parts.push(Value::String(p.to_string()));
  }
  ```

**Duplicated byte-scanning logic in expression parser** — `crates/mds-core/src/parser_helpers.rs:128-246` (82% confidence)
- **Problem**: `parse_expr_inner` (~128 lines) reimplements `Var`, `Call`, `QualifiedCall`, and `MemberAccess` parsing logic that already exists in `parse_interpolation_expr` and `parse_dot_expr`. Parallel implementations increase divergence risk if grammar evolves (e.g., method chaining, index access).
- **Fix**: Refactor `parse_interpolation_expr` to call `parse_expr_inner` for core parsing, then wrap result in `Interpolation { expr, offset, len }`. Makes `parse_expr_inner` the canonical expression parser.

**Duplicated logic in `validate_for_node` vs `validate_expr`** — `crates/mds-core/src/validator.rs:89-193` (82% confidence)
- **Problem**: The `Expr::QualifiedCall` arm (lines 146-172) is nearly identical to the one in `validate_expr` (lines 327-354). If validation changes, must update both.
- **Fix**: Delegate non-Var cases to `validate_expr` directly, reducing from 105 to ~50 lines.

**`parse_simple_condition` function complexity (94 lines, CC ~12)** — `crates/mds-core/src/parser_helpers.rs:569-662` (85% confidence)
- **Problem**: High cyclomatic complexity with deeply nested inline bare-`=` scanner (nesting depth 4, 38 lines). New contributors will struggle to follow the logic.
- **Fix**: Extract bare-`=` detection into named helper `has_bare_assignment_operator(s: &str) -> bool`, reducing parent to ~55 lines and nesting depth to 2.

**Silent absorption of unmatched close-parens in scanner functions (4 occurrences)** — `crates/mds-core/src/parser_helpers.rs:67, 368, 475, 632` (82% confidence)
- **Problem**: All scanners use `paren_depth = paren_depth.saturating_sub(1)` for `)`. While preventing panics, it silently accepts unbalanced parens. For `strip_trailing_directive_colon`, this could find a "bare colon" that is actually inside an unbalanced paren group.
- **Fix**: Check `paren_depth > 0` at loop exit and return `None` (indicating malformed input):
  ```rust
  if paren_depth > 0 {
      return None; // Unclosed parenthesis
  }
  ```

**ForBlock import style inconsistent in validator.rs** — `crates/mds-core/src/validator.rs:1, 90` (88% confidence)
- **Problem**: Removed `ForBlock` from top-level import (line 1) but uses `crate::ast::ForBlock` inline at line 90. Every other AST type is imported at the top.
- **Fix**: Add to import line: `use crate::ast::{required_param_count, Arg, Condition, Expr, ForBlock, IfBlock, Node};`

**Inconsistent unterminated-string error handling between @if and @elseif** — `crates/mds-core/src/parser.rs:246-253, 311-312` (85% confidence)
- **Problem**: `parse_if_block` provides targeted error for unterminated strings, but `collect_elseif_branches` only gives generic error. Since the PR introduces expression-containing directives, both should benefit.
- **Fix**: Apply same `has_unterminated_string` check in `collect_elseif_branches`.

**@define still uses strip_suffix(':') while @if/@for use strip_trailing_directive_colon** — `crates/mds-core/src/parser.rs:373-376` (80% confidence)
- **Problem**: Inconsistent directive colon-stripping strategy. Low risk now (colon in defaults is inside parens), but if future changes add expression support to `@define`, this becomes a latent bug.
- **Fix**: Consider using `strip_trailing_directive_colon` for `@define` for uniform directive parsing.

**Stale doc comment references CondValue in Condition's PartialEq rationale** — `crates/mds-core/src/ast.rs:37-38` (85% confidence)
- **Problem**: Says "Condition does not derive PartialEq even though CondValue does," but Condition now holds Expr (not CondValue). Misleading for readers.
- **Fix**: Update to remove CondValue reference:
  ```rust
  /// `Condition` intentionally does **not** derive `PartialEq`.
  /// `Expr::NumberLiteral(f64)` uses IEEE 754 semantics where `NaN != NaN`
  ```

---

## Should-Fix Issues (Category 2)

### MEDIUM Severity

**Missing OR (||) operator test with expression-based operands** — `crates/mds-core/src/parser_tests.rs` (85% confidence)
- **Problem**: `parse_if_and_with_calls` tests `&&` with `Expr::Call` operands, but no equivalent for `||`. The paren-aware `||` splitting was modified but remains untested with expressions.
- **Fix**: Add `parse_if_or_with_calls` and `evaluate_if_or_with_calls` tests.

**evaluate_condition signature change broadens mutability requirement** — `crates/mds-core/src/evaluator.rs:414-418` (80% confidence)
- **Problem**: Changed from `&Scope` to `&mut Scope` because conditions can now invoke functions. While necessary, it means conditions are no longer side-effect-free. This is an accepted architectural trade-off but should be documented explicitly.
- **Fix**: Add comment: "Note: condition expressions may invoke functions that modify scope (push/pop call frames). Short-circuit evaluation means not all operands may be evaluated."

**No @for with qualified call test despite validator coverage** — `crates/mds-core/src/validator.rs:146-172` (80% confidence)
- **Problem**: The validator has dedicated `Expr::QualifiedCall` branch for `@for x in ns.func(args):` with zero test coverage. The `@if` equivalent is tested but `@for` is not.
- **Fix**: Add test exercising `@for x in ns.func(args):` through full compile pipeline.

**CondValue docstring says "RHS of an equality condition" but is now only used for defaults** — `crates/mds-core/src/ast.rs:8` (85% confidence)
- **Problem**: Stale reference to equality conditions. CondValue is now only used for `@define` parameter defaults.
- **Fix**: Update to: "A literal value for a default parameter in `@define` blocks."

**Trivial wrapper `evaluate_condition_value` adds indirection without value** — `crates/mds-core/src/evaluator.rs:405-411` (82% confidence)
- **Problem**: One-line function that just calls `evaluate_expr`. Call sites could call `evaluate_expr` directly, consistent with how `evaluate_for` does at line 617.
- **Fix**: Inline and call `evaluate_expr` directly, or document intent if planning future condition-specific validation.

---

## Pre-existing Issues (Not Blocking)

- **No NotEq operator tests at all** (88% confidence) — `Condition::NotEq` has zero dedicated tests across the entire test suite (pre-dates PR).
- **Large file: parser_helpers.rs at 1373 lines** (80% confidence) — Exceeds critical 500-line threshold. Informational only; future refactoring could split into `scan.rs`, `expr_parse.rs`, `directive_parse.rs`.
- **Repeated byte-level scanning across functions** (85% confidence) — Pre-existing but expanded by this PR; covered under blocking HIGH issue above.

---

## Positive Observations

1. **Type-driven design** (`Expr` in conditions vs `Vec<String>`) unifies the expression model cleanly, eliminating impedance mismatch between parser and evaluator.
2. **Resource limits well-placed**: `MAX_ARRAY_ELEMENTS` (100K) for `split()` and `MAX_OUTPUT_SIZE` (50MB) for `join()` add defense-in-depth against amplification attacks.
3. **Recursion and depth limits enforced**: `MAX_CALL_DEPTH`, `MAX_NESTING_DEPTH`, `MAX_DOT_SEGMENTS`, `MAX_LOGICAL_OPERANDS` all carry forward to new code paths.
4. **Backward compatibility verified**: Existing `@if var:` and `@for item in items:` syntax works unchanged; explicit backward-compatibility tests added.
5. **Test coverage solid**: 764 tests pass, including 31 new Rust tests and 9 Node-API integration tests. High-quality tests following AAA structure.
6. **Security correctly handled**: `strip_trailing_directive_colon` prevents directive-boundary confusion attacks; NaN equality semantics preserved.

---

## Action Plan
1. **Extract `ScanState` abstraction** to consolidate quote/paren scanning across 5+ functions (highest risk)
2. **Unify `CondValue` and `Expr` literals** by removing `CondValue` and using `Expr` throughout
3. **Update all affected docstrings** (module docstring, `parse_condition`, `Condition` PartialEq, `CondValue`)
4. **Fix `split()` allocation** to use bounded iterator instead of collect-then-check
5. **Add missing tests** (NotEq with expressions, OR with expressions, @for with qualified calls)
6. **Document architectural trade-off** on scope mutation in conditions

---

## Convergence Status
**Cycle**: 1 (first review)
**Prior Resolution**: (none)
**Assessment**: First cycle — all findings are new. Multiple reviewers converged on the same high-risk areas (quote/paren scanning duplication, CondValue/Expr duplication, missing test coverage), indicating these are genuine blocking concerns not false positives.

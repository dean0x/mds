# Code Review Summary

**Branch**: chore/release-readiness -> main
**Date**: 2026-05-15_2223
**Reviewers**: 10 (security, architecture, performance, complexity, consistency, regression, testing, reliability, rust, documentation)

## Merge Recommendation: CHANGES_REQUESTED

Documentation fixes required for release notes accuracy before merge. Two blocking architectural issues in run_build and resolver LIFO ordering must be resolved. One HIGH performance issue in evaluate_for must be fixed. All other issues are "should-fix" or pre-existing.

---

## Issue Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| **Blocking** | 0 | 4 | 2 | 0 |
| **Should Fix** | 0 | 0 | 7 | 0 |
| **Pre-existing** | 0 | 0 | 7 | 1 |
| **TOTAL** | 0 | 4 | 16 | 1 |

---

## Blocking Issues (Must Fix Before Merge)

### CRITICAL

**CHANGELOG escape syntax contradicts spec and code** - `CHANGELOG.md:22` (Documentation, Confidence: 98%)
- **Problem**: CHANGELOG states "Escaped braces (`{{` produces `{`)" but actual syntax is `\{` per spec section 4.2 and lexer implementation. Users copying from release notes will use wrong syntax.
- **Fix**: Change line 22 to use correct syntax `\{` instead of `{{`
- **Impact**: Blocks release due to misleading public documentation

### HIGH

**`resolve_input` extracted but only used by `run_check` -- inconsistent decomposition** - `src/main.rs:440-445,457-466` (Architecture, Confidence: 85%; Consistency, Confidence: 92%)
- **Problem**: New `resolve_input()` helper is used only by `run_check` but `run_build` duplicates the same logic inline (with banner). Creates asymmetry -- changes to auto-detection require updating two places.
- **Fix**: Refactor `resolve_input` to return `(PathBuf, bool)` indicating whether auto-detection occurred, then both `run_build` and `run_check` can use it with conditional banner printing.
- **Confidence**: 85-92% (flagged by both architecture and consistency reviewers)
- **Category**: Blocking (YOUR changes created this asymmetry in the extraction)

**`evaluate_for` clones entire iterable array before iteration** - `src/evaluator.rs:288` (Performance, Confidence: 85%)
- **Problem**: Calls `.to_vec()` on iterable slice, allocating full copy of all array elements before iteration. For arrays at `MAX_LOOP_ITERATIONS` (100k elements), doubles peak memory. Pre-existing code but touched in this PR's test additions.
- **Fix**: Iterate over borrowed items directly, cloning only individual items for scope insertion:
  ```rust
  for item in items {
      // ...
      scope.set_var(&block.var, item.clone());
  }
  ```
- **Impact**: HIGH -- Memory efficiency for large loops, straightforward fix

**CHANGELOG Library API section omits 4 public functions** - `CHANGELOG.md:49-53` (Documentation, Confidence: 85%)
- **Problem**: Lists only 7 functions but omits `compile_str_with()`, `check_str_with()`, `compile_str_collecting_warnings()`, `check_str_collecting_warnings()`. Incomplete public API record for v0.1.0.
- **Fix**: Expand Library API section to include all 11 public entry points
- **Category**: HIGH blocking (documentation for release must be complete)

**Resolver LIFO invariant check occurs after early return on error** - `src/resolver.rs:212-222` (Reliability, Confidence: 85%; Regression, Confidence: 82%; Complexity, Confidence: 80%; Architecture, Confidence: 80%)
- **Problem**: LIFO invariant check at line 218 is skipped when `process_module` fails (early return at line 216). If both a module error AND LIFO corruption occur, the corruption goes undetected. While structurally unlikely, violates defensive programming principle for safety-critical code.
- **Fix**: Apply `prefer_first_error` pattern from evaluator (lines 208-220) to check LIFO invariant regardless of module error. Store popped value check result before early return, then validate after module result.
  ```rust
  let popped = self.resolving.pop();
  let lifo_ok = popped.as_ref() == Some(&canonical);
  let resolved = resolved?;
  if !lifo_ok {
      return Err(MdsError::syntax("internal error: resolving stack LIFO invariant violated..."));
  }
  ```
- **Confidence**: 80-85% (flagged by 4 reviewers across multiple perspectives)
- **Category**: Blocking (affects safety invariant verification in YOUR changes)

---

## Should-Fix Issues (High Priority, Recommend Including)

### MEDIUM

**Duplicated `MAX_TRAVERSAL_DEPTH` constant across modules** - `src/main.rs:29`, `src/resolver.rs:47` (Architecture, Consistency, Reliability, Rust, Confidence: 80-82%)
- **Problem**: Both modules independently define `const MAX_TRAVERSAL_DEPTH: usize = 256`. Silent drift risk if one is changed without the other. DRY violation.
- **Fix**: Define once in `resolver.rs` as `pub(crate) const MAX_TRAVERSAL_DEPTH: usize = 256` and import in `main.rs`
- **Flagged by**: 4 reviewers

**`ResolvedModule` fields are all `pub` -- leaky abstraction** - `src/resolver.rs:36-41` (Architecture, Confidence: 82%)
- **Problem**: All fields are `pub` but proper accessor methods exist (`get_export`, `get_all_exports`). Direct field access bypasses export visibility logic. Inconsistent with PR's API hardening via `#[non_exhaustive]` and `pub(crate)`.
- **Fix**: Change fields to `pub(crate)` to enforce accessor method usage
- **Category**: Should Fix (code you touched)

**`run_build` exceeds recommended function length** - `src/main.rs:447-516` (Complexity, Confidence: 85%)
- **Problem**: 70 lines with 4 nesting levels, handles too many responsibilities. Should extract output-writing block into dedicated function.
- **Fix**: Extract lines 492-514 into `write_output(path: Option<PathBuf>, compiled: &str, quiet: bool)` to reduce to ~45 lines
- **Category**: Should Fix (your extraction introduced this)

**LIFO error message inconsistency between resolver and evaluator** - `src/resolver.rs:218-222` vs `src/evaluator.rs:208-215` (Consistency, Confidence: 82%)
- **Problem**: Resolver returns bare error message while evaluator includes diagnostic details (expected vs. got). Resolver version is less debuggable.
- **Fix**: Align resolver error to include diagnostic context like evaluator does
- **Category**: Should Fix (consistency in safety-critical code)

**Constant visibility pattern undocumented** - `src/evaluator.rs:9-22`, `src/resolver.rs:44-50` (Consistency, Confidence: 85%)
- **Problem**: Some constants `pub(crate)`, others private, without clear pattern documented. `MAX_FILE_SIZE` is re-exported as `pub const` in lib.rs while others aren't.
- **Fix**: Add doc comments explaining when constants are `pub(crate)` vs private
- **Category**: Should Fix (documentation for maintainability)

**Validator `@if` shares mutable scope between branches** - `src/validator.rs:23-38` (Reliability, Confidence: 82%)
- **Problem**: `@if` validates then_body and else_body against same `&mut Scope` frame. Future directives (e.g., hypothetical `@let`) at then-body level would leak into else_body validation. Currently no such construct, but latent coupling.
- **Fix**: Document invariant that `@if` does not push scope frame
- **Category**: Should Fix (document for future safety)

**`canonicalize_and_check` performs redundant syscalls on cache hits** - `src/resolver.rs:128-144` (Performance, Confidence: 83%)
- **Problem**: Calls `canonicalize_and_check` before cache lookup. For projects with 20+ imports resolving to same files, pays security check cost on every resolve, not just misses.
- **Fix**: Move cache lookup before full security check by computing canonical path first, then checking cache before other checks
- **Category**: Should Fix (performance in touched code)

---

## Suggestions (Lower Confidence, 60-79%)

**`run_build` does not use `resolve_input_verbose`** - `src/main.rs:458-467` (Complexity, Consistency, Rust, Confidence: 70-82%)
- Problem: Overlaps with blocking issue above. Duplication of pattern.

**`format!` allocation in `call_qualified_function`** - `src/evaluator.rs:243` (Performance, Confidence: 80%)
- Problem: Allocates `format!("{namespace}.{name}")` on every qualified call. Low impact but optimization opportunity.

**`run_check` does not print "Checking ..." banner** - `src/main.rs:527` (Consistency, Confidence: 70%)
- Problem: `run_build` prints banner on auto-detect, `run_check` does not. UX consistency gap.

**Spec does not document single-quote string literals** - `spec.md` (Documentation, Confidence: 82%)
- Problem: Parser accepts `'string'` and `"string"` but spec only shows double-quoted examples. Should align documentation.

**README lists --quiet under Build options but it's global** - `README.md:59` (Documentation, Confidence: 82%)
- Problem: `--quiet` flag is defined as global but documented under "Build options" only. Implies it only works with build.

**Spec section 7.3 omits --quiet for check command** - `spec.md:394` (Documentation, Confidence: 80%)
- Problem: Doesn't mention `--quiet` is available for `mds check`.

---

## Pre-existing Issues (Not Blocking)

| Issue | File | Confidence | Notes |
|-------|------|-----------|-------|
| `ModuleCache` has no limit on total cached modules | `src/resolver.rs:54-62` | 80% | Acceptable for v0.1; for future optimization |
| `collect_define` eagerly snapshots scope for every `@define` | `src/resolver.rs:594-602` | 82% | Architectural, acceptable for v0.1 with typical module sizes |
| `evaluate_nodes` output string with no size hint | `src/evaluator.rs:59` | 80% | Minor optimization; use `with_capacity()` |
| `build_cycle_string` allocates intermediate Vec | `src/resolver.rs:719-725` | 80% | Error path only; minor optimization |
| Spec grammar omits `\}` escape | `spec.md:621` | 85% | Update grammar to include both `\{` and `\}` |
| Validator `scope.pop()` result discarded | `src/validator.rs:62,74` | 70% | Correct per comment; add `debug_assert!` for robustness |

---

## Testing Coverage

### High-Confidence Test Gaps (Should Address)

**`output_size_limit_rejects_oversized_output` allocates 50 MB** - `src/evaluator.rs:622` (Confidence: 85%)
- Problem: Allocates full 50 MB string in single allocation. Heavy for unit test; can cause OOM in CI.
- Fix: Use two smaller allocations or loop of text nodes accumulating past limit
- **Recommendation**: Fix before merge to prevent flaky CI

**`warning_cap_at_max_warnings` assertion is loose** - `tests/integration.rs:3186-3194` (Confidence: 82%)
- Problem: Asserts `<= 1000` but should verify exactly 1000. One-sided boundary test.
- Fix: Change to `assert_eq!(warnings.len(), 1000, ...)`

**`call_depth_limit_rejects_deep_call_stack` error assertion is loose** - `src/evaluator.rs:606-609` (Confidence: 80%)
- Problem: Checks for "call depth" OR "recursion" OR "128". Should tighten to expected message.
- Fix: Assert only for "call depth exceeds"

---

## Summary by Reviewer

| Reviewer | Score | Recommendation | Key Finding |
|----------|-------|-----------------|------------|
| Security | 9/10 | APPROVED | No blocking issues; strong hardening |
| Architecture | 8/10 | APPROVED_WITH_CONDITIONS | `resolve_input` asymmetry + `MAX_TRAVERSAL_DEPTH` duplication |
| Performance | 7/10 | APPROVED_WITH_CONDITIONS | `.to_vec()` in `evaluate_for` is HIGH blocking |
| Complexity | 7/10 | APPROVED_WITH_CONDITIONS | `run_build` at 70 lines needs extraction |
| Consistency | 8/10 | APPROVED_WITH_CONDITIONS | `resolve_input` usage inconsistency + LIFO error messages |
| Regression | 9/10 | APPROVED_WITH_CONDITIONS | LIFO check ordering risk (low impact given structure) |
| Testing | 8/10 | APPROVED_WITH_CONDITIONS | 50 MB test allocation + loose boundary assertions |
| Reliability | 9/10 | APPROVED_WITH_CONDITIONS | LIFO check skipped on error path + validator scope coupling |
| Rust | 8/10 | APPROVED | No blocking; strong patterns throughout |
| Documentation | 7/10 | CHANGES_REQUESTED | CRITICAL: CHANGELOG escape syntax wrong + HIGH: API omissions |

---

## Action Plan

### Phase 1: Fix Blocking Issues (Must Do Before Merge)
1. **Fix CHANGELOG escape syntax** (`{{` → `\{`) — 5 min
2. **Complete CHANGELOG Library API section** with 4 omitted functions — 10 min
3. **Fix `resolve_input` inconsistency** — extract to support both run_build and run_check — 20 min
4. **Fix `evaluate_for` .to_vec()` allocation** — iterate borrowed items — 10 min
5. **Fix resolver LIFO invariant check ordering** — move validation before early return — 15 min

**Time estimate for blocking fixes**: ~60 minutes

### Phase 2: Should-Fix Issues (Recommended for v0.1)
1. Extract `MAX_TRAVERSAL_DEPTH` to shared location — 15 min
2. Make `ResolvedModule` fields `pub(crate)` — 5 min
3. Extract `write_output` function from `run_build` — 20 min
4. Align LIFO error messages between resolver and evaluator — 5 min
5. Fix test allocations and assertions — 15 min
6. Fix README --quiet placement and spec --quiet documentation — 10 min

**Time estimate for should-fix**: ~70 minutes

### Phase 3: Nice-to-Have (Post-v0.1)
- Document constant visibility pattern
- Optimize `canonicalize_and_check` cache hit path
- Minor performance optimizations (`format!` in `call_qualified_function`, etc.)
- Update spec grammar for single-quote strings and `\}` escape

---

## Confidence Levels

Blocking issues have 80-98% confidence (multiple independent reviewers agreeing). No CRITICAL issues in active code, but CRITICAL in documentation (release notes accuracy).

**Consensus findings** (flagged by 4+ reviewers):
- `resolve_input` inconsistency: 4 reviewers (85-92% confidence)
- `MAX_TRAVERSAL_DEPTH` duplication: 4 reviewers (80-82% confidence)
- LIFO invariant ordering: 4 reviewers (80-85% confidence)
- Escape syntax in CHANGELOG: 1 reviewer (98% confidence — documentation is unambiguous)

---

## Overall Assessment

**Release readiness**: 85% complete. Strong hardening work across security, performance, and reliability. Extraction refactoring is clean and reduces code complexity well. The 10 reviewer perspectives show high agreement on key issues:

1. **What's working well**: Security hardening, API surface tightening, test coverage expansion, panic elimination, resource limits, TOCTOU fix
2. **What needs attention**: Documentation accuracy for release, architectural consistency in extracted helpers, one performance optimization in hot path
3. **Risk level**: LOW — no data loss risks, no security vulnerabilities in new code, all changes preserve existing behavior

**Recommendation**: Fix the 5 blocking issues (1-2 hours) and merge. The should-fix issues should be tracked as tech debt for post-v0.1 if time is constrained, but all are straightforward and low-risk.

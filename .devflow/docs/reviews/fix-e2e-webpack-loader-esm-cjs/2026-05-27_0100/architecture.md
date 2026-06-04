# Architecture Review Report

**Branch**: fix-e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27T01:00:00Z

## Issues in Your Changes (BLOCKING)

### HIGH

**Duplicated path-resolution logic across `evaluate_condition` variants** - `crates/mds-core/src/evaluator.rs:346-362`
**Confidence**: 85%
- Problem: The diff introduces both a `resolve_condition_path` helper (line 346) and an `evaluate_condition` function (line 355). The `resolve_condition_path` helper correctly factors out the dot-path resolution, but the same "empty path" internal-error guard (`MdsError::syntax("internal error: @if block has empty condition path")`) is now expressed in three places: `resolve_condition_path` (line 348), and the two validator call sites in `validate_condition`. While `evaluate_condition` itself is clean, the error-message string is duplicated across the evaluator and validator without a shared constant. This is a minor SRP/DRY violation -- the invariant ("condition path must be non-empty") is enforced identically in two modules with string literals.
- Fix: Extract the internal-error message string as a `const` in `ast.rs` or a shared module, and have both `resolve_condition_path` (evaluator) and `validate_condition` (validator) reference it. Alternatively, since `Condition::path()` already guarantees access to the inner path, consider a `Condition::root()` method that returns `Result<&str, MdsError>` to centralize the invariant check.

```rust
// In ast.rs — Condition impl
pub fn root(&self) -> Result<&str, MdsError> {
    self.path().first().map(|s| s.as_str()).ok_or_else(|| {
        MdsError::syntax("internal error: @if block has empty condition path")
    })
}
```

### MEDIUM

**Stale comment left in `parse_condition`** - `crates/mds-core/src/parser.rs:539-543`
**Confidence**: 90%
- Problem: Lines 539-543 contain a block comment describing a "pre-check" for bare `=` that says "This is handled below after the equality check." This is a leftover note from an earlier iteration -- the actual bare `=` check is at lines 588-599. The comment at the top of the function is misleading: it describes an intent to check early but then says it is handled later. This adds no value and confuses readers about the control flow.
- Fix: Remove the stale comment block (lines 539-543).

```rust
fn parse_condition(s: &str) -> Result<Condition, MdsError> {
    let s = s.trim();

    // Negation prefix
    if let Some(rest) = s.strip_prefix('!') {
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`parse_body` signature grew a second terminator parameter without a type alias or builder** - `crates/mds-core/src/parser.rs:113-117`
**Confidence**: 82%
- Problem: `parse_body` now takes two slice parameters (`exact_terminators: &[&str]` and `prefix_terminators: &[&str]`) that are positionally identical in type. At each of the 7 call sites (lines 45, 228, 255, 269, 325, 384, and in `parse_if_block`), the caller must remember which slice is "exact" and which is "prefix" -- there is no type-level distinction. Most call sites pass `&[]` for prefix_terminators, which works but the API shape invites accidental swaps that the compiler cannot catch.
- Fix: This is acceptable for now given the small number of call sites, but consider introducing a struct or enum if more terminator modes are added:

```rust
enum Terminator<'a> {
    Exact(&'a str),
    Prefix(&'a str),
}

fn parse_body(&mut self, terminators: &[Terminator]) -> Result<Vec<Node>, MdsError>
```

**`_esmImport` wrapper uses `new Function()` -- architecturally necessary but should be documented as a security-reviewed pattern** - `packages/webpack-loader/src/index.ts:10-13`
**Confidence**: 80%
- Problem: The `new Function('id', 'return import(id)')` pattern is a well-known workaround for TypeScript's CJS `import()` rewriting (TS#43329), and the comment explains the "why" well. However, `new Function()` is essentially `eval` and bypasses CSP in browser contexts. While this is a Node.js-only webpack loader (browser CSP is irrelevant), this pattern should carry a brief note that it was security-reviewed and is safe specifically because: (a) the `id` argument is always a hard-coded string (`'@mds/mds'`), never user input; (b) this runs in Node.js build tooling, not in a browser.
- Fix: Add a one-line comment reinforcing the security posture:

```typescript
// SECURITY: Safe — `id` is always the hard-coded string '@mds/mds', never user input.
// This runs in Node.js build tooling only; CSP is not applicable.
const _esmImport: (id: string) => Promise<unknown> = new Function(
```

## Pre-existing Issues (Not Blocking)

No critical pre-existing issues found in the reviewed files.

## Suggestions (Lower Confidence)

- **`IfBlock.elseif_branches` uses `Vec<(Condition, Vec<Node>)>` tuple instead of a named struct** - `crates/mds-core/src/ast.rs:132` (Confidence: 70%) -- A named struct (e.g., `ElseIfBranch { condition: Condition, body: Vec<Node> }`) would be more self-documenting than a tuple, especially when pattern-matched in evaluator and validator code. Low priority since the tuple is only destructured in a few places.

- **`MAX_ELSEIF_BRANCHES` check is off-by-one (allows 257 branches)** - `crates/mds-core/src/parser.rs:259` (Confidence: 75%) -- The check `if elseif_branches.len() > MAX_ELSEIF_BRANCHES` fires only after the branch is pushed, and `>` (not `>=`) means it rejects at 257 branches, not 256. If the intent is "max 256 branches", the check should be `>=` or moved before the push. Minor because the limit is defensive, not semantic.

- **Dual-build script uses inline `node -e` for package.json generation** - `packages/bundler-utils/package.json:26` and `packages/webpack-loader/package.json:22` (Confidence: 65%) -- The build script inlines `node -e "require('fs').writeFileSync('dist-cjs/package.json', ...)"` which is fragile and duplicated across two packages. A shared script or build plugin would be cleaner if more packages adopt dual builds.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Architecture Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The architecture of both changes is sound. The Rust-side template language extension (Condition enum, CondValue, elseif_branches) follows the existing AST/parser/evaluator/validator layering cleanly. The new types are well-placed in the AST module, condition parsing is correctly extracted into standalone functions (`parse_condition`, `parse_dot_path`, `parse_cond_value`, `find_unquoted_operator`), and the evaluator's `evaluate_condition` dispatches through a single match. The webpack CJS dual-build approach is standard and the `_esmImport` workaround is a known pattern for the TypeScript CJS import rewriting issue.

Conditions for approval:
1. Remove the stale comment block in `parse_condition` (lines 539-543) to avoid reader confusion.
2. Consider (non-blocking) extracting the duplicated internal-error message into a shared location or `Condition::root()` method to prevent future drift.

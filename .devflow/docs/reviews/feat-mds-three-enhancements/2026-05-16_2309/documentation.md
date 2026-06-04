# Documentation Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### HIGH

**Incorrect function signature in KNOWLEDGE.md** - `.features/mds-compiler/KNOWLEDGE.md:298`
**Confidence**: 95%
- Problem: The documented signature `resolve_dot_path(path: &[String], scope: &Scope) -> Result<Value, MdsError>` does not match the actual implementation `resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError>`. The function takes three parameters (root as `&str`, fields as `&[String]`, and scope), not two. The description below it ("walks a dot-separated path starting from `path[0]`...") matches the 2-param mental model but not the actual code.
- Fix: Update line 298 of KNOWLEDGE.md to:
  ```
  **`resolve_dot_path(root: &str, fields: &[String], scope: &Scope) -> Result<Value, MdsError>`**: Private function that resolves a dot-path. `root` is looked up in scope as the starting variable, then `fields` are traversed into `Value::Object` values. Returns `MdsError::undefined_var` if the root is missing, or `MdsError::syntax` if a field is missing or an intermediate value is not an object.
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Spec Section 4.1 frontmatter example lacks an object variable** - `spec.md:36-43`
**Confidence**: 82%
- Problem: Section 4.1 now documents object support ("Types supported: string, number, boolean, array, object") and describes dot-notation access, but the code example only shows string, array, boolean, and number variables. A user reading the example first would not see how to define an object in frontmatter.
- Fix: Add an object variable to the example:
  ```mds
  ---
  name: Alice
  items: [apple, banana]
  premium: true
  count: 3
  config:
    debug: true
    greeting: Hello
  ---
  ```

**Falsy values list omits `NaN`** - `spec.md:92`
**Confidence**: 80%
- Problem: The falsy values bullet was modified in this branch (added `empty object {}`), but it still omits `NaN` which is also falsy per the implementation (`Value::Number(n) => *n != 0.0 && !n.is_nan()`). The KNOWLEDGE.md correctly lists `NaN` as falsy. Since this line was already being edited, the omission should be fixed while here.
- Fix: Change to: `Falsy values: false, null, empty string "", empty array [], empty object {}, 0, NaN`

## Pre-existing Issues (Not Blocking)

(none at CRITICAL severity)

## Suggestions (Lower Confidence)

- **Spec Section 4.4 loop example uses bare `config` but `config` is not defined in the example** - `spec.md:110` (Confidence: 65%) -- The key-value iteration example `@for key, value in config:` references a `config` variable that isn't shown being defined in any surrounding context. Adding a brief comment or frontmatter snippet would help clarity.

- **KNOWLEDGE.md evaluator section could document `resolve_dot_path` call sites more precisely** - `.features/mds-compiler/KNOWLEDGE.md:300-304` (Confidence: 62%) -- The four listed call sites reference expression variant names rather than function names (e.g., "evaluate_expr(Expr::MemberAccess)"). While technically correct, new developers might find it clearer to see the actual evaluator function names that contain the calls.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 0 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Documentation Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The documentation update is comprehensive and largely accurate. The spec correctly documents the three new features (object types, frontmatter preservation, escape sequences), the grammar summary is consistent with the AST, and the KNOWLEDGE.md thoroughly covers architecture details. The single blocking issue is a function signature mismatch in the developer documentation that could mislead contributors working with `resolve_dot_path`. The two should-fix items are minor completeness gaps in the user-facing spec.

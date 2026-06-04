# Testing Review Report

**Branch**: feat/mds-three-enhancements -> main
**Date**: 2026-05-16

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Missing parser unit tests for MemberAccess expression parsing** - `src/parser.rs`
**Confidence**: 85%
- Problem: The parser was significantly changed to produce `Expr::MemberAccess` and `Arg::MemberAccess` variants from dot-notation expressions (lines 506-555, 683-697), but no unit tests were added to the parser's `mod tests` block to verify this parsing. The existing `parse_qualified_call` test covers `{utils.greet("Alice")}` but there are no parser-level tests for `{obj.key}` -> `MemberAccess`, `{a.b.c}` -> multi-level MemberAccess, or `{greet(config.name)}` -> `Call` with `Arg::MemberAccess`. Integration tests cover the end-to-end behavior, but the parser is a distinct unit with its own test module and should have coverage for the new branching logic in `parse_interpolation_expr` and `parse_single_arg_inner`.
- Fix: Add parser unit tests:
```rust
#[test]
fn parse_member_access_single_field() {
    let src = "{obj.key}";
    let tokens = tokenize(src, "test.mds").unwrap();
    let module = parse_with_ctx(&tokens, "", "").unwrap();
    if let Node::Interpolation(interp) = &module.body[0] {
        assert!(matches!(interp.expr, Expr::MemberAccess { .. }));
    } else {
        panic!("expected Interpolation node with MemberAccess");
    }
}

#[test]
fn parse_member_access_multi_level() {
    let src = "{a.b.c}";
    let tokens = tokenize(src, "test.mds").unwrap();
    let module = parse_with_ctx(&tokens, "", "").unwrap();
    if let Node::Interpolation(interp) = &module.body[0] {
        if let Expr::MemberAccess { object, fields } = &interp.expr {
            assert_eq!(object, "a");
            assert_eq!(fields, &["b".to_string(), "c".to_string()]);
        } else {
            panic!("expected MemberAccess");
        }
    } else {
        panic!("expected Interpolation node");
    }
}

#[test]
fn parse_arg_member_access() {
    let result = parse_single_arg("config.name");
    assert!(matches!(result.unwrap(), Arg::MemberAccess { .. }));
}
```

**Missing unit tests for `strip_type_mds` and `prepend_frontmatter`** - `src/lib.rs:342-372`
**Confidence**: 82%
- Problem: Two new private functions (`strip_type_mds` and `prepend_frontmatter`) were added to `src/lib.rs` but have no unit tests in the existing `mod tests` block (which currently only tests `clean_output`). These functions have branching logic: `strip_type_mds` returns `None` when only `type: mds` remains, and `prepend_frontmatter` has three exit paths (no raw, empty after strip, success). Integration tests cover the main paths, but edge cases are better caught at the unit level.
- Fix: Add unit tests to the existing `src/lib.rs` test module:
```rust
#[test]
fn strip_type_mds_removes_type_line() {
    let result = strip_type_mds("type: mds\nname: Alice\n");
    assert_eq!(result, Some("name: Alice\n".to_string()));
}

#[test]
fn strip_type_mds_returns_none_when_only_type() {
    let result = strip_type_mds("type: mds\n");
    assert!(result.is_none());
}

#[test]
fn strip_type_mds_preserves_other_type_values() {
    // "type: page" should NOT be stripped
    let result = strip_type_mds("type: page\nname: Alice\n");
    assert!(result.unwrap().contains("type: page"));
}

#[test]
fn prepend_frontmatter_none_raw_returns_body() {
    let result = prepend_frontmatter(None, "body".to_string());
    assert_eq!(result, "body");
}

#[test]
fn prepend_frontmatter_with_content() {
    let result = prepend_frontmatter(Some("name: Alice\n"), "Hello!\n".to_string());
    assert_eq!(result, "---\nname: Alice\n---\nHello!\n");
}
```

**Missing parser unit test for `@for key, value in obj:` syntax** - `src/parser.rs:258-293`
**Confidence**: 83%
- Problem: The `parse_for_block` method was substantially rewritten to support key-value destructuring (`@for key, value in obj:`) and dot-path iterables (`@for item in config.items:`). No parser unit tests verify that these produce `ForBlock` nodes with the correct `key_var` and iterable `Vec<String>` fields. Only the existing `parse_for_block` test verifies the old `@for item in items:` pattern.
- Fix: Add parser unit tests:
```rust
#[test]
fn parse_for_key_value() {
    let src = "@for key, value in obj:\n{key}\n@end\n";
    let tokens = tokenize(src, "test.mds").unwrap();
    let module = parse_with_ctx(&tokens, "", "").unwrap();
    if let Node::For(block) = &module.body[0] {
        assert_eq!(block.key_var, Some("key".to_string()));
        assert_eq!(block.var, "value");
    } else {
        panic!("expected For node");
    }
}

#[test]
fn parse_for_dot_path_iterable() {
    let src = "@for item in config.items:\n{item}\n@end\n";
    let tokens = tokenize(src, "test.mds").unwrap();
    let module = parse_with_ctx(&tokens, "", "").unwrap();
    if let Node::For(block) = &module.body[0] {
        assert_eq!(block.iterable, vec!["config".to_string(), "items".to_string()]);
    } else {
        panic!("expected For node");
    }
}
```

**Missing parser unit test for `@if config.debug:` dot-path condition** - `src/parser.rs:215-226`
**Confidence**: 82%
- Problem: The `parse_if_block` method now splits the condition string on `.` to produce a `Vec<String>`, but the existing `parse_if_block` test only checks `matches!(module.body[0], Node::If(_))` without verifying the condition vector. No test covers a dot-path condition like `@if config.debug:`.
- Fix: Add a parser unit test:
```rust
#[test]
fn parse_if_dot_path_condition() {
    let src = "@if config.debug:\nDebug!\n@end\n";
    let tokens = tokenize(src, "test.mds").unwrap();
    let module = parse_with_ctx(&tokens, "", "").unwrap();
    if let Node::If(block) = &module.body[0] {
        assert_eq!(block.condition, vec!["config".to_string(), "debug".to_string()]);
    } else {
        panic!("expected If node");
    }
}
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`truthiness` unit test in `value.rs` not updated for Object variant** - `src/value.rs:233-243`
**Confidence**: 85%
- Problem: The existing `truthiness` test (line 233) exercises all original `Value` variants but was not updated to include the new `Object` variant. While separate `object_truthiness_empty` and `object_truthiness_non_empty` tests were added (lines 320-335), the canonical `truthiness` test that serves as a single-glance overview of all truthiness rules remains incomplete. This is a documentation/consistency concern: someone reading `truthiness` expects it to be comprehensive.
- Fix: Add Object truthiness assertions to the existing test:
```rust
#[test]
fn truthiness() {
    // ... existing assertions ...
    assert!(!Value::Object(HashMap::new()).is_truthy());
    let mut m = HashMap::new();
    m.insert("k".to_string(), Value::Null);
    assert!(Value::Object(m).is_truthy());
}
```

**Feature knowledge document contains stale information contradicting the changes** - `.features/mds-compiler/KNOWLEDGE.md:134,489`
**Confidence**: 90%
- Problem: The feature knowledge document at `.features/mds-compiler/KNOWLEDGE.md` contains two statements that directly contradict the changes in this branch:
  - Line 134: "The Value enum has five variants: String, Number(f64), Boolean, Array(Vec<Value>), Null. Objects/maps are explicitly unsupported in v0.1."
  - Line 489: "Object types unsupported -- YAML mappings and JSON objects are rejected at the value conversion layer."
  - Line 525: "dot-notation variable access is rejected"
  These are now false. Object/map support and dot-notation are the primary features being added. While this is a documentation issue rather than a testing issue, stale feature knowledge leads to incorrect test expectations and review conclusions in future work. The knowledge document also shapes how the `Arg` enum section (line 85-93) describes only three variants when there are now four (`Arg::MemberAccess`).
- Fix: Update the feature knowledge to reflect the new reality: six Value variants, objects supported, dot-notation access supported, four Arg variants. (Some of these updates appear to be in the unstaged changes already.)

## Pre-existing Issues (Not Blocking)

### LOW

**Existing tests modified from error-asserting to success-asserting without regression guard** - `tests/integration.rs:1537-1542,1702-1707`
**Confidence**: 80%
- Problem: Two tests (`dot_notation_variable_access_gives_clear_error` and `yaml_map_type_rejected`) that previously asserted that certain syntax was rejected have been renamed and rewritten to assert success. This is correct behavior for a feature-addition branch, but there is no explicit test that the old error path is gone -- i.e., that `{obj.key}` specifically does NOT produce a "dot notation for variables is not supported" error anymore. While the success assertion implicitly covers this, a comment acknowledging the behavioral change would aid future reviewers.
- Fix: No code change required. The success assertion implicitly validates this. Consider adding a brief inline comment noting the behavioral change, e.g., `// Previously rejected with "dot notation not supported"; now supported via MemberAccess.`

## Suggestions (Lower Confidence)

- **Missing test for `Arg::MemberAccess` depth-bounded validation** - `src/validator.rs:205-211` (Confidence: 70%) -- The validator's `validate_var_args` handles `Arg::MemberAccess` but there is no test that a deeply nested function call with a MemberAccess argument is validated correctly. This is likely covered implicitly but a targeted unit test would be more robust.

- **No test for namespace-vs-object disambiguation error** - `src/evaluator.rs:159-163` (Confidence: 72%) -- When `{ns.key}` is used where `ns` is an imported module namespace (not a variable), the evaluator produces a specific error. No integration test covers this disambiguation path. Consider a test where an alias-imported module name collides with an object variable name.

- **`strip_type_mds` edge case: `type:mds` without space** - `src/lib.rs:346-349` (Confidence: 65%) -- The `strip_prefix("type:")` then `.trim() == "mds"` handles `type: mds` and `type:mds` (no space) and `type:  mds` (extra spaces). But there is no test for the no-space variant, which could regress silently.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 4 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 1 |

**Testing Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The test suite is solid overall with 325 passing tests and good coverage of the three new features (object/map support, frontmatter preservation, escape docs fixes). The branch adds 26 new integration tests and 10 new unit tests covering happy paths, error paths, edge cases (empty objects, nested objects, depth limits), and cross-feature interactions (objects in arrays, objects in frontmatter, function arguments with dot paths).

The main gap is the absence of parser-level unit tests for the new `MemberAccess` expression variant, dot-path conditionals, and key-value for-loop destructuring. These parsing changes are substantial (new branching logic in `parse_interpolation_expr`, `parse_single_arg_inner`, `parse_if_block`, `parse_for_block`) but are only validated through end-to-end integration tests. Adding parser unit tests would catch parsing regressions earlier and with more precise failure messages. Similarly, the new `strip_type_mds` and `prepend_frontmatter` functions in `lib.rs` have branching logic that deserves unit-level coverage alongside their integration-level coverage.

None of these gaps represent missing coverage of critical paths -- the integration tests do cover the behavior. The issues are about test architecture: unit tests for unit-testable functions, following the existing pattern in the codebase where each module has its own `mod tests`.

# Reliability Review Report

**Branch**: fix/e2e-webpack-loader-esm-cjs -> main
**Date**: 2026-05-27T03:06Z

## Issues in Your Changes (BLOCKING)

### MEDIUM

**MAX_ELSEIF_BRANCHES check is off-by-one: allows parsing one extra branch before rejection** - `crates/mds-core/src/parser.rs:273`
**Confidence**: 82%
- Problem: The `MAX_ELSEIF_BRANCHES` limit check occurs AFTER the (N+1)th branch's condition and body have already been fully parsed (lines 268-271), but BEFORE the push (line 279). When exactly 256 branches are in the vec, the loop enters one more iteration, parses the 257th branch's condition and body entirely, then returns the error. The limit on the Vec size is correct (never exceeds 256), but the parser does unnecessary work parsing a branch it will discard, and the error message says "more than 256" when exactly 257 branches exist. More importantly, the check should be at the top of the loop iteration, before doing parsing work, to match the `enter_block()` pattern used elsewhere.
- Fix: Move the check before parsing the branch:
```rust
while let Some(Token::Directive(d, _)) = self.peek() {
    if !d.trim().starts_with("@elseif ") {
        break;
    }
    // Check limit BEFORE parsing the next branch
    if elseif_branches.len() >= MAX_ELSEIF_BRANCHES {
        return Err(MdsError::syntax(format!(
            "@if block has more than {MAX_ELSEIF_BRANCHES} @elseif branches"
        )));
    }
    // Consume the @elseif directive token
    let elseif_dir = d.clone();
    self.pos += 1;
    // ... rest of parsing ...
    elseif_branches.push((elseif_cond, elseif_body));
}
```

**`findProjectRoot` uses synchronous `existsSync` in async call chain** - `packages/mds/src/util/module-scanner.ts:29`
**Confidence**: 80%
- Problem: `findProjectRoot` calls `existsSync` up to `2 * MAX_TRAVERSAL_DEPTH` (512) times in the worst case (2 markers per directory, 256 directories). Each `existsSync` blocks the Node.js event loop. On network-mounted filesystems or slow disks with deep directory trees, this could block the event loop for noticeable periods. The function is called once per `buildModulesMap`, so the total count is bounded, but the bound (512 sync I/O calls) is higher than ideal.
- Fix: Replace `existsSync` with `fs/promises` `access` and make `findProjectRoot` async:
```typescript
import { access } from 'node:fs/promises';

export async function findProjectRoot(start: string): Promise<string> {
  let dir = start;
  for (let i = 0; i < MAX_TRAVERSAL_DEPTH; i++) {
    for (const marker of PROJECT_ROOT_MARKERS) {
      try {
        await access(resolve(dir, marker));
        return dir;
      } catch { /* not found, continue */ }
    }
    const parent = dirname(dir);
    if (parent === dir) return start;
    dir = parent;
  }
  return start;
}
```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **`values_equal` f64 comparison may surprise users with near-equal floats** - `crates/mds-core/src/evaluator.rs:339` (Confidence: 65%) -- `n == e` uses exact IEEE 754 equality for `f64`, meaning `@if rate == 3.14:` could fail for YAML-parsed floats due to representation error. The spec says "strict equality" and tests cover this, so this is by design, but users may be surprised. Consider documenting this behavior explicitly in error messages or warnings.

- **`_esmImport` via `new Function` has no error wrapping** - `packages/webpack-loader/src/index.ts:10-13` (Confidence: 62%) -- If `import(id)` fails at runtime (e.g., module not found in CJS context), the error surfaces as a generic rejection from the dynamically constructed function with no stack trace pointing to the webpack loader. The `getLazy` factory does have a shape-check on the result (line 41), which catches partial failures, but a total import failure would surface as an opaque error.

- **`MAX_ELSEIF_BRANCHES` (256) vs `MAX_NESTING_DEPTH` (64) mismatch** - `crates/mds-core/src/ast.rs:10` (Confidence: 70%) -- The comment says "Matches MAX_NESTING_DEPTH" but `MAX_ELSEIF_BRANCHES=256` while `MAX_NESTING_DEPTH=64`. The prior resolution cycle lowered nesting depth from 256 to 64 but left `MAX_ELSEIF_BRANCHES` at 256. These are independent concerns (nesting depth is about stack frames; elseif branches are flat), so the values need not match, but the comment is misleading.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 8/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The reliability posture of this PR is strong. All new loops are bounded (the `@elseif` while-loop is bounded by both `MAX_ELSEIF_BRANCHES` and finite token stream; `findProjectRoot` is bounded by `MAX_TRAVERSAL_DEPTH` and filesystem root detection). Precondition assertions are present via `Condition::root()`. The nesting depth limit was correctly lowered from 256 to 64 (applies ADR-001 pre-merge quality principles) and the security test was adapted with an 8 MB stack to accommodate the old limit value. The `find_unquoted_operator` scanner properly handles escape sequences to prevent runaway string scanning. The two MEDIUM issues are correctness/ordering improvements, not blocking concerns.

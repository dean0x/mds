# Testing Review Report

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-20

## Issues in Your Changes (BLOCKING)

### HIGH

**Test E-5: Vacuous assertion on help property** - `crates/mds-napi/__test__/index.spec.mjs:251-262`
**Confidence**: 92%
- Problem: Test E-5 ("undefined var error has help property") only validates `help` when it is present (`if ('help' in err)`). If the property is absent, the test silently passes without asserting anything meaningful. This makes the test vacuous -- it cannot fail, so it provides zero confidence that `help` is actually surfaced for this error type. The Rust code at `lib.rs:153-155` does conditionally attach `help` via `serialized.help`, so whether it is present depends on the error variant. The test should either assert the property exists or clearly document that absence is acceptable and test both branches.
- Fix: Either assert that `help` is present (if the `undefined_var` error is documented to always provide help), or restructure as two tests -- one for an error that always has `help` and one that never does:
  ```js
  test('E-5: undefined var error has help property', () => {
    assert.throws(
      () => compile('Hello {undefined_var}!\n'),
      (err) => {
        // If help is expected for this error type, assert it exists:
        assert.ok('help' in err, 'undefined_var errors should include help');
        assert.ok(typeof err.help === 'string');
        assert.ok(err.help.length > 0, 'help should not be empty');
        return true;
      },
    );
  });
  ```

**Test E-8: Vacuous assertion on span property** - `crates/mds-napi/__test__/index.spec.mjs:284-296`
**Confidence**: 92%
- Problem: Same pattern as E-5. Test E-8 ("span is object when present") only validates `span` when `err.span !== undefined`. If span is not present, the test passes vacuously. The Rust code at `lib.rs:157-171` attaches span when `serialized.span` is `Some(...)`. For `undefined_var` errors, span data (offset, length, line, column) is likely populated since the parser knows where the undefined variable reference is. The test should either assert its presence or document why absence is acceptable.
- Fix: Assert span is present for `undefined_var` errors (which have a known location in source):
  ```js
  test('E-8: undefined var error has span with offset and length', () => {
    assert.throws(
      () => compile('Hello {undefined_var}!\n'),
      (err) => {
        assert.ok(err.span !== undefined, 'undefined_var errors should have span');
        assert.ok(typeof err.span === 'object' && err.span !== null);
        assert.ok(typeof err.span.offset === 'number');
        assert.ok(typeof err.span.length === 'number');
        return true;
      },
    );
  });
  ```

### MEDIUM

**Test F-CF6: Swallowed errors make test cwd-dependent and unreliable** - `crates/mds-napi/__test__/index.spec.mjs:123-138`
**Confidence**: 85%
- Problem: Test F-CF6 ("relative path resolves from cwd") catches `file_not_found` and `io` errors and silently returns, making the test pass regardless of whether relative path resolution actually works. The comment acknowledges this ("relative resolution depends on cwd at test time"), but a test that passes in all conditions verifies nothing. This is a flaky test anti-pattern -- it will never catch a regression in relative path resolution.
- Fix: Either control the cwd via `process.chdir()` in a before/after hook to make the test deterministic, or convert to a skip with diagnostic output rather than silently passing:
  ```js
  test('F-CF6: relative path resolves from cwd', () => {
    const relativePath = 'crates/mds-napi/__test__/fixtures/simple.mds';
    try {
      const result = compileFile(relativePath);
      assert.ok(result.output.includes('Hello Alice!'), `got: ${result.output}`);
    } catch (e) {
      if (e.code === 'mds::file_not_found' || e.code === 'mds::io') {
        // Use test context to skip instead of silently passing
        throw new Error(`SKIP: relative path resolution depends on cwd (current: ${process.cwd()})`);
      }
      throw e;
    }
  });
  ```

**Missing test: `checkFile` with vars** - `crates/mds-napi/__test__/index.spec.mjs`
**Confidence**: 85%
- Problem: The test suite covers `compileFile` with vars (F-CF2), `compile` with vars (F-C6), and `check` with vars (F-K5), but there is no test for `checkFile` with vars. The `checkFile` function at `lib.rs:512-524` accepts vars through `parse_file_opts`, so this is a valid code path that lacks coverage. The `var.mds` fixture uses `{name}` which would trigger `undefined_var` without vars -- `checkFile` should accept and apply them.
- Fix: Add a test:
  ```js
  test('F-K10: checkFile with vars', () => {
    const result = checkFile(VAR_MDS, { vars: { name: 'World' } });
    assert.ok(Array.isArray(result.warnings));
    assert.deepEqual(result.warnings, []);
  });
  ```

**Missing test: `check` and `checkFile` return shapes** - `crates/mds-napi/__test__/index.spec.mjs`
**Confidence**: 82%
- Problem: The `compile` tests (F-C1) verify the full return shape (`result.output`, `result.warnings`, `result.dependencies`), but the `check` tests (F-K1) only verify `result.warnings` is an array. There is no test asserting that `check`/`checkFile` results do NOT have an `output` property or `dependencies` property -- confirming the `CheckResult` shape (which only has `warnings` per `lib.rs:70-74`) is distinct from `CompileResult`.
- Fix: Add shape assertions:
  ```js
  test('F-K1a: check result has only warnings property', () => {
    const result = check('Hello World!\n');
    assert.ok(Array.isArray(result.warnings));
    assert.equal(result.output, undefined, 'check should not return output');
    assert.equal(result.dependencies, undefined, 'check should not return dependencies');
  });
  ```

**Missing test: `basePath` as non-string type** - `crates/mds-napi/__test__/index.spec.mjs`
**Confidence**: 80%
- Problem: The options validation tests cover `basePath: ''` (empty string, V-3) and `basePath` on file variants (V-4, V-5), but do not test `basePath` with a non-string type (e.g., `basePath: 42`). The Rust code at `lib.rs:330-338` has an explicit branch for non-string, non-null basePath values that throws `invalid_options`. This code path is untested.
- Fix: Add a test:
  ```js
  test('V-7: basePath as number throws mds::invalid_options', () => {
    assert.throws(
      () => compile('Hello!\n', { basePath: 42 }),
      (err) => {
        assert.equal(err.code, 'mds::invalid_options');
        return true;
      },
    );
  });
  ```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**Missing test: `vars` as array (non-object, non-string)** - `crates/mds-napi/src/lib.rs:288-294`
**Confidence**: 82%
- Problem: Test V-2 validates `vars: 'not-an-object'` (string), but the `parse_vars_field` function at lib.rs:288 rejects any non-object value. An array `[]` would also be rejected, but this path isn't tested. Arrays are a common JavaScript mistake (`vars: [...]` instead of `vars: {...}`) and would exercise the same error path but with a different `json_type_name` output ("array" vs "string").
- Fix: Add a quick test:
  ```js
  test('V-8: vars as array throws mds::invalid_options', () => {
    assert.throws(
      () => compile('Hello!\n', { vars: ['not', 'an', 'object'] }),
      (err) => {
        assert.equal(err.code, 'mds::invalid_options');
        assert.ok(err.message.includes('array'), `expected 'array' in: ${err.message}`);
        return true;
      },
    );
  });
  ```

**Missing test: `unknown key on check` and `unknown key on checkFile`** - `crates/mds-napi/src/lib.rs:344-351, 385-391`
**Confidence**: 80%
- Problem: Options validation tests V-1 and V-6 cover unknown keys for `compile` and `compileFile`, but there are no corresponding tests for `check` and `checkFile` with unknown keys. Both functions use the same options parsing (`parse_compile_opts` for `check`, `parse_file_opts` for `checkFile`), and both have the unknown-key rejection logic. Without tests, a regression in `check`/`checkFile` option validation would go undetected.
- Fix: Add tests for completeness:
  ```js
  test('V-9: unknown key on check throws mds::invalid_options', () => {
    assert.throws(
      () => check('Hello!\n', { unknownKey: true }),
      (err) => {
        assert.equal(err.code, 'mds::invalid_options');
        return true;
      },
    );
  });

  test('V-10: unknown key on checkFile throws mds::invalid_options', () => {
    assert.throws(
      () => checkFile(SIMPLE_MDS, { unknownKey: true }),
      (err) => {
        assert.equal(err.code, 'mds::invalid_options');
        return true;
      },
    );
  });
  ```

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Missing test: warnings array is populated for a template that triggers warnings** - `crates/mds-napi/__test__/index.spec.mjs` (Confidence: 70%) -- All compile/check tests assert warnings is an array or is empty, but no test verifies that warnings can actually contain entries. If a future change breaks warning propagation, no test would catch it. This requires finding or creating a fixture that triggers a warning.

- **Missing test: span.line and span.column presence** - `crates/mds-napi/__test__/index.spec.mjs:284` (Confidence: 65%) -- The Rust code at lib.rs:162-167 conditionally attaches `line` and `column` to span objects. No test verifies these optional fields are present and correct for errors that have location data. This would increase confidence in the full span serialization path.

- **Resource limit test R-3 allocates 10 MiB** - `crates/mds-napi/__test__/index.spec.mjs:400-411` (Confidence: 65%) -- Tests R-1, R-2, and R-3 each allocate 10+ MiB strings. R-3 allocates exactly 10 MiB and then compiles it, which is slow. In CI, this may cause memory pressure or timeouts. Consider adding a test timeout or documenting the expected resource usage.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | - | 2 | 4 | - |
| Should Fix | - | - | 2 | - |
| Pre-existing | - | - | - | - |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

### Rationale

The test suite is well-structured with 46 tests across 7 describe blocks, good use of `node:test` runner, clear naming conventions (F-C/F-CF/F-K/E/V/R/P prefixes), and solid coverage of the happy-path API surface. The Arrange-Act-Assert pattern is followed consistently, and fixtures are well-organized.

The two HIGH issues (vacuous assertions in E-5 and E-8) are the primary concern -- these tests will never fail regardless of whether the code works correctly, which violates the "tests validate behavior" principle. The MEDIUM issues are missing coverage for valid code paths that have explicit Rust implementation but no corresponding JavaScript tests.

None of these issues indicate broken functionality, but the vacuous assertions should be fixed to provide actual test value, and the coverage gaps should be addressed to match the documented API surface.

# Testing Review Report

**Branch**: refactor/12-23-resolve-source-api-surface -> main
**Date**: 2026-05-26T12:07

## Issues in Your Changes (BLOCKING)

### HIGH

**Missing error-path test for non-UTF-8 path rejection in Rust public API** - `crates/mds-core/tests/api_surface.rs:659-682`
**Confidence**: 85%
- Problem: The core refactoring in `lib.rs` adds explicit UTF-8 validation at 5 call sites (`check`, `compile_collecting_warnings`, `check_collecting_warnings`, `compile_with_deps`, and `resolve_base_dir`). Each converts `Path -> &str` via `.to_str()` and returns `MdsError::io("path is not valid UTF-8")` on failure. The two new API surface tests (`module_cache_resolve_path_accepts_str` and `module_cache_resolve_source_accepts_str`) only verify that the new `&str` signature compiles and that happy-path usage works. No test verifies that the UTF-8 rejection path actually produces the expected error. This is a new error branch introduced by this PR and it has zero test coverage.
- Fix: Add at least one test that constructs a non-UTF-8 `OsStr` path (e.g., on Unix via `std::os::unix::ffi::OsStrExt`) and asserts that `mds::check()` or `mds::compile()` returns an `MdsError::Io` with a message containing "not valid UTF-8". Example:
```rust
#[cfg(unix)]
#[test]
fn non_utf8_path_returns_io_error() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let bad_path = OsStr::from_bytes(&[0xff, 0xfe]);
    let result = mds::check(std::path::Path::new(bad_path), None);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("not valid UTF-8"), "got: {msg}");
}
```

### MEDIUM

**`_setTransformerForTesting` fire-and-forget `void lazy.get()` not tested for pre-resolution guarantee** - `packages/webpack-loader/src/index.ts:77-79`
**Confidence**: 82%
- Problem: The refactored `_setTransformerForTesting` creates a `LazyInit` wrapping the transformer and immediately fires `void lazy.get()` to pre-resolve it. The `void` discard means the promise is not awaited. While the factory `async () => t` resolves synchronously (microtask), the test in `loader.spec.mjs` line 133 calls `_setTransformerForTesting(mockTransformer)` and then immediately calls `mdsLoader.call(ctx)`. This works because the microtask resolves before the `await` in `mdsLoader`, but the behavior relies on microtask ordering rather than an explicit contract. The existing `loader.spec.mjs` tests do not directly test `_setTransformerForTesting` for this synchronous-resolution guarantee -- they test it only as a setup step for other tests.
- Fix: Add a targeted test that asserts `_setTransformerForTesting` makes the transformer immediately available (i.e., the very next synchronous `getLazy().get()` resolves without re-invoking the factory):
```javascript
test('_setTransformerForTesting pre-resolves the lazy singleton', async () => {
    let factoryCallCount = 0;
    const mockTransformer = {
        shouldTransform() { return true; },
        async transform() { return { code: 'ok', warnings: [], dependencies: [] }; },
    };
    _setTransformerForTesting(mockTransformer);
    // Immediately use via the loader -- should not trigger dynamic import('@mds/mds')
    const ctx = createLoaderContext(SIMPLE_MDS);
    await mdsLoader.call(ctx);
    assert.equal(ctx.callbackResult.err, null);
});
```

## Issues in Code You Touched (Should Fix)

### MEDIUM

**LazyInit `reset()` during in-flight `get()` has no test coverage** - `packages/bundler-utils/src/lazy-init.ts:34-38`, `packages/bundler-utils/__test__/lazy-init.spec.mjs`
**Confidence**: 83%
- Problem: The `LazyInit` test suite covers: single get, concurrent get, rejection+retry, success idempotence, reset-then-get, void factory, and null factory. However, there is no test for calling `reset()` while a `get()` is still in-flight (i.e., the factory promise has not yet resolved). The `reset()` method sets `pending = null`, `resolved = false`, `instance = undefined` -- but an in-flight promise's `.then()` handler will still run and set `resolved = true` and `instance = result` after the reset. This means `reset()` during in-flight is silently broken: the old factory completion will corrupt the state, and a subsequent `get()` will return the stale value instead of re-invoking the factory.
- Fix: Either (a) add a test that documents this as a known limitation (reset must not be called while get is in-flight), or (b) fix the implementation to guard against post-reset completion (e.g., via a generation counter) and add a test for the correct behavior. At minimum, document the constraint:
```javascript
test('reset() during in-flight get — documents known limitation', async () => {
    let resolveFactory;
    const lazy = new LazyInit(() => new Promise(r => { resolveFactory = r; }));
    const p1 = lazy.get(); // starts factory
    lazy.reset();          // clears pending
    resolveFactory('stale');
    // p1 resolves with 'stale' AND sets resolved=true, corrupting state
    const v1 = await p1;
    assert.equal(v1, 'stale');
    // Ideally the next get() should re-invoke factory, but due to the
    // race it returns the stale value. Document this as a known limitation.
});
```

## Pre-existing Issues (Not Blocking)

### MEDIUM

**Rust `api_surface.rs` tests assert existence but not behavior for most public functions** - `crates/mds-core/tests/api_surface.rs:10-28`
**Confidence**: 80%
- Problem: The `public_functions_exist` test calls 16+ public functions with `let _ = ...`, discarding every result. These are compile-time checks that the API surface exists, but they provide zero behavioral verification. The function calls will succeed or fail at runtime, but errors are silently discarded. While compile-time API surface checks have value for regression prevention, mixing them with runtime test infrastructure gives a false sense of coverage. The newer tests in the file (e.g., `compile_virtual_exists`, `compile_with_deps_native_fs_integration`) correctly assert behavior.
- Fix: This is a pre-existing pattern; no action required for this PR. In a future cleanup, consider splitting compile-time signature checks (using `const _: fn(...) -> ... = mds::...` patterns as in `cli_import_pattern_works`) from runtime behavioral tests.

## Suggestions (Lower Confidence)

- **Concurrent reset+get race in webpack-loader** - `packages/webpack-loader/src/index.ts:59-64` (Confidence: 65%) -- `_resetForTesting` calls `lazy?.reset()` then `lazy = null`. If a concurrent `mdsLoader` call is in-flight, the reset of the LazyInit's internal state could interact with the in-flight promise. Low risk since this is test-only code gated by `NODE_ENV=test`.

- **No test for `resolve_base_dir` cwd UTF-8 failure** - `crates/mds-core/src/lib.rs:220-226` (Confidence: 62%) -- The `None` branch of `resolve_base_dir` now validates that the current working directory is valid UTF-8. This is nearly impossible to trigger on modern systems (cwd is almost always UTF-8), but the error path exists and is untested. Testing would require OS-level manipulation to set a non-UTF-8 cwd, which is impractical.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 1 | 0 |
| Should Fix | 0 | 0 | 1 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Testing Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The new `LazyInit` test suite is well-structured and covers the core behavioral contract (singleton semantics, concurrency dedup, rejection retry, void/null edge cases, reset). The existing `transform.spec.mjs` and `loader.spec.mjs` implicitly exercise the refactored code paths through integration. However, the primary motivation for this PR -- switching `resolve_path`/`resolve_source` from `&Path` to `&str` with explicit UTF-8 error handling -- introduces new error branches in 5 Rust functions that have no dedicated test coverage for the failure case. Adding at least one test for the non-UTF-8 rejection path would close this gap. The `LazyInit.reset()` during in-flight race should either be tested or documented as a known limitation.

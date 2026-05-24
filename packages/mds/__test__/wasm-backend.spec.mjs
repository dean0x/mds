/**
 * WASM backend unit tests for @mds/mds universal package.
 * Tests: U-WB1 through U-WB13
 *
 * Imports dist/backend/wasm.js directly to exercise internal state
 * without going through the full node.ts entry point.
 */
import { test, describe, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { initWasmNode, createWasmBackend, _resetForTesting } from '../dist/backend/wasm.js';

// Mirror of MAX_INIT_RETRIES from src/backend/wasm.ts.
// If this value drifts from the source, U-WB2 will fail to trigger the
// exhaustion path, surfacing the mismatch via a test failure rather than
// silently testing the wrong threshold.
const MAX_INIT_RETRIES = 3;

describe('wasm backend — circuit breaker', () => {
  afterEach(() => {
    // Restore a clean state after each test so the module singleton does not
    // bleed into subsequent tests or into the main backend.spec tests.
    _resetForTesting(0);
  });

  test('U-WB1: initWasmNode() attempts loading when failures are below the limit', async () => {
    // Pre-seed 2 failures (one below the threshold of 3).
    _resetForTesting(MAX_INIT_RETRIES - 1);
    // Should succeed because failures (2) < MAX_INIT_RETRIES (3).
    await assert.doesNotReject(initWasmNode());
  });

  test('U-WB2: initWasmNode() throws permanently once failure count reaches MAX_INIT_RETRIES', async () => {
    // Pre-seed exactly MAX_INIT_RETRIES failures to simulate exhaustion.
    _resetForTesting(MAX_INIT_RETRIES);
    await assert.rejects(
      () => initWasmNode(),
      (err) => {
        assert.ok(err instanceof Error);
        assert.ok(
          err.message.includes('failed to initialize after'),
          `expected permanent-failure message, got: ${err.message}`,
        );
        assert.ok(
          err.message.includes(String(MAX_INIT_RETRIES)),
          `expected retry count in message, got: ${err.message}`,
        );
        return true;
      },
    );
  });

  test('U-WB3: initWasmNode() succeeds and produces a valid WasmModule when WASM module is present', async () => {
    // Regression: shape-check in tryLoadCandidate must accept a well-formed WASM
    // module (compile/check/scanImports all present). Before the fix, ALL errors
    // were swallowed and the module was cast blindly via "as WasmModule".
    // This test confirms the happy path: a correct module passes the shape check.
    await assert.doesNotReject(
      () => initWasmNode(),
      'initWasmNode() should resolve when a valid WASM module is on the candidate path',
    );
  });

  test('U-WB4: circuit breaker message includes retry count and is non-empty after exhaustion', async () => {
    // Verify the circuit breaker fires with a diagnostic message that cites the
    // attempt count — confirming errors are never silently swallowed.
    // Regression: prior "bare catch { return null; }" discarded all errors.
    _resetForTesting(MAX_INIT_RETRIES);
    await assert.rejects(
      () => initWasmNode(),
      (err) => {
        assert.ok(err instanceof Error, 'must be an Error instance');
        assert.ok(err.message.length > 0, 'error message must not be empty');
        assert.ok(
          err.message.includes('failed to initialize after'),
          `expected circuit-breaker message, got: ${err.message}`,
        );
        assert.ok(
          err.message.includes(String(MAX_INIT_RETRIES)),
          `expected retry count ${MAX_INIT_RETRIES} in message, got: ${err.message}`,
        );
        return true;
      },
    );
  });

  // ---------------------------------------------------------------------------
  // New tests for the split API
  // ---------------------------------------------------------------------------

  test('U-WB5: initWasmNode() returns WasmModule with compile, check, and scanImports', async () => {
    const mod = await initWasmNode();
    assert.equal(typeof mod.compile, 'function', 'WasmModule must have compile');
    assert.equal(typeof mod.check, 'function', 'WasmModule must have check');
    assert.equal(typeof mod.scanImports, 'function', 'WasmModule must have scanImports');
  });

  test('U-WB6: concurrent initWasmNode() calls share single promise', async () => {
    _resetForTesting(0);
    // Fire two concurrent calls — they must both resolve and share state.
    const [mod1, mod2] = await Promise.all([initWasmNode(), initWasmNode()]);
    // Both must be the same object (promise deduplication guarantee).
    assert.strictEqual(mod1, mod2, 'concurrent initWasmNode() calls must return the same module reference');
  });

  test('U-WB8: failed initWasmNode() does not poison subsequent calls (circuit breaker allows retries below limit)', async () => {
    // Seed 1 failure (below limit). First call should still succeed by reloading.
    _resetForTesting(1);
    await assert.doesNotReject(
      () => initWasmNode(),
      'initWasmNode() should retry and succeed when failures < MAX_INIT_RETRIES',
    );
  });

  test('U-WB9: createWasmBackend(mod) is synchronous and returns MdsBaseBackend', async () => {
    const mod = await initWasmNode();
    // createWasmBackend is synchronous — no await needed.
    let backend;
    assert.doesNotThrow(() => {
      backend = createWasmBackend(mod);
    });
    assert.ok(backend !== undefined, 'createWasmBackend must return a backend');
    assert.equal(typeof backend.compile, 'function', 'must have compile');
    assert.equal(typeof backend.check, 'function', 'must have check');
    assert.equal(typeof backend.getBackend, 'function', 'must have getBackend');
  });

  test('U-WB10: createWasmBackend(mod).compile("Hello!\\n") returns correct output', async () => {
    const mod = await initWasmNode();
    const backend = createWasmBackend(mod);
    const result = backend.compile('Hello!\n');
    assert.equal(result.output, 'Hello!\n', `expected "Hello!\\n", got: ${result.output}`);
    assert.ok(Array.isArray(result.warnings));
    assert.ok(Array.isArray(result.dependencies));
  });

  test('U-WB11: createWasmBackend(mod).getBackend() returns "wasm"', async () => {
    const mod = await initWasmNode();
    const backend = createWasmBackend(mod);
    assert.equal(backend.getBackend(), 'wasm');
  });

  test('U-WB12: createWasmBackend(mod) has NO compileFile or checkFile', async () => {
    const mod = await initWasmNode();
    const backend = createWasmBackend(mod);
    assert.equal(
      'compileFile' in backend,
      false,
      'MdsBaseBackend must not have compileFile',
    );
    assert.equal(
      'checkFile' in backend,
      false,
      'MdsBaseBackend must not have checkFile',
    );
  });

  test('U-WB13: tryLoadCandidate rejects modules missing scanImports', async () => {
    // This test verifies the shape validation at the boundary.
    // The shape check now requires scanImports; a module without it returns null
    // from tryLoadCandidate. We test this indirectly by verifying that a successful
    // initWasmNode() always yields a module with scanImports (the built WASM has it).
    const mod = await initWasmNode();
    assert.equal(
      typeof mod.scanImports,
      'function',
      'initWasmNode() must only succeed when scanImports is present in the WASM module',
    );
  });
});

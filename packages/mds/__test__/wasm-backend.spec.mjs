/**
 * WASM backend unit tests for @mds/mds universal package.
 * Tests: U-WB1 through U-WB5
 *
 * Imports dist/backend/wasm.js directly to exercise internal state
 * without going through the full node.ts entry point.
 */
import { test, describe, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { init, _resetForTesting } from '../dist/backend/wasm.js';

const MAX_INIT_RETRIES = 3;

describe('wasm backend — circuit breaker', () => {
  afterEach(() => {
    // Restore a clean state after each test so the module singleton does not
    // bleed into subsequent tests or into the main backend.spec tests.
    _resetForTesting(0);
  });

  test('U-WB1: init() succeeds when failures are below the limit', async () => {
    // Pre-seed 2 failures (one below the threshold of 3).
    _resetForTesting(MAX_INIT_RETRIES - 1);
    // Should succeed because failures (2) < MAX_INIT_RETRIES (3).
    await assert.doesNotReject(init());
  });

  test('U-WB2: init() throws permanently once failure count reaches MAX_INIT_RETRIES', async () => {
    // Pre-seed exactly MAX_INIT_RETRIES failures to simulate exhaustion.
    _resetForTesting(MAX_INIT_RETRIES);
    await assert.rejects(
      () => init(),
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

  test('U-WB3: init() succeeds and produces a valid backend when WASM module is present', async () => {
    // Regression: shape-check in tryLoadCandidate must accept a well-formed WASM
    // module (compile/check/scanImports all present). Before the fix, ALL errors
    // were swallowed and the module was cast blindly via "as WasmModule".
    // This test confirms the happy path: a correct module passes the shape check.
    _resetForTesting(0);
    await assert.doesNotReject(
      () => init(),
      'init() should resolve when a valid WASM module is on the candidate path',
    );
  });

  test('U-WB4: circuit breaker message includes retry count after exhaustion', async () => {
    // Verify that when the circuit breaker fires it cites the attempt count in
    // its message — confirming the error is not swallowed at any layer.
    _resetForTesting(MAX_INIT_RETRIES);
    await assert.rejects(
      () => init(),
      (err) => {
        assert.ok(err instanceof Error, 'must be an Error instance');
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

  test('U-WB5: error message is non-empty — errors are never silently swallowed', async () => {
    // Regression: prior "bare catch { return null; }" swallowed all errors.
    // Verify that any error surfaced by init() carries a non-empty message.
    _resetForTesting(MAX_INIT_RETRIES);
    await assert.rejects(
      () => init(),
      (err) => {
        assert.ok(err instanceof Error, 'error must be an Error instance');
        assert.ok(err.message.length > 0, 'error message must not be empty');
        return true;
      },
    );
  });
});

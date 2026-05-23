/**
 * WASM backend unit tests for @mds/mds universal package.
 * Tests: U-WB1 through U-WB2
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
});

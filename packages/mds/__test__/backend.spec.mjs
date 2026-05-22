/**
 * Backend selection tests for @mds/mds universal package.
 * Tests: U-B1 through U-B5
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { getBackend, compile } from '../dist/node.js';

describe('backend', () => {
  test('U-B1: getBackend() returns "native" or "wasm"', () => {
    const backend = getBackend();
    assert.ok(
      backend === 'native' || backend === 'wasm',
      `expected "native" or "wasm", got: ${backend}`,
    );
  });

  test('U-B2: getBackend() returns string', () => {
    assert.ok(typeof getBackend() === 'string');
  });

  test('U-B3: native backend is selected by default when napi is available', () => {
    // The default (no MDS_BACKEND env var) should prefer native.
    // Since we're running tests with the .node file available, backend should be native.
    const backend = getBackend();
    assert.equal(backend, 'native', `expected native backend, got: ${backend}`);
  });

  test('U-B4: compile works regardless of backend', () => {
    // This test validates that compile() works with whatever backend was selected.
    const result = compile('Hello World!\n');
    assert.equal(result.output, 'Hello World!\n');
  });

  test('U-B5: compile result shape is consistent across backends', () => {
    const result = compile('Hello World!\n');
    assert.ok(typeof result.output === 'string', 'output must be string');
    assert.ok(Array.isArray(result.warnings), 'warnings must be array');
    assert.ok(Array.isArray(result.dependencies), 'dependencies must be array');
  });
});

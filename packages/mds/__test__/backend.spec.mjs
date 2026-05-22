/**
 * Backend selection tests for @mds/mds universal package.
 * Tests: U-B1 through U-B5
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { getBackend, compile } from '../dist/node.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

describe('backend', () => {
  test('U-B1: getBackend() returns "native" or "wasm"', () => {
    const backend = getBackend();
    assert.ok(
      backend === 'native' || backend === 'wasm',
      `expected "native" or "wasm", got: ${backend}`,
    );
  });

  test('U-B2: native backend is selected by default when napi is available', () => {
    // The default (no MDS_BACKEND env var) should prefer native.
    // Since we're running tests with the .node file available, backend should be native.
    const backend = getBackend();
    assert.equal(backend, 'native', `expected native backend, got: ${backend}`);
  });

  test('U-B3: compile works regardless of backend', () => {
    // This test validates that compile() works with whatever backend was selected.
    const result = compile('Hello World!\n');
    assert.equal(result.output, 'Hello World!\n');
  });

  test('U-B4: compile result shape is consistent across backends', () => {
    const result = compile('Hello World!\n');
    assert.ok(typeof result.output === 'string', 'output must be string');
    assert.ok(Array.isArray(result.warnings), 'warnings must be array');
    assert.ok(Array.isArray(result.dependencies), 'dependencies must be array');
  });

  test('U-B5: MDS_BACKEND=wasm forces WASM backend', () => {
    // Spawn a subprocess with MDS_BACKEND=wasm to test backend selection
    // without affecting the current process's already-resolved backend.
    const script = path.join(__dirname, 'backend-wasm-helper.mjs');
    const output = execFileSync(process.execPath, ['--input-type=module'], {
      input: `import { getBackend } from '../dist/node.js';\nconsole.log(getBackend());\n`,
      cwd: __dirname,
      env: { ...process.env, MDS_BACKEND: 'wasm' },
      encoding: 'utf8',
    });
    assert.equal(output.trim(), 'wasm', `expected WASM backend when MDS_BACKEND=wasm, got: ${output.trim()}`);
  });
});

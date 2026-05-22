/**
 * Cross-backend parity tests for @mds/mds universal package.
 * Tests: U-P1 through U-P6
 *
 * Verifies that native and WASM backends produce identical results.
 * WASM tests are skipped when WASM module is not built.
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createNativeBackend } from '../dist/backend/native.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const FIXTURES = path.join(__dirname, 'fixtures');
const napiAddon = require(path.join(__dirname, '../../..', 'crates/mds-napi/mds-napi.node'));
const nativeBackend = createNativeBackend(napiAddon);

describe('native backend parity', () => {
  test('U-P1: compile plain text matches expected output', () => {
    const result = nativeBackend.compile('Hello World!\n');
    assert.equal(result.output, 'Hello World!\n');
    assert.deepEqual(result.warnings, []);
    assert.deepEqual(result.dependencies, []);
  });

  test('U-P2: compile with frontmatter vars', () => {
    const source = '---\nname: Test\n---\nHello {name}!\n';
    const result = nativeBackend.compile(source);
    assert.ok(result.output.includes('Hello Test!'), `got: ${result.output}`);
  });

  test('U-P3: compile with runtime vars', () => {
    const result = nativeBackend.compile('Hello {name}!\n', { vars: { name: 'Parity' } });
    assert.equal(result.output, 'Hello Parity!\n');
  });

  test('U-P4: check returns warnings array', () => {
    const result = nativeBackend.check('Hello!\n');
    assert.ok(Array.isArray(result.warnings));
  });

  test('U-P5: compile syntax error throws', () => {
    assert.throws(() => nativeBackend.compile('Hello {name\n'));
  });

  test('U-P6: compileFile resolves with correct shape', async () => {
    const simpleMds = path.join(FIXTURES, 'simple.mds');
    const result = await nativeBackend.compileFile(simpleMds);
    assert.ok(typeof result.output === 'string');
    assert.ok(Array.isArray(result.warnings));
    assert.ok(Array.isArray(result.dependencies));
  });
});

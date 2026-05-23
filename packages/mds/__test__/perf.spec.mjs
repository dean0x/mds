/**
 * Performance benchmarks for @mds/mds universal package.
 * Tests: U-PF1 through U-PF5
 *
 * These are lightweight benchmarks that verify the API can handle
 * repeated calls without degradation (not strict timing assertions).
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { compile, compileFile } from '../dist/node.js';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.join(__dirname, 'fixtures');
const SIMPLE_MDS = path.join(FIXTURES, 'simple.mds');

describe('performance', () => {
  test('U-PF1: compile 100 times completes in reasonable time', () => {
    const start = Date.now();
    for (let i = 0; i < 100; i++) {
      compile('Hello World!\n');
    }
    const elapsed = Date.now() - start;
    // Allow 2 seconds for 100 compiles — very generous threshold.
    assert.ok(elapsed < 2000, `100 compiles took ${elapsed}ms (expected < 2000ms)`);
  });

  test('U-PF2: compile with vars 100 times completes quickly', () => {
    const start = Date.now();
    for (let i = 0; i < 100; i++) {
      compile('Hello {name}!\n', { vars: { name: `World${i}` } });
    }
    const elapsed = Date.now() - start;
    assert.ok(elapsed < 2000, `100 compile-with-vars took ${elapsed}ms`);
  });

  test('U-PF3: compile result has expected structure every time', () => {
    for (let i = 0; i < 10; i++) {
      const result = compile(`Hello ${i}!\n`);
      assert.ok(typeof result.output === 'string');
      assert.ok(Array.isArray(result.warnings));
      assert.ok(Array.isArray(result.dependencies));
    }
  });

  test('U-PF4: compileFile 10 times completes in reasonable time', async () => {
    const start = Date.now();
    const promises = Array.from({ length: 10 }, () => compileFile(SIMPLE_MDS));
    await Promise.all(promises);
    const elapsed = Date.now() - start;
    assert.ok(elapsed < 3000, `10 concurrent compileFile took ${elapsed}ms`);
  });

  test('U-PF5: large source string compiles without memory issues', () => {
    // Build a source with many lines but well within limits.
    const lines = Array.from({ length: 1000 }, (_, i) => `Line ${i}: Hello World!`).join('\n');
    const source = `${lines}\n`;
    const result = compile(source);
    assert.ok(result.output.includes('Line 0:'));
    assert.ok(result.output.includes('Line 999:'));
  });
});

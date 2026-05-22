/**
 * Core compile() tests for @mds/mds universal package.
 * Tests: U-C1 through U-C6
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { SIMPLE_MDS, FIXTURES } from './helpers.mjs';
import { compile, isMdsError } from '../dist/node.js';

describe('compile', () => {
  test('U-C1: compile plain text with no options', () => {
    const result = compile('Hello World!\n');
    assert.equal(result.output, 'Hello World!\n');
    assert.ok(Array.isArray(result.warnings), 'warnings should be array');
    assert.ok(Array.isArray(result.dependencies), 'dependencies should be array');
    assert.equal(result.warnings.length, 0);
    assert.equal(result.dependencies.length, 0);
  });

  test('U-C2: compile with frontmatter variables', () => {
    const source = '---\nname: Alice\n---\nHello {name}!\n';
    const result = compile(source);
    assert.ok(result.output.includes('Hello Alice!'), `expected "Hello Alice!" in: ${result.output}`);
  });

  test('U-C3: compile with runtime vars', () => {
    const source = 'Hello {name}!\n';
    const result = compile(source, { vars: { name: 'World' } });
    assert.equal(result.output, 'Hello World!\n');
  });

  test('U-C4: compile returns warnings for empty @include', () => {
    // Empty include does not fail, but emits a warning.
    const source = '---\nname: Test\n---\nHello!\n';
    const result = compile(source);
    assert.equal(typeof result.output, 'string');
    assert.ok(Array.isArray(result.warnings));
  });

  test('U-C5: compile syntax error throws MdsError with code', () => {
    assert.throws(
      () => compile('Hello {name\n'),
      (err) => {
        assert.ok(isMdsError(err), `expected MdsError, got: ${err}`);
        assert.ok(typeof err.code === 'string', 'code should be string');
        return true;
      },
    );
  });

  test('U-C6: compile returns empty dependencies when no imports', () => {
    const result = compile('Hello World!\n');
    assert.deepEqual(result.dependencies, []);
  });
});

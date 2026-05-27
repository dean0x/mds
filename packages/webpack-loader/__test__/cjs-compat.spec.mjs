/**
 * CJS compatibility tests for @mds/webpack-loader.
 *
 * Verifies that the CJS build (dist-cjs/) can be loaded via require() and
 * exports the default loader function. This is the primary condition for
 * Webpack 5 interoperability — Webpack resolves loaders using require().
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

describe('webpack-loader CJS build', () => {
  test('loads without error via require()', () => {
    const cjsPath = resolve(__dirname, '../dist-cjs/index.js');
    const cjsBuild = require(cjsPath);
    assert.ok(cjsBuild, 'CJS build should load successfully');
  });

  test('exports default as an async function (the loader)', () => {
    const { default: mdsLoader } = require(resolve(__dirname, '../dist-cjs/index.js'));
    assert.equal(typeof mdsLoader, 'function', 'default export should be a function');
    // Webpack loaders must return a Promise. Verify the behavioral contract by
    // invoking the loader with a minimal mock context that satisfies its
    // interface: async() returns a no-op callback, getOptions() returns {}.
    // We only check the return type — we do not assert on side effects.
    let capturedCallback = null;
    const mockContext = {
      resourcePath: '/dev/null/nonexistent.mds',
      async() { return (err, _content) => { capturedCallback = err; }; },
      addDependency() {},
      emitWarning() {},
      getOptions() { return {}; },
    };
    const result = mdsLoader.call(mockContext);
    assert.ok(
      result instanceof Promise,
      'default export should return a Promise when called (async function)',
    );
    // Drain the promise so the test runner does not report an unhandled rejection.
    return result.catch(() => {});
  });

  test('exports _resetForTesting helper', () => {
    const { _resetForTesting } = require(resolve(__dirname, '../dist-cjs/index.js'));
    assert.equal(typeof _resetForTesting, 'function', '_resetForTesting should be a function');
  });

  test('exports _setTransformerForTesting helper', () => {
    const { _setTransformerForTesting } = require(resolve(__dirname, '../dist-cjs/index.js'));
    assert.equal(typeof _setTransformerForTesting, 'function', '_setTransformerForTesting should be a function');
  });

});

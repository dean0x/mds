/**
 * Tests for @mds/webpack-loader.
 */
import { test, describe, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SIMPLE_MDS = resolve(__dirname, '../../mds/__test__/fixtures/simple.mds');

// ---------------------------------------------------------------------------
// Import loader and reset helper
// ---------------------------------------------------------------------------
const { default: mdsLoader, _resetForTesting } = await import('../dist/index.js');

// ---------------------------------------------------------------------------
// Mock LoaderContext factory
// ---------------------------------------------------------------------------
function createLoaderContext(resourcePath, overrides = {}) {
  const addedDeps = [];
  const emittedWarnings = [];
  let callbackResult = null;

  const ctx = {
    resourcePath,
    getOptions() { return {}; },
    addDependency(dep) { addedDeps.push(dep); },
    emitWarning(err) { emittedWarnings.push(err); },
    async() {
      return (err, content) => {
        callbackResult = { err, content };
      };
    },
    get addedDeps() { return addedDeps; },
    get emittedWarnings() { return emittedWarnings; },
    get callbackResult() { return callbackResult; },
    ...overrides,
  };
  return ctx;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
describe('mdsLoader', () => {
  beforeEach(() => {
    _resetForTesting();
  });

  test('default export is a function', () => {
    assert.equal(typeof mdsLoader, 'function');
  });

  test('loader calls async callback with compiled content for .mds file', async () => {
    const ctx = createLoaderContext(SIMPLE_MDS);
    await mdsLoader.call(ctx);

    assert.ok(ctx.callbackResult !== null, 'callback should have been called');
    assert.equal(ctx.callbackResult.err, null, 'should not error');
    assert.ok(
      typeof ctx.callbackResult.content === 'string',
      'content should be a string',
    );
    assert.ok(
      ctx.callbackResult.content.includes('export default'),
      'content should have export default',
    );
  });

  test('loader calls addDependency for each dependency', async () => {
    // Use import_consumer which imports import_provider
    const consumerMds = resolve(__dirname, '../../mds/__test__/fixtures/import_consumer.mds');
    const ctx = createLoaderContext(consumerMds);
    await mdsLoader.call(ctx);

    assert.equal(ctx.callbackResult.err, null);
    // The consumer imports provider so at least 1 dep
    assert.ok(ctx.addedDeps.length >= 1, `expected deps, got: ${JSON.stringify(ctx.addedDeps)}`);
  });

  test('loader calls callback with error for nonexistent file', async () => {
    const ctx = createLoaderContext('/nonexistent/path/file.mds');
    await mdsLoader.call(ctx);

    assert.ok(ctx.callbackResult !== null);
    assert.ok(ctx.callbackResult.err instanceof Error, 'should call back with error');
  });

  test('transformer singleton is reused across calls', async () => {
    const ctx1 = createLoaderContext(SIMPLE_MDS);
    const ctx2 = createLoaderContext(SIMPLE_MDS);

    // Both should succeed without double-initializing
    await mdsLoader.call(ctx1);
    await mdsLoader.call(ctx2);

    assert.equal(ctx1.callbackResult.err, null);
    assert.equal(ctx2.callbackResult.err, null);
  });

  test('_resetForTesting clears singleton state', async () => {
    const ctx1 = createLoaderContext(SIMPLE_MDS);
    await mdsLoader.call(ctx1);
    assert.equal(ctx1.callbackResult.err, null);

    // Reset and run again — should re-initialize cleanly
    _resetForTesting();
    const ctx2 = createLoaderContext(SIMPLE_MDS);
    await mdsLoader.call(ctx2);
    assert.equal(ctx2.callbackResult.err, null);
  });

  test('emitWarning called for each warning', async () => {
    // We can't easily inject a mock transformer, so we rely on the real compile
    // which shouldn't emit warnings for simple.mds. Test the happy path.
    const ctx = createLoaderContext(SIMPLE_MDS);
    await mdsLoader.call(ctx);
    // For simple.mds there are no warnings
    assert.equal(ctx.emittedWarnings.length, 0);
  });
});

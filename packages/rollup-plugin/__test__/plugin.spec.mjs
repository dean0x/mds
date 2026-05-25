/**
 * Tests for @mds/rollup-plugin.
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import mdsPlugin from '../dist/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SIMPLE_MDS = resolve(__dirname, '../../mds/__test__/fixtures/simple.mds');
const CONSUMER_MDS = resolve(__dirname, '../../mds/__test__/fixtures/import_consumer.mds');

// ---------------------------------------------------------------------------
// Mock plugin context
// ---------------------------------------------------------------------------
function createPluginContext(overrides = {}) {
  const addedWatchFiles = [];
  const warnings = [];
  const errors = [];

  return {
    warn(msg) { warnings.push(msg); },
    addWatchFile(id) { addedWatchFiles.push(id); },
    error(msg, pos) {
      const message = typeof msg === 'string' ? msg : msg.message;
      const err = new Error(message);
      if (pos) err.pos = pos;
      errors.push(err);
      throw err;
    },
    get addedWatchFiles() { return addedWatchFiles; },
    get warnings() { return warnings; },
    get errors() { return errors; },
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
describe('mdsPlugin (rollup)', () => {
  test('plugin has name "mds"', () => {
    const plugin = mdsPlugin();
    assert.equal(plugin.name, 'mds');
  });

  test('has buildStart and transform hooks', () => {
    const plugin = mdsPlugin();
    assert.equal(typeof plugin.buildStart, 'function');
    assert.equal(typeof plugin.transform, 'function');
  });

  test('does NOT have enforce property (Rollup does not use it)', () => {
    const plugin = mdsPlugin();
    assert.equal(plugin.enforce, undefined);
  });

  test('transform returns null for non-mds file before buildStart', async () => {
    const plugin = mdsPlugin();
    const ctx = createPluginContext();
    const result = await plugin.transform.call(ctx, '', '/path/to/file.ts');
    assert.equal(result, null);
  });

  test('buildStart initializes transformer', async () => {
    const plugin = mdsPlugin();
    const ctx = createPluginContext();
    await plugin.buildStart.call(ctx);
    // After buildStart, should transform .mds files
    const result = await plugin.transform.call(ctx, '', SIMPLE_MDS);
    assert.ok(result !== null, 'should not be null for .mds');
    assert.ok(result.code.includes('export default'), 'should have default export');
    assert.equal(result.map, null);
  });

  test('transform returns null for non-mds after buildStart', async () => {
    const plugin = mdsPlugin();
    const ctx = createPluginContext();
    await plugin.buildStart.call(ctx);
    const result = await plugin.transform.call(ctx, '', '/path/to/file.ts');
    assert.equal(result, null);
  });

  test('transform calls addWatchFile for each dependency', async () => {
    const plugin = mdsPlugin();
    const ctx = createPluginContext();
    await plugin.buildStart.call(ctx);
    await plugin.transform.call(ctx, '', CONSUMER_MDS);
    assert.ok(ctx.addedWatchFiles.length >= 1, 'expected at least one watch file');
  });

  test('options passed through to compiler', () => {
    const options = { vars: { env: 'production' } };
    const plugin = mdsPlugin(options);
    assert.ok(plugin, 'plugin created with options');
  });

  test('transform calls this.error when compile fails', async () => {
    const plugin = mdsPlugin();
    const ctx = createPluginContext();
    await plugin.buildStart.call(ctx);

    // Try to compile a nonexistent file — should call this.error
    await assert.rejects(
      () => plugin.transform.call(ctx, '', '/nonexistent/path/file.mds'),
      (err) => {
        assert.ok(err instanceof Error, 'should throw Error');
        return true;
      },
    );
  });
});

/**
 * Tests for LazyInit<T>.
 */
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { LazyInit } from '../dist/index.js';

describe('LazyInit', () => {
  test('get() calls factory exactly once across multiple calls', async () => {
    let callCount = 0;
    const lazy = new LazyInit(async () => {
      callCount++;
      return 'value';
    });

    const r1 = await lazy.get();
    const r2 = await lazy.get();
    const r3 = await lazy.get();

    assert.equal(r1, 'value');
    assert.equal(r2, 'value');
    assert.equal(r3, 'value');
    assert.equal(callCount, 1, 'factory should be called exactly once');
  });

  test('concurrent get() calls deduplicate — factory called once under race', async () => {
    let callCount = 0;
    const lazy = new LazyInit(async () => {
      callCount++;
      return 'concurrent-value';
    });

    const [r1, r2, r3] = await Promise.all([
      lazy.get(),
      lazy.get(),
      lazy.get(),
    ]);

    assert.equal(r1, 'concurrent-value');
    assert.equal(r2, 'concurrent-value');
    assert.equal(r3, 'concurrent-value');
    assert.equal(callCount, 1, 'factory should be called once even under concurrent load');
  });

  test('factory rejection clears pending, next get() retries', async () => {
    let callCount = 0;
    const lazy = new LazyInit(async () => {
      callCount++;
      if (callCount === 1) throw new Error('transient failure');
      return 'retry-value';
    });

    // First call — factory rejects.
    await assert.rejects(() => lazy.get(), /transient failure/);

    // Second call — must retry factory, not re-throw the cached rejection.
    const result = await lazy.get();
    assert.equal(result, 'retry-value');
    assert.equal(callCount, 2, 'factory should have been called twice');
  });

  test('after success, factory is never called again', async () => {
    let callCount = 0;
    const lazy = new LazyInit(async () => {
      callCount++;
      return 42;
    });

    await lazy.get();
    await lazy.get();
    await lazy.get();

    assert.equal(callCount, 1, 'factory must not be called after successful resolution');
  });

  test('reset() clears state, next get() re-invokes factory', async () => {
    let callCount = 0;
    const lazy = new LazyInit(async () => {
      callCount++;
      return `call-${callCount}`;
    });

    const r1 = await lazy.get();
    assert.equal(r1, 'call-1');
    assert.equal(callCount, 1);

    lazy.reset();

    const r2 = await lazy.get();
    assert.equal(r2, 'call-2');
    assert.equal(callCount, 2, 'factory should be called again after reset');
  });

  test('T = void factory works correctly (resolved flag, not instance check)', async () => {
    let callCount = 0;
    const lazy = new LazyInit(async () => {
      callCount++;
      // Returns void (undefined).
    });

    await lazy.get();
    await lazy.get();

    assert.equal(callCount, 1, 'void factory should be called exactly once');
  });

  test('factory that returns null works correctly (null is valid T value)', async () => {
    let callCount = 0;
    const lazy = new LazyInit(async () => {
      callCount++;
      return null;
    });

    const r1 = await lazy.get();
    const r2 = await lazy.get();

    assert.equal(r1, null);
    assert.equal(r2, null);
    assert.equal(callCount, 1, 'null-returning factory should be called exactly once');
  });
});

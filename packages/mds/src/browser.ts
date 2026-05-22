import type {
  BackendType,
  CheckResult,
  CompileOptions,
  CompileResult,
  FileOptions,
  InitOptions,
  MdsBackend,
} from './types.js';
import { init as wasmInit } from './backend/wasm.js';

export { isMdsError } from './types.js';
export type {
  CompileResult,
  CheckResult,
  CompileOptions,
  FileOptions,
  MdsError,
  MdsErrorSpan,
  BackendType,
  InitOptions,
} from './types.js';

let _backend: MdsBackend | undefined;
// Promise cached synchronously to prevent double-init race when
// multiple callers invoke init() concurrently.
let _initPromise: Promise<void> | null = null;

/**
 * Initialize the WASM backend. Must be called before compile/check in browser environments.
 * Idempotent — safe to call multiple times. Concurrent calls share the same init promise.
 */
export async function init(options?: InitOptions): Promise<void> {
  if (_backend !== undefined) return;
  if (_initPromise !== null) return _initPromise;
  _initPromise = _doInit(options);
  return _initPromise;
}

async function _doInit(options?: InitOptions): Promise<void> {
  try {
    // wasmInit populates the singleton with options (e.g. wasmUrl) before createWasmBackend reads it.
    await wasmInit(options);
    const { createWasmBackend } = await import('./backend/wasm.js');
    _backend = await createWasmBackend();
  } catch (err) {
    // Reset so a subsequent call can retry after a transient failure.
    _initPromise = null;
    throw err;
  }
}

function assertInitialized(): MdsBackend {
  if (_backend === undefined) {
    throw new Error('@mds/mds: call init() before using compile/check in a browser environment');
  }
  return _backend;
}

export function compile(source: string, options?: CompileOptions): CompileResult {
  return assertInitialized().compile(source, options);
}

export function check(source: string, options?: CompileOptions): CheckResult {
  return assertInitialized().check(source, options);
}

export function getBackend(): BackendType {
  return 'wasm';
}

export function compileFile(_path: string, _options?: FileOptions): Promise<CompileResult> {
  return Promise.reject(
    new Error(
      '@mds/mds: compileFile() is not available in browser environments. ' +
      'Use compile() with a pre-loaded source string instead.',
    ),
  );
}

export function checkFile(_path: string, _options?: FileOptions): Promise<CheckResult> {
  return Promise.reject(
    new Error(
      '@mds/mds: checkFile() is not available in browser environments. ' +
      'Use check() with a pre-loaded source string instead.',
    ),
  );
}

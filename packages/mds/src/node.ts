import type {
  BackendType,
  MdsBackend,
  CompileResult,
  CheckResult,
  CompileOptions,
  FileOptions,
} from './types.js';

const forceBackend = process.env['MDS_BACKEND'] as BackendType | undefined;

let backend: MdsBackend;

if (forceBackend === 'wasm') {
  const { createWasmBackend } = await import('./backend/wasm.js');
  backend = await createWasmBackend();
} else {
  let nativeErr: unknown;
  try {
    const { createRequire } = await import('node:module');
    const require = createRequire(import.meta.url);
    const addon = require('mds-napi') as object;
    const { createNativeBackend } = await import('./backend/native.js');
    backend = createNativeBackend(addon as Parameters<typeof createNativeBackend>[0]);
  } catch (err) {
    nativeErr = err;
    if (forceBackend === 'native') {
      throw new Error(`MDS_BACKEND=native but native addon failed to load: ${String(err)}`);
    }
    try {
      console.warn('@mds/mds: native addon unavailable, falling back to WASM');
      const { createWasmBackend } = await import('./backend/wasm.js');
      backend = await createWasmBackend();
    } catch (wasmErr) {
      throw new Error(
        `@mds/mds: no backend available. Native: ${String(nativeErr)}. WASM: ${String(wasmErr)}`,
      );
    }
  }
}

export function compile(source: string, options?: CompileOptions): CompileResult {
  return backend.compile(source, options);
}

export function check(source: string, options?: CompileOptions): CheckResult {
  return backend.check(source, options);
}

export function compileFile(path: string, options?: FileOptions): Promise<CompileResult> {
  return backend.compileFile(path, options);
}

export function checkFile(path: string, options?: FileOptions): Promise<CheckResult> {
  return backend.checkFile(path, options);
}

export function getBackend(): BackendType {
  return backend.getBackend();
}

export { init } from './backend/wasm.js';
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

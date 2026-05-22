import type {
  BackendType,
  CheckResult,
  CompileOptions,
  CompileResult,
  FileOptions,
  InitOptions,
  MdsBackend,
} from '../types.js';

/**
 * Shape of the WASM module exports (built with --target nodejs).
 * The WASM module exports compile(source, options) and check(source, options).
 * options: { filename?, modules?, vars? }
 */
interface WasmModule {
  compile(source: string, options?: { filename?: string; modules?: Record<string, string>; vars?: Record<string, unknown> }): CompileResult;
  check(source: string, options?: { filename?: string; modules?: Record<string, string>; vars?: Record<string, unknown> }): CheckResult;
  scanImports(source: string): string[];
  default?: (input?: unknown) => Promise<void>;
}

let wasmModule: WasmModule | undefined;
// Promise cached BEFORE async work starts — prevents double-init race.
let initPromise: Promise<void> | null = null;

/**
 * Initialize the WASM backend (idempotent singleton).
 *
 * Must be called before compile/check in browser environments.
 * In Node.js environments loaded via node.ts, this is called automatically.
 */
export async function init(options?: InitOptions): Promise<void> {
  if (initPromise !== null) {
    return initPromise;
  }
  initPromise = _init(options);
  return initPromise;
}

async function _init(options?: InitOptions): Promise<void> {
  // In Node.js: load the built WASM module from the mds-wasm pkg directory.
  // The WASM is built with `wasm-pack build --target nodejs`.
  const { createRequire } = await import('node:module');
  const require = createRequire(import.meta.url);

  // Try to load from the napi package's sibling pkg directory.
  // Fallback paths for different install scenarios.
  const candidates = [
    // Workspace: pkg is built next to mds-wasm crate
    new URL('../../../../../crates/mds-wasm/pkg/mds_wasm.js', import.meta.url).pathname,
    // npm install scenario: mds-wasm might be a separate package
    'mds-wasm',
  ];

  let loadError: unknown;
  for (const candidate of candidates) {
    try {
      const mod = require(candidate) as WasmModule;
      // For nodejs target, wasm-pack generates a CJS module that is already
      // initialized (no need to call default()). If it has a default export
      // that is a function, call it for browser targets.
      if (typeof mod.default === 'function') {
        await mod.default(options?.wasmUrl);
      }
      wasmModule = mod;
      return;
    } catch (e) {
      loadError = e;
    }
  }

  throw new Error(
    `@mds/mds: failed to load WASM module. Build it first with: wasm-pack build crates/mds-wasm --target nodejs --out-dir pkg. ${String(loadError)}`,
  );
}

function assertInitialized(): WasmModule {
  if (wasmModule === undefined) {
    throw new Error(
      '@mds/mds: WASM backend not initialized. Call init() first.',
    );
  }
  return wasmModule;
}

/**
 * Create a WASM backend instance. Calls init() internally.
 */
export async function createWasmBackend(): Promise<MdsBackend> {
  await init();
  return {
    compile(source: string, options?: CompileOptions): CompileResult {
      const wasm = assertInitialized();
      return wasm.compile(source, {
        filename: 'input.mds',
        modules: {},
        ...(options?.vars !== undefined ? { vars: options.vars } : {}),
      });
    },

    check(source: string, options?: CompileOptions): CheckResult {
      const wasm = assertInitialized();
      return wasm.check(source, {
        filename: 'input.mds',
        modules: {},
        ...(options?.vars !== undefined ? { vars: options.vars } : {}),
      });
    },

    async compileFile(path: string, options?: FileOptions): Promise<CompileResult> {
      const wasm = assertInitialized();
      const { buildModulesMap } = await import('../util/module-scanner.js');
      const { entryFilename, modules } = await buildModulesMap(
        path,
        (source) => wasm.scanImports(source),
        { maxModules: 256, maxAggregateSize: 10 * 1024 * 1024 },
      );
      return wasm.compile(modules[entryFilename] ?? '', {
        filename: entryFilename,
        modules,
        ...(options?.vars !== undefined ? { vars: options.vars } : {}),
      });
    },

    async checkFile(path: string, options?: FileOptions): Promise<CheckResult> {
      const wasm = assertInitialized();
      const { buildModulesMap } = await import('../util/module-scanner.js');
      const { entryFilename, modules } = await buildModulesMap(
        path,
        (source) => wasm.scanImports(source),
        { maxModules: 256, maxAggregateSize: 10 * 1024 * 1024 },
      );
      return wasm.check(modules[entryFilename] ?? '', {
        filename: entryFilename,
        modules,
        ...(options?.vars !== undefined ? { vars: options.vars } : {}),
      });
    },

    getBackend(): BackendType {
      return 'wasm';
    },
  };
}

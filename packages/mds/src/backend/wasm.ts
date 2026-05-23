import type {
  BackendType,
  CheckResult,
  CompileOptions,
  CompileResult,
  FileOptions,
  InitOptions,
  MdsBackend,
} from '../types.js';
import { buildModulesMap } from '../util/module-scanner.js';
import { varsOpt } from '../util/options.js';

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
const MAX_INIT_RETRIES = 3;
let initFailures = 0;

/**
 * Reset all singleton state, optionally pre-seeding the failure counter.
 *
 * FOR TESTING ONLY — allows integration tests to exercise the retry-exhaustion
 * path without spawning a subprocess or driving N actual failures.
 *
 * @param failures - Number of failures to pre-seed. Defaults to 0 (full reset).
 *                   Pass MAX_INIT_RETRIES (3) to simulate exhaustion directly.
 * @internal
 */
export function _resetForTesting(failures = 0): void {
  wasmModule = undefined;
  initPromise = null;
  initFailures = failures;
}

/**
 * Initialize the WASM backend (idempotent singleton).
 *
 * Must be called before compile/check in browser environments.
 * In Node.js environments loaded via node.ts, this is called automatically.
 *
 * Concurrent calls share the same init promise. If init fails, the cached
 * promise is cleared so subsequent calls can retry, up to MAX_INIT_RETRIES
 * times. After that, every call throws immediately without re-attempting.
 */
export async function init(options?: InitOptions): Promise<void> {
  if (initPromise !== null) {
    return initPromise;
  }
  if (initFailures >= MAX_INIT_RETRIES) {
    throw new Error(
      `@mds/mds: WASM backend failed to initialize after ${MAX_INIT_RETRIES} attempts. Check that the WASM module is built and accessible.`,
    );
  }
  initPromise = _init(options).catch((err) => {
    // Reset so a subsequent call can retry after a transient failure.
    initFailures += 1;
    initPromise = null;
    throw err;
  });
  return initPromise;
}

/**
 * Attempt to load a single WASM candidate path.
 *
 * Returns the loaded module on success, or null if the candidate is not found.
 * Re-throws unexpected errors so the caller can surface them.
 */
async function tryLoadCandidate(
  candidate: string,
  require: NodeRequire,
  wasmUrl: InitOptions['wasmUrl'],
): Promise<WasmModule | null> {
  try {
    const mod = require(candidate) as WasmModule;
    // For nodejs target, wasm-pack generates a CJS module that is already
    // initialized (no need to call default()). If it has a default export
    // that is a function, call it for browser targets.
    if (typeof mod.default === 'function') {
      await mod.default(wasmUrl);
    }
    return mod;
  } catch {
    return null;
  }
}

async function _init(options?: InitOptions): Promise<void> {
  // In Node.js: load the built WASM module from the mds-wasm pkg directory.
  // The WASM is built with `wasm-pack build --target nodejs`.
  const { createRequire } = await import('node:module');
  const require = createRequire(import.meta.url);

  // Exactly 2 candidates — structurally bounded; no dynamic growth expected.
  const candidates: readonly string[] = [
    // Workspace: pkg is built next to mds-wasm crate
    new URL('../../../../crates/mds-wasm/pkg/mds_wasm.js', import.meta.url).pathname,
    // npm install scenario: mds-wasm might be a separate package
    'mds-wasm',
  ];

  for (const candidate of candidates) {
    const mod = await tryLoadCandidate(candidate, require, options?.wasmUrl);
    if (mod !== null) {
      wasmModule = mod;
      return;
    }
  }

  throw new Error(
    `@mds/mds: failed to load WASM module. Build it first with: wasm-pack build crates/mds-wasm --target nodejs --out-dir pkg`,
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
 * Deep-frozen default compile/check options for the common no-vars path.
 * Both the outer object and the nested modules map are frozen so that WASM
 * FFI cannot mutate shared state across calls.
 */
const DEFAULT_MODULES: Record<string, string> = Object.freeze({} as Record<string, string>);
const DEFAULT_COMPILE_OPTS = Object.freeze({ filename: 'input.mds', modules: DEFAULT_MODULES });

/** Build the options object for compile/check, merging vars when present. */
function compileOpts(options?: CompileOptions): { filename: string; modules: Record<string, string>; vars?: Record<string, unknown> } {
  const vars = varsOpt(options);
  return vars !== undefined ? { ...DEFAULT_COMPILE_OPTS, ...vars } : DEFAULT_COMPILE_OPTS;
}

/**
 * Create a WASM backend instance. Calls init() internally.
 */
export async function createWasmBackend(options?: InitOptions): Promise<MdsBackend> {
  await init(options);
  return {
    compile(source: string, options?: CompileOptions): CompileResult {
      const wasm = assertInitialized();
      return wasm.compile(source, compileOpts(options));
    },

    check(source: string, options?: CompileOptions): CheckResult {
      const wasm = assertInitialized();
      return wasm.check(source, compileOpts(options));
    },

    async compileFile(path: string, options?: FileOptions): Promise<CompileResult> {
      const wasm = assertInitialized();
      const { entryFilename, modules } = await buildModulesMap(path, (src) => wasm.scanImports(src));
      return wasm.compile(modules[entryFilename] ?? '', {
        filename: entryFilename,
        modules,
        ...varsOpt(options),
      });
    },

    async checkFile(path: string, options?: FileOptions): Promise<CheckResult> {
      const wasm = assertInitialized();
      const { entryFilename, modules } = await buildModulesMap(path, (src) => wasm.scanImports(src));
      return wasm.check(modules[entryFilename] ?? '', {
        filename: entryFilename,
        modules,
        ...varsOpt(options),
      });
    },

    getBackend(): BackendType {
      return 'wasm';
    },
  };
}

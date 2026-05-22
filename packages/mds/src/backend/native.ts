import type {
  BackendType,
  CheckResult,
  CompileOptions,
  CompileResult,
  FileOptions,
  MdsBackend,
} from '../types.js';

/**
 * Shape of the napi addon exports.
 * compile/check accept { basePath?, vars? } for string sources.
 * compileFile/checkFile accept { vars? } for file paths.
 */
interface NapiAddon {
  compile(source: string, opts?: { basePath?: string; vars?: Record<string, unknown> }): CompileResult;
  check(source: string, opts?: { basePath?: string; vars?: Record<string, unknown> }): CheckResult;
  compileFile(path: string, opts?: { vars?: Record<string, unknown> }): CompileResult;
  checkFile(path: string, opts?: { vars?: Record<string, unknown> }): CheckResult;
}

/**
 * Create a native (napi) backend adapter from an injected addon.
 *
 * The addon is injected rather than imported directly so callers can test
 * with a mock and the module remains environment-agnostic.
 */
export function createNativeBackend(addon: NapiAddon): MdsBackend {
  return {
    compile(source: string, options?: CompileOptions): CompileResult {
      return addon.compile(source, options?.vars !== undefined ? { vars: options.vars } : undefined);
    },

    check(source: string, options?: CompileOptions): CheckResult {
      return addon.check(source, options?.vars !== undefined ? { vars: options.vars } : undefined);
    },

    compileFile(path: string, options?: FileOptions): Promise<CompileResult> {
      try {
        return Promise.resolve(
          addon.compileFile(path, options?.vars !== undefined ? { vars: options.vars } : undefined),
        );
      } catch (err) {
        return Promise.reject(err);
      }
    },

    checkFile(path: string, options?: FileOptions): Promise<CheckResult> {
      try {
        return Promise.resolve(
          addon.checkFile(path, options?.vars !== undefined ? { vars: options.vars } : undefined),
        );
      } catch (err) {
        return Promise.reject(err);
      }
    },

    getBackend(): BackendType {
      return 'native';
    },
  };
}

import type { MdsApi, MdsPluginOptions, TransformResult } from './types.js';
import { shouldTransform as checkTransform, cleanId } from './frontmatter.js';

// Unicode line separator (U+2028) and paragraph separator (U+2029) must be
// escaped in JavaScript string literals embedded in source code.
function escapeForJs(str: string): string {
  let result = '';
  for (let i = 0; i < str.length; i++) {
    const ch = str[i]!;
    const code = ch.charCodeAt(0);
    switch (true) {
      case ch === '\\': result += '\\\\'; break;
      case ch === '"': result += '\\"'; break;
      case ch === '\n': result += '\\n'; break;
      case ch === '\r': result += '\\r'; break;
      case code === 0x2028: result += '\\u2028'; break;
      case code === 0x2029: result += '\\u2029'; break;
      default: result += ch;
    }
  }
  return result;
}

export function createMdsTransformer(mds: MdsApi, options?: MdsPluginOptions): {
  shouldTransform(id: string): boolean | Promise<boolean>;
  transform(id: string): Promise<TransformResult>;
} {
  let initialized = false;
  let initPromise: Promise<void> | null = null;

  async function ensureInit(): Promise<void> {
    if (initialized) return;
    if (initPromise === null) {
      initPromise = mds.init().then(() => {
        initialized = true;
      });
    }
    return initPromise;
  }

  return {
    shouldTransform(id: string): boolean | Promise<boolean> {
      return checkTransform(id);
    },

    async transform(id: string): Promise<TransformResult> {
      await ensureInit();
      const clean = cleanId(id);
      const result = await mds.compileFile(
        clean,
        options?.vars !== undefined ? { vars: options.vars } : undefined,
      );
      const code =
        `export default "${escapeForJs(result.output)}";\n` +
        `export const metadata = ${JSON.stringify({ warnings: result.warnings, dependencies: result.dependencies })};\n`;
      return {
        code,
        dependencies: result.dependencies,
        warnings: result.warnings,
      };
    },
  };
}

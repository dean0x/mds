import { createMdsTransformer, formatMdsError } from '@mds/bundler-utils';
import type { MdsPluginOptions } from '@mds/bundler-utils';

interface LoaderContext {
  resourcePath: string;
  async(): (err: Error | null, content?: string) => void;
  addDependency(dep: string): void;
  emitWarning(err: Error): void;
  getOptions(): MdsPluginOptions;
}

let transformer: ReturnType<typeof createMdsTransformer> | null = null;
let initPromise: Promise<void> | null = null;

async function ensureTransformer(options: MdsPluginOptions): Promise<NonNullable<typeof transformer>> {
  if (transformer !== null) return transformer;
  if (initPromise === null) {
    initPromise = import('@mds/mds').then((mds) => {
      transformer = createMdsTransformer(mds, options);
    });
  }
  await initPromise;
  // After initPromise resolves, transformer is guaranteed to be set.
  // The non-null assertion is safe here because the Promise sets transformer before resolving.
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  return transformer!;
}

export default async function mdsLoader(this: LoaderContext): Promise<void> {
  const callback = this.async();
  try {
    const options = this.getOptions();
    const t = await ensureTransformer(options);
    const result = await t.transform(this.resourcePath);
    for (const dep of result.dependencies) {
      this.addDependency(dep);
    }
    for (const warning of result.warnings) {
      this.emitWarning(new Error(warning));
    }
    callback(null, result.code);
  } catch (err) {
    const formatted = formatMdsError(err, this.resourcePath);
    callback(new Error(formatted.message));
  }
}

/**
 * Reset singleton state for testing.
 * FOR TESTING ONLY.
 */
export function _resetForTesting(): void {
  transformer = null;
  initPromise = null;
}

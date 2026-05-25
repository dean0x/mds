import type { MdsPluginOptions } from '@mds/bundler-utils';
interface LoaderContext {
    resourcePath: string;
    async(): (err: Error | null, content?: string) => void;
    addDependency(dep: string): void;
    emitWarning(err: Error): void;
    getOptions(): MdsPluginOptions;
}
export default function mdsLoader(this: LoaderContext): Promise<void>;
/**
 * Reset singleton state for testing.
 * FOR TESTING ONLY.
 */
export declare function _resetForTesting(): void;
export {};
//# sourceMappingURL=index.d.ts.map
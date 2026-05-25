import type { MdsPluginOptions } from '@mds/bundler-utils';
interface PluginContext {
    warn(msg: string): void;
    addWatchFile(id: string): void;
    error(msg: string, pos?: {
        line: number;
        column: number;
    }): never;
}
interface RollupPlugin {
    name: string;
    buildStart?: (this: PluginContext) => void | Promise<void>;
    transform?: (this: PluginContext, code: string, id: string) => Promise<{
        code: string;
        map: null;
    } | null>;
}
export default function mdsPlugin(options?: MdsPluginOptions): RollupPlugin;
export {};
//# sourceMappingURL=index.d.ts.map
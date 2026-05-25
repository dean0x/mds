import type { MdsPluginOptions } from '@mds/bundler-utils';
interface PluginTransformContext {
    warn(msg: string): void;
    addWatchFile(id: string): void;
}
interface VitePlugin {
    name: string;
    enforce?: 'pre' | 'post';
    buildStart?: (this: PluginTransformContext) => void | Promise<void>;
    transform?: (this: PluginTransformContext, code: string, id: string) => Promise<{
        code: string;
        map: null;
    } | null>;
    handleHotUpdate?: (ctx: {
        file: string;
        server: {
            ws: {
                send(payload: {
                    type: string;
                    path?: string;
                }): void;
            };
        };
    }) => void | undefined | unknown[];
}
export default function mdsPlugin(options?: MdsPluginOptions): VitePlugin;
export {};
//# sourceMappingURL=index.d.ts.map
import type { MdsApi, MdsPluginOptions, TransformResult } from './types.js';
export declare function createMdsTransformer(mds: MdsApi, options?: MdsPluginOptions): {
    shouldTransform(id: string): boolean | Promise<boolean>;
    transform(id: string): Promise<TransformResult>;
};
//# sourceMappingURL=transform.d.ts.map
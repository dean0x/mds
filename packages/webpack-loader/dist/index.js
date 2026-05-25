import { createMdsTransformer, formatMdsError } from '@mds/bundler-utils';
let transformer = null;
let initPromise = null;
async function ensureTransformer(options) {
    if (transformer !== null)
        return transformer;
    if (initPromise === null) {
        initPromise = import('@mds/mds').then((mds) => {
            transformer = createMdsTransformer(mds, options);
        });
    }
    await initPromise;
    // After initPromise resolves, transformer is guaranteed to be set.
    // The non-null assertion is safe here because the Promise sets transformer before resolving.
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    return transformer;
}
export default async function mdsLoader() {
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
    }
    catch (err) {
        const formatted = formatMdsError(err, this.resourcePath);
        callback(new Error(formatted.message));
    }
}
/**
 * Reset singleton state for testing.
 * FOR TESTING ONLY.
 */
export function _resetForTesting() {
    transformer = null;
    initPromise = null;
}
//# sourceMappingURL=index.js.map
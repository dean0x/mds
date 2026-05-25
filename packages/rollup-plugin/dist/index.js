import { createMdsTransformer, formatMdsError, cleanId } from '@mds/bundler-utils';
export default function mdsPlugin(options) {
    let transformer = null;
    return {
        name: 'mds',
        async buildStart() {
            const mds = await import('@mds/mds');
            transformer = createMdsTransformer(mds, options);
        },
        async transform(_, id) {
            if (transformer === null)
                return null;
            const clean = cleanId(id);
            const should = await transformer.shouldTransform(clean);
            if (!should)
                return null;
            try {
                const result = await transformer.transform(id);
                for (const dep of result.dependencies) {
                    this.addWatchFile(dep);
                }
                for (const warning of result.warnings) {
                    this.warn(warning);
                }
                return { code: result.code, map: null };
            }
            catch (err) {
                const formatted = formatMdsError(err, clean);
                if (formatted.line !== undefined) {
                    this.error(formatted.message, {
                        line: formatted.line,
                        column: formatted.column ?? 0,
                    });
                }
                else {
                    this.error(formatted.message);
                }
            }
        },
    };
}
//# sourceMappingURL=index.js.map
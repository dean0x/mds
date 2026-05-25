import { createMdsTransformer, formatMdsError, cleanId, isMdsExtension } from '@mds/bundler-utils';
export default function mdsPlugin(options) {
    let transformer = null;
    return {
        name: 'mds',
        enforce: 'pre',
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
                // Vite expects thrown errors (not this.error()) for the error overlay.
                // Attach loc and id so Vite can display the error with position info.
                const error = new Error(formatted.message);
                error.id = formatted.id;
                if (formatted.line !== undefined) {
                    error.loc = { line: formatted.line, column: formatted.column ?? 0 };
                }
                throw error;
            }
        },
        handleHotUpdate(ctx) {
            const clean = cleanId(ctx.file);
            if (isMdsExtension(clean)) {
                ctx.server.ws.send({ type: 'full-reload' });
                return [];
            }
            return undefined;
        },
    };
}
//# sourceMappingURL=index.js.map
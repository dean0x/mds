import { readFile } from 'node:fs/promises';
export function isMdsExtension(id) {
    return id.endsWith('.mds');
}
export function cleanId(id) {
    const qIdx = id.indexOf('?');
    const hIdx = id.indexOf('#');
    if (qIdx === -1 && hIdx === -1)
        return id;
    const cutAt = qIdx !== -1 && hIdx !== -1
        ? Math.min(qIdx, hIdx)
        : qIdx !== -1
            ? qIdx
            : hIdx;
    return id.slice(0, cutAt);
}
/**
 * Checks whether a file should be transformed by the MDS bundler plugin.
 *
 * - `.mds` files: always transform (synchronous true)
 * - `.md` files with `type: mds` inside their frontmatter block: transform (async)
 * - Everything else: skip (synchronous false or async false)
 *
 * Frontmatter detection reads only the first 500 bytes and looks for:
 * 1. File starts with `---`
 * 2. There is a closing `---` before byte 500
 * 3. Between the opening and closing `---`, there is a `type: mds` key
 */
export function shouldTransform(id) {
    const clean = cleanId(id);
    if (isMdsExtension(clean))
        return true;
    if (!clean.endsWith('.md'))
        return false;
    // Async: read first 500 bytes and check for type: mds in frontmatter
    return readFile(clean, { encoding: 'utf-8' })
        .then((content) => {
        const head = content.slice(0, 500);
        if (!head.startsWith('---'))
            return false;
        // Find the closing --- (must be after the opening line, i.e. after index 3)
        const closeIdx = head.indexOf('\n---', 3);
        if (closeIdx === -1)
            return false;
        // Extract frontmatter block (between opening --- and closing ---)
        const frontmatter = head.slice(3, closeIdx);
        // Check for `type: mds` as a YAML key (at start of line or after whitespace)
        return /(?:^|\n)\s*type:\s*mds\b/.test(frontmatter);
    })
        .catch(() => false);
}
//# sourceMappingURL=frontmatter.js.map
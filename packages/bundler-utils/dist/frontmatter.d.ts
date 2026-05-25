export declare function isMdsExtension(id: string): boolean;
export declare function cleanId(id: string): string;
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
export declare function shouldTransform(id: string): boolean | Promise<boolean>;
//# sourceMappingURL=frontmatter.d.ts.map
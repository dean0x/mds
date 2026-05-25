function isMdsErrorLike(err) {
    if (!(err instanceof Error))
        return false;
    const code = err['code'];
    return typeof code === 'string' && code.startsWith('mds::');
}
export function formatMdsError(err, id) {
    if (isMdsErrorLike(err)) {
        let message = err.message;
        if (err.help !== undefined)
            message += `\n  help: ${err.help}`;
        const result = { message, id };
        if (err.span?.line !== undefined)
            result.line = err.span.line;
        if (err.span?.column !== undefined)
            result.column = err.span.column;
        return result;
    }
    if (err instanceof Error) {
        return { message: err.message, id };
    }
    return { message: String(err), id };
}
//# sourceMappingURL=errors.js.map
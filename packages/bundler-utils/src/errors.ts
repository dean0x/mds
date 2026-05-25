import type { FormattedError } from './types.js';

interface MdsErrorLike {
  message: string;
  code?: string;
  help?: string;
  span?: { line?: number; column?: number };
}

function isMdsErrorLike(err: unknown): err is MdsErrorLike {
  if (!(err instanceof Error)) return false;
  const code = (err as unknown as Record<string, unknown>)['code'];
  return typeof code === 'string' && code.startsWith('mds::');
}

export function formatMdsError(err: unknown, id: string): FormattedError {
  if (isMdsErrorLike(err)) {
    let message = err.message;
    if (err.help !== undefined) message += `\n  help: ${err.help}`;
    const result: FormattedError = { message, id };
    if (err.span?.line !== undefined) result.line = err.span.line;
    if (err.span?.column !== undefined) result.column = err.span.column;
    return result;
  }
  if (err instanceof Error) {
    return { message: err.message, id };
  }
  return { message: String(err), id };
}

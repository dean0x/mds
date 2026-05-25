export interface MdsApi {
  compileFile(path: string, options?: { vars?: Record<string, unknown> }): Promise<CompileResult>;
  init(): Promise<void>;
  isMdsError(err: unknown): boolean;
}

export interface CompileResult {
  output: string;
  warnings: string[];
  dependencies: string[];
}

export interface TransformResult {
  code: string;
  dependencies: string[];
  warnings: string[];
}

export interface MdsPluginOptions {
  vars?: Record<string, unknown>;
}

export interface FormattedError {
  message: string;
  id?: string;
  line?: number;
  column?: number;
  frame?: string;
}

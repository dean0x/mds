declare module '*.mds' {
  const content: string;
  export default content;
  export const metadata: { warnings: string[]; dependencies: string[] };
}

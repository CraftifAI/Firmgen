declare module "highlight.js/lib/core" {
  const hljs: {
    registerLanguage: (name: string, module: unknown) => void;
    getLanguage: (name: string) => unknown;
    highlight: (
      text: string,
      options: { language: string; ignoreIllegals?: boolean },
    ) => { value: string };
  };
  export default hljs;
}

declare module "highlight.js/lib/languages/*" {
  const language: unknown;
  export default language;
}

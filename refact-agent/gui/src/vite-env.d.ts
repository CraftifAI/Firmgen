/// <reference types="vite/client" />

declare module "*.module.css" {
  const classes: { readonly [key: string]: string };
  export default classes;
}

interface ImportMetaEnv {
  readonly VITE_REFACT_LSP_PORT?: string;
  readonly VITE_CRAFTIF_API_BASE?: string;
}
interface ImportMeta {
  readonly env: ImportMetaEnv;
}

type VersionInfo = { semver?: string; commit?: string } | undefined;
declare const __REFACT_CHAT_VERSION__: VersionInfo;
declare const __REFACT_LSP_PORT__: number | undefined;
declare const __REFACT_EMBEDDED_MODE__: boolean | undefined;
interface Window {
  __REFACT_CHAT_VERSION__: VersionInfo;
  __REFACT_LSP_PORT__?: number;
  __REFACT_EMBEDDED_MODE__?: boolean;
}

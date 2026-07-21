/// <reference types="vite/client" />

declare module "*.png" {
  const src: string;
  export default src;
}

declare module "@anycode/brand-mark" {
  const src: string;
  export default src;
}

interface ImportMetaEnv {
  readonly VITE_API_BASE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE?: string;
  readonly VITE_ACCOUNT_API_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

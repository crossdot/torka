/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Base path/URL of the versioned API, e.g. `/api/v1`. */
  readonly VITE_API_BASE_URL?: string;
  /** Bearer token for `/api/*`; build-time value, never a stored secret. */
  readonly VITE_API_TOKEN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

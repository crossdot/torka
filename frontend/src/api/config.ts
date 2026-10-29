/**
 * API endpoint configuration.
 *
 * The token comes from `VITE_API_TOKEN` (build-time) and is sent only as an
 * `Authorization` header. It is never written to localStorage/sessionStorage and
 * no cookies are used for identification (API.md / ADR-005).
 */

const DEFAULT_API_BASE_URL = '/api/v1';

const configuredBaseUrl = import.meta.env.VITE_API_BASE_URL?.trim() || DEFAULT_API_BASE_URL;

/** Versioned API base, e.g. `/api/v1` or `https://host/api/v1`. */
export const API_BASE_URL: string = configuredBaseUrl.replace(/\/+$/, '');

const TOKEN_STORAGE_KEY = 'api_token';
const buildToken = import.meta.env.VITE_API_TOKEN?.trim() ?? '';

/** Runtime bearer token: sessionStorage override, else the build-time default. */
export function getApiToken(): string {
  const stored =
    typeof sessionStorage !== 'undefined' ? sessionStorage.getItem(TOKEN_STORAGE_KEY) : null;
  return (stored ?? buildToken).trim();
}

export function setApiToken(token: string): void {
  sessionStorage.setItem(TOKEN_STORAGE_KEY, token.trim());
}

export function clearApiToken(): void {
  sessionStorage.removeItem(TOKEN_STORAGE_KEY);
}

/** Health/readiness are public and live outside the versioned API prefix. */
const ROOT_BASE_URL: string = API_BASE_URL.replace(/\/api\/v1$/i, '');

export function hasApiToken(): boolean {
  return getApiToken().length > 0;
}

/** Absolute path for a versioned API resource. */
export function apiUrl(path: string): string {
  return `${API_BASE_URL}${path.startsWith('/') ? path : `/${path}`}`;
}

/** Absolute path for a root-level endpoint such as `/health`. */
export function rootUrl(path: string): string {
  return `${ROOT_BASE_URL}${path.startsWith('/') ? path : `/${path}`}`;
}

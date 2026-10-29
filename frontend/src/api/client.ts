/**
 * Minimal typed JSON client for the torka API.
 *
 * Responsibilities:
 * - attach the bearer token to `/api/*` requests (never cookies);
 * - build query strings without leaking `undefined`/`null` filters;
 * - normalise both the documented `{"error":{...}}` envelope and the flat
 *   `{code,message,request_id}` shape into a single `ApiError`;
 * - surface aborts unchanged so callers can ignore superseded requests.
 */

import { getApiToken } from './config';
import type { ApiErrorDetail } from '@/types';

/**
 * `code` is either a server code from API.md (`VALIDATION_ERROR`,
 * `PROVIDER_TIMEOUT`, ...) or a client-side value: `CLIENT_MISSING_TOKEN`,
 * `NETWORK_ERROR`, `INVALID_RESPONSE`, or `HTTP_<status>` when a non-2xx
 * response has no documented error envelope (API.md defines no dedicated code
 * for 401, for example).
 */
export class ApiError extends Error {
  readonly code: string;
  readonly status: number;
  readonly requestId: string | null;

  constructor(
    code: string,
    message: string,
    options: { status?: number; requestId?: string | null } = {},
  ) {
    super(message);
    this.name = 'ApiError';
    this.code = code;
    this.status = options.status ?? 0;
    this.requestId = options.requestId ?? null;
  }
}

export type QueryValue = string | number | boolean | null | undefined;

export interface RequestOptions {
  query?: Record<string, QueryValue>;
  signal?: AbortSignal;
  /** Set to `false` for public endpoints (`/health`, `/ready`). */
  auth?: boolean;
  /** HTTP method; defaults to `GET`. */
  method?: string;
  /** JSON body for non-GET requests. */
  body?: unknown;
}

function buildUrl(url: string, query?: Record<string, QueryValue>): string {
  if (!query) {
    return url;
  }
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value === undefined || value === null) {
      continue;
    }
    params.set(key, String(value));
  }
  const search = params.toString();
  return search.length > 0 ? `${url}?${search}` : url;
}

/** Extracts the error detail from either documented or legacy response shapes. */
export function parseErrorPayload(body: unknown): ApiErrorDetail | null {
  if (typeof body !== 'object' || body === null) {
    return null;
  }
  // `body` and `body.error` are untrusted JSON: only the fields we read are
  // narrowed (see API.md), everything else stays `unknown`.
  const payload = body as Record<string, unknown>;
  const candidate =
    typeof payload.error === 'object' && payload.error !== null
      ? (payload.error as Record<string, unknown>)
      : payload;
  const code = candidate.code;
  const message = candidate.message;
  if (typeof code !== 'string' || typeof message !== 'string') {
    return null;
  }
  const requestId = typeof candidate.request_id === 'string' ? candidate.request_id : null;
  return { code, message, request_id: requestId };
}

export function isAbortError(error: unknown): boolean {
  return error instanceof Error && error.name === 'AbortError';
}

export async function requestJson<T>(url: string, options: RequestOptions = {}): Promise<T> {
  const headers = new Headers({ Accept: 'application/json' });

  if (options.auth !== false) {
    const token = getApiToken();
    if (token.length > 0) {
      headers.set('Authorization', `Bearer ${token}`);
    }
  }

  if (options.body !== undefined) {
    headers.set('Content-Type', 'application/json');
  }

  let response: Response;
  try {
    response = await fetch(buildUrl(url, options.query), {
      method: options.method ?? 'GET',
      headers,
      body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
      // Bearer auth only: no cookies are sent or relied on (API.md).
      credentials: 'omit',
      cache: 'no-store',
      signal: options.signal,
    });
  } catch (error) {
    if (isAbortError(error) || options.signal?.aborted) {
      throw error;
    }
    throw new ApiError('NETWORK_ERROR', 'Не удалось связаться с API.');
  }

  const headerRequestId = response.headers.get('x-request-id');
  const text = await response.text();
  let body: unknown = null;
  if (text.length > 0) {
    try {
      body = JSON.parse(text);
    } catch {
      body = null;
    }
  }

  if (!response.ok) {
    const detail = parseErrorPayload(body);
    throw new ApiError(
      detail?.code ?? `HTTP_${response.status}`,
      detail?.message ?? `Запрос завершился с HTTP ${response.status}.`,
      { status: response.status, requestId: detail?.request_id ?? headerRequestId },
    );
  }

  if (body === null) {
    // Empty success body (e.g., 202 on DELETE) — nothing to return.
    return undefined as T;
  }

  return body as T;
}

import { requestJson } from './client';
import { apiUrl } from './config';
import type { SearchResponse } from '@/types';

export interface SearchRequest {
  readonly q: string;
  /** Optional year filter; omitted from the query string when null. */
  readonly year?: number | null;
  readonly signal?: AbortSignal;
}

/** `GET /api/v1/search?q=...&year=...` (API.md). */
export function searchTitles({ q, year = null, signal }: SearchRequest): Promise<SearchResponse> {
  return requestJson<SearchResponse>(apiUrl('/search'), {
    query: { q, year },
    signal,
  });
}

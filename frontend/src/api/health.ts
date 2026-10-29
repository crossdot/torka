import { requestJson } from './client';
import { rootUrl } from './config';
import type { HealthResponse } from '@/types';

/** `GET /health` — public liveness endpoint (API.md, no bearer token). */
export function fetchHealth(signal?: AbortSignal): Promise<HealthResponse> {
  return requestJson<HealthResponse>(rootUrl('/health'), { auth: false, signal });
}

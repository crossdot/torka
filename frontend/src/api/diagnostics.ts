import { requestJson } from './client';
import { apiUrl } from './config';
import type { DiagnosticsResponse } from '@/types';

/** `GET /api/v1/diagnostics` — safe config/health snapshot (no secrets). */
export function fetchDiagnostics(signal?: AbortSignal): Promise<DiagnosticsResponse> {
  return requestJson<DiagnosticsResponse>(apiUrl('/diagnostics'), { signal });
}

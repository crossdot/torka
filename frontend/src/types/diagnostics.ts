/** `GET /api/v1/diagnostics` — safe config/health snapshot (no secrets). */
export interface DiagnosticsResponse {
  readonly version: string;
  readonly providers: readonly string[];
  readonly hwaccel_mode: string;
  readonly auth_configured: boolean;
}

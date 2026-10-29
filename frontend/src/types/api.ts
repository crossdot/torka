/**
 * Transport-level types from `API.md`. Only fields that the contract defines are
 * modelled here; optionality/nullability reflects that a provider may not know
 * a value, not new API surface.
 */

/** Unified API error detail: `{"error":{"code","message","request_id"}}`. */
export interface ApiErrorDetail {
  readonly code: string;
  readonly message: string;
  readonly request_id: string | null;
}

/** `GET /health` and `GET /ready` (public, outside `/api/v1`). */
export interface HealthResponse {
  readonly status: string;
  readonly version: string;
}

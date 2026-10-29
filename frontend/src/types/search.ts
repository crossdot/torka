/**
 * `GET /api/v1/search` response as sketched in `API.md`. No fields are added
 * beyond the documented search sketch.
 */

export interface SearchItem {
  readonly id: string;
  readonly title: string;
  readonly year: number | null;
  readonly provider_id: string;
  readonly size_bytes: number | null;
  readonly seeders: number | null;
  readonly leechers: number | null;
  readonly info_hash: string | null;
  readonly magnet_available: boolean;
  readonly quality: string | null;
  readonly languages: readonly string[];
  readonly source_url: string | null;
}

/**
 * Per-provider outcome. `API.md` only pins the `"ok"` value; treat any other
 * value, or a non-null `error_code`, as a provider failure.
 */
export interface ProviderStatus {
  readonly id: string;
  readonly status: string;
  readonly duration_ms: number | null;
  readonly error_code: string | null;
}

export interface SearchResponse {
  readonly items: readonly SearchItem[];
  readonly providers: readonly ProviderStatus[];
}

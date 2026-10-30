/** Playback session contract from API.md. */
export interface CreatePlaybackResponse {
  readonly session_id: string;
  readonly status: string;
  readonly status_url: string;
}

export interface PlaybackStatus {
  readonly session_id: string;
  readonly status: string;
  readonly error?: string | null;
  readonly duration_secs?: number | null;
  readonly hls_window_secs?: number;
}

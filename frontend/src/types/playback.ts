/** Playback session contract from API.md. */
export interface CreatePlaybackResponse {
  readonly session_id: string;
  readonly status: string;
  readonly status_url: string;
}

export interface PlaybackStatus {
  readonly session_id: string;
  readonly status: string;
}

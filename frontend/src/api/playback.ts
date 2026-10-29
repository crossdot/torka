import { requestJson } from './client';
import { apiUrl } from './config';
import type { CreatePlaybackResponse, PlaybackStatus } from '@/types';

export interface CreateSessionRequest {
  readonly source_id: string;
  readonly file_id?: number | null;
  readonly preferred_mode?: string;
}

/** `POST /api/v1/playback/sessions` — create a session (202). */
export function createPlaybackSession(body: CreateSessionRequest): Promise<CreatePlaybackResponse> {
  return requestJson<CreatePlaybackResponse>(apiUrl('/playback/sessions'), {
    method: 'POST',
    body,
  });
}

/** `GET /api/v1/playback/sessions/{id}` — session status. */
export function getPlaybackStatus(id: string, signal?: AbortSignal): Promise<PlaybackStatus> {
  return requestJson<PlaybackStatus>(apiUrl(`/playback/sessions/${id}`), { signal });
}

/** `DELETE /api/v1/playback/sessions/{id}` — stop (idempotent). */
export async function stopPlaybackSession(id: string): Promise<void> {
  await requestJson<unknown>(apiUrl(`/playback/sessions/${id}`), { method: 'DELETE' });
}

/** `POST /api/v1/playback/sessions/{id}/seek` — restart the pipeline at `positionSecs`. */
export function seekPlaybackSession(id: string, positionSecs: number): Promise<void> {
  return requestJson<void>(apiUrl(`/playback/sessions/${id}/seek`), {
    method: 'POST',
    body: { position_secs: positionSecs },
  });
}

/** Absolute-path URL of the HLS master playlist for a session. */
export function playlistUrl(id: string): string {
  return apiUrl(`/playback/sessions/${id}/master.m3u8`);
}

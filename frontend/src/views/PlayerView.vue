<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import Hls from 'hls.js';
import { RouterLink, useRoute } from 'vue-router';

import {
  createPlaybackSession,
  getPlaybackStatus,
  playlistUrl,
  seekPlaybackSession,
  stopPlaybackSession,
} from '@/api/playback';

type Phase = 'loading' | 'ready' | 'error';

const route = useRoute();
const sourceId = String(route.params.sourceId);

const video = ref<HTMLVideoElement | null>(null);
const phase = ref<Phase>('loading');
const loadingMessage = ref('Подготовка потока…');
const error = ref<string | null>(null);

let sessionId: string | null = null;
let hls: Hls | null = null;
let pollTimer: ReturnType<typeof setTimeout> | null = null;
let stopped = false;
let durationSecs: number | null = null;
let seeking = false;

/** Pin the seekbar to the known total duration (the playlist is still growing). */
function applyKnownDuration(): void {
  if (hls === null || durationSecs === null) {
    return;
  }
  for (const level of hls.levels) {
    if (level.details) {
      level.details.totalduration = durationSecs;
    }
  }
}

function setupHls(url: string): void {
  const el = video.value;
  if (el === null) {
    error.value = 'Видеоэлемент не инициализирован.';
    phase.value = 'error';
    return;
  }

  if (Hls.isSupported()) {
    hls = new Hls();
    hls.loadSource(url);
    hls.attachMedia(el);
    hls.on(Hls.Events.MANIFEST_PARSED, applyKnownDuration);
    hls.on(Hls.Events.LEVEL_UPDATED, applyKnownDuration);
    hls.on(Hls.Events.ERROR, (_event, data) => {
      if (data.fatal) {
        error.value = 'Ошибка воспроизведения потока.';
      }
    });
  } else if (el.canPlayType('application/vnd.apple.mpegurl')) {
    el.src = url;
  } else {
    error.value = 'Этот браузер не поддерживает HLS.';
  }
}

function bufferedEnd(): number {
  const el = video.value;
  if (el === null) {
    return 0;
  }
  const b = el.buffered;
  return b.length > 0 ? b.end(b.length - 1) : 0;
}

/** Restart the pipeline at `positionSecs`, then reload the stream from there. */
async function seekRemote(positionSecs: number): Promise<void> {
  if (sessionId === null || seeking) {
    return;
  }
  seeking = true;
  try {
    await seekPlaybackSession(sessionId, positionSecs);
  } catch (e) {
    fail(e);
    return;
  } finally {
    seeking = false;
  }
  if (hls !== null) {
    hls.destroy();
    hls = null;
  }
  setupHls(playlistUrl(sessionId));
}

/** Forward seek beyond the buffered range needs a pipeline restart. */
function onSeeking(): void {
  if (seeking) {
    return;
  }
  const el = video.value;
  if (el === null || sessionId === null) {
    return;
  }
  const target = el.currentTime;
  const end = bufferedEnd();
  if (end > 0 && target > end + 1) {
    void seekRemote(target);
  }
}

async function start(): Promise<void> {
  try {
    const session = await createPlaybackSession({ source_id: sourceId });
    sessionId = session.session_id;
    void poll();
  } catch (e) {
    fail(e);
  }
}

async function poll(): Promise<void> {
  if (sessionId === null || stopped) {
    return;
  }
  try {
    const s = await getPlaybackStatus(sessionId);
    if (s.status === 'ready' || s.status === 'playing') {
      durationSecs = s.duration_secs ?? null;
      play(playlistUrl(sessionId));
      return;
    }
    if (s.status === 'failed' || s.status === 'expired' || s.status === 'stopped') {
      error.value = s.error ?? `Воспроизведение не удалось: ${s.status}`;
      phase.value = 'error';
      return;
    }
    loadingMessage.value = s.status === 'buffering' ? 'Буферизация…' : 'Подготовка потока…';
    pollTimer = setTimeout(() => void poll(), 500);
  } catch (e) {
    fail(e);
  }
}

function play(url: string): void {
  phase.value = 'ready';
  const el = video.value;
  if (el !== null) {
    el.addEventListener('seeking', onSeeking);
  }
  setupHls(url);
}

function fail(e: unknown): void {
  error.value = e instanceof Error ? e.message : 'Не удалось запустить воспроизведение.';
  phase.value = 'error';
}

async function stop(): Promise<void> {
  if (stopped) {
    return;
  }
  stopped = true;
  if (pollTimer !== null) {
    clearTimeout(pollTimer);
    pollTimer = null;
  }
  if (hls !== null) {
    hls.destroy();
    hls = null;
  }
  video.value?.removeEventListener('seeking', onSeeking);
  if (sessionId !== null) {
    const id = sessionId;
    sessionId = null;
    try {
      await stopPlaybackSession(id);
    } catch {
      // best-effort; the server also cleans up on expiry/shutdown
    }
  }
}

onMounted(() => {
  void start();
});

onBeforeUnmount(() => {
  void stop();
});
</script>

<template>
  <section class="page">
    <header class="page__header">
      <h1 class="page__title">Воспроизведение</h1>
    </header>

    <div v-if="phase === 'loading'" class="state state--loading">
      <span class="spinner" aria-hidden="true"></span>
      <p>{{ loadingMessage }}</p>
    </div>

    <div v-else-if="phase === 'error'" class="state state--error" role="alert">
      <p>{{ error }}</p>
      <RouterLink class="button" to="/">Назад к поиску</RouterLink>
    </div>

    <video
      v-show="phase === 'ready'"
      ref="video"
      class="player"
      controls
      autoplay
      playsinline
    ></video>
  </section>
</template>

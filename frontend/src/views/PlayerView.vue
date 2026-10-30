<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
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

const durationSecs = ref<number | null>(null);
const currentTime = ref(0);
const playing = ref(false);
/** Absolute timeline offset of the current FFmpeg run (0 until a forward seek). */
const seekBase = ref(0);

const absolutePosition = computed(() => seekBase.value + currentTime.value);
/** Seekbar element (uncontrolled: its value is set imperatively on timeupdate,
 *  so a reactive binding doesn't fight the user's drag). */
const seekbar = ref<HTMLInputElement | null>(null);

let sessionId: string | null = null;
let hls: Hls | null = null;
let pollTimer: ReturnType<typeof setTimeout> | null = null;
let stopped = false;
let seeking = false;
let dragging = false;
let hlsWindowSecs = 0;

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

/** Restart the pipeline at `positionSecs`, then reload the stream from there. */
async function seekRemote(positionSecs: number): Promise<void> {
  if (sessionId === null || seeking) {
    return;
  }
  seeking = true;
  try {
    await seekPlaybackSession(sessionId, positionSecs);
  } catch (e) {
    seeking = false;
    fail(e);
    return;
  }
  seekBase.value = positionSecs;
  currentTime.value = 0;
  seeking = false;
  if (hls !== null) {
    hls.destroy();
    hls = null;
  }
  setupHls(playlistUrl(sessionId));
}

function onSeekInput(): void {
  dragging = true;
}

function onSeekChange(e: Event): void {
  dragging = false;
  const target = parseFloat((e.target as HTMLInputElement).value);
  const el = video.value;
  if (el === null || sessionId === null || !Number.isFinite(target)) {
    return;
  }
  // Live (sliding) HLS: the playhead tracks the FFmpeg production edge, so the
  // native seek range is [playhead - HLS window, playhead + small margin].
  // Anything outside it (forward, or backward before the window) restarts.
  const absPos = seekBase.value + currentTime.value;
  const absStart = seekBase.value + Math.max(0, currentTime.value - hlsWindowSecs);
  if (target > absPos + 3 || target < absStart) {
    void seekRemote(target);
  } else {
    el.currentTime = Math.max(0, target - seekBase.value);
  }
}

function onTimeUpdate(): void {
  const el = video.value;
  if (el !== null) {
    currentTime.value = el.currentTime;
    if (!dragging && !seeking && seekbar.value !== null) {
      seekbar.value.value = String(absolutePosition.value);
    }
  }
}

function togglePlay(): void {
  const el = video.value;
  if (el === null) {
    return;
  }
  if (el.paused) {
    void el.play();
  } else {
    el.pause();
  }
}

function formatTime(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  const mm = String(m).padStart(2, '0');
  const sss = String(ss).padStart(2, '0');
  return h > 0 ? `${h}:${mm}:${sss}` : `${m}:${sss}`;
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
      durationSecs.value = s.duration_secs ?? null;
      hlsWindowSecs = s.hls_window_secs ?? 0;
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

async function play(url: string): Promise<void> {
  phase.value = 'ready';
  // The <video> is conditionally rendered: wait for the DOM to mount it so the
  // `video` ref is populated before attaching hls.js.
  await nextTick();
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

    <template v-else>
      <video
        ref="video"
        class="player"
        autoplay
        playsinline
        @timeupdate="onTimeUpdate"
        @play="playing = true"
        @pause="playing = false"
      ></video>

      <div class="controls">
        <button class="controls__play" type="button" @click="togglePlay">
          {{ playing ? '⏸' : '▶' }}
        </button>
        <input
          ref="seekbar"
          class="controls__seek"
          type="range"
          :min="0"
          :max="durationSecs ?? 0"
          step="1"
          aria-label="Позиция"
          @input="onSeekInput"
          @change="onSeekChange"
        />
        <span class="controls__time">
          {{ formatTime(absolutePosition) }} / {{ formatTime(durationSecs ?? 0) }}
        </span>
      </div>
    </template>
  </section>
</template>

<style scoped>
.controls {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-top: 0.75rem;
}
.controls__play {
  width: 2.5rem;
  height: 2.5rem;
  font-size: 1rem;
  line-height: 1;
  cursor: pointer;
}
.controls__seek {
  flex: 1;
}
.controls__time {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
</style>

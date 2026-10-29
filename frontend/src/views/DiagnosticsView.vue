<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';

import { fetchDiagnostics } from '@/api/diagnostics';
import type { DiagnosticsResponse } from '@/types';

type Status = 'loading' | 'ready' | 'error';

const status = ref<Status>('loading');
const data = ref<DiagnosticsResponse | null>(null);
const error = ref<string | null>(null);
let controller: AbortController | null = null;

async function load(): Promise<void> {
  controller?.abort();
  const current = new AbortController();
  controller = current;
  status.value = 'loading';
  error.value = null;

  try {
    const diagnostics = await fetchDiagnostics(current.signal);
    if (current.signal.aborted) {
      return;
    }
    data.value = diagnostics;
    status.value = 'ready';
  } catch (e) {
    if (current.signal.aborted) {
      return;
    }
    error.value = e instanceof Error ? e.message : 'Не удалось загрузить диагностику.';
    status.value = 'error';
  } finally {
    if (controller === current) {
      controller = null;
    }
  }
}

onMounted(() => {
  void load();
});

onBeforeUnmount(() => {
  controller?.abort();
  controller = null;
});
</script>

<template>
  <section class="page">
    <header class="page__header">
      <h1 class="page__title">Диагностика</h1>
    </header>

    <p v-if="status === 'loading'" class="state state--loading">Загрузка…</p>

    <div v-else-if="status === 'error'" class="state state--error" role="alert">
      <p>{{ error }}</p>
      <button class="button button--primary" type="button" @click="load">Повторить</button>
    </div>

    <dl v-else-if="data" class="diagnostics">
      <div class="diagnostics__row">
        <dt>Версия</dt>
        <dd>{{ data.version }}</dd>
      </div>
      <div class="diagnostics__row">
        <dt>Аппаратное ускорение</dt>
        <dd>{{ data.hwaccel_mode }}</dd>
      </div>
      <div class="diagnostics__row">
        <dt>API-токен</dt>
        <dd>{{ data.auth_configured ? 'настроен' : 'не настроен' }}</dd>
      </div>
      <div class="diagnostics__row">
        <dt>Провайдеры</dt>
        <dd>{{ data.providers.length > 0 ? data.providers.join(', ') : 'нет включённых' }}</dd>
      </div>
    </dl>
  </section>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';

import { fetchHealth } from '@/api/health';

type HealthState = 'checking' | 'ok' | 'unavailable';

const state = ref<HealthState>('checking');
const version = ref<string | null>(null);
let controller: AbortController | null = null;

async function check(): Promise<void> {
  controller?.abort();
  const current = new AbortController();
  controller = current;
  state.value = 'checking';
  version.value = null;

  try {
    const health = await fetchHealth(current.signal);
    if (current.signal.aborted) {
      return;
    }
    state.value = 'ok';
    version.value = health.version.length > 0 ? health.version : null;
  } catch {
    if (current.signal.aborted) {
      return;
    }
    state.value = 'unavailable';
  } finally {
    if (controller === current) {
      controller = null;
    }
  }
}

onMounted(() => {
  void check();
});

onBeforeUnmount(() => {
  controller?.abort();
  controller = null;
});
</script>

<template>
  <div class="health" :class="`health--${state}`" role="status">
    <span class="health__dot" aria-hidden="true"></span>
    <span class="visually-hidden">Состояние API:</span>
    <span v-if="state === 'checking'">проверка…</span>
    <span v-else-if="state === 'ok'"
      >API ok<template v-if="version !== null"> · v{{ version }}</template></span
    >
    <template v-else>
      <span>API недоступен</span>
      <button class="button button--ghost button--small" type="button" @click="check">
        Проверить снова
      </button>
    </template>
  </div>
</template>

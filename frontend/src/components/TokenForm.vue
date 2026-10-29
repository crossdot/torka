<script setup lang="ts">
import { ref } from 'vue';

import { clearApiToken, getApiToken, setApiToken } from '@/api/config';

const token = ref(getApiToken());
const saved = ref(false);

function save(): void {
  setApiToken(token.value);
  saved.value = true;
  window.setTimeout(() => {
    saved.value = false;
  }, 1500);
}

function clear(): void {
  clearApiToken();
  token.value = '';
  saved.value = false;
}
</script>

<template>
  <form class="token" @submit.prevent="save">
    <label class="visually-hidden" for="api-token">API-токен</label>
    <input
      id="api-token"
      v-model="token"
      class="token__input"
      type="password"
      autocomplete="off"
      placeholder="API-токен"
    />
    <button class="button button--small" type="submit">Сохранить</button>
    <button
      v-if="token.length > 0"
      class="button button--ghost button--small"
      type="button"
      @click="clear"
    >
      Сбросить
    </button>
    <span v-if="saved" class="token__saved" role="status" aria-label="Токен сохранён">✓</span>
  </form>
</template>

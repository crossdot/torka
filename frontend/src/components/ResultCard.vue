<script setup lang="ts">
import { computed, useId } from 'vue';

import type { SearchItem } from '@/types';

const props = defineProps<{ item: SearchItem }>();

const hintId = `result-hint-${useId()}`;

const year = computed(() => (typeof props.item.year === 'number' ? props.item.year : null));

const size = computed(() => {
  const bytes = props.item.size_bytes;
  if (typeof bytes !== 'number' || !Number.isFinite(bytes) || bytes < 0) {
    return null;
  }
  const units = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const digits = unitIndex === 0 || value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[unitIndex] ?? 'Б'}`;
});

const quality = computed(() => {
  const value = props.item.quality;
  return typeof value === 'string' && value.trim().length > 0 ? value : null;
});

const languages = computed(() => {
  const value = props.item.languages;
  return Array.isArray(value) && value.length > 0 ? value.join(', ') : null;
});

const seeders = computed(() =>
  typeof props.item.seeders === 'number' ? props.item.seeders : null,
);
const leechers = computed(() =>
  typeof props.item.leechers === 'number' ? props.item.leechers : null,
);
</script>

<template>
  <article class="card">
    <header class="card__header">
      <h3 class="card__title">{{ item.title }}</h3>
      <p class="card__facts">
        <span v-if="year !== null">{{ year }}</span>
        <span v-if="size !== null">{{ size }}</span>
        <span class="card__provider" :title="`Провайдер: ${item.provider_id}`">{{
          item.provider_id
        }}</span>
      </p>
    </header>

    <dl v-if="quality !== null || languages !== null" class="card__meta">
      <template v-if="quality !== null">
        <dt>Качество</dt>
        <dd>{{ quality }}</dd>
      </template>
      <template v-if="languages !== null">
        <dt>Языки</dt>
        <dd>{{ languages }}</dd>
      </template>
    </dl>

    <p v-if="seeders !== null" class="card__peers">
      Сиды: {{ seeders }}
      <template v-if="leechers !== null"> · Личи: {{ leechers }}</template>
    </p>

    <footer class="card__footer">
      <button class="button" type="button" disabled :aria-describedby="hintId">Выбрать</button>
      <span :id="hintId" class="card__hint">Выбор источника появится вместе с playback API.</span>
    </footer>
  </article>
</template>

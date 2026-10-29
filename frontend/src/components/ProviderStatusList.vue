<script setup lang="ts">
import type { ProviderStatus } from '@/types';

defineProps<{ providers: readonly ProviderStatus[] }>();

function isFailed(provider: ProviderStatus): boolean {
  return provider.status !== 'ok' || provider.error_code !== null;
}
</script>

<template>
  <section v-if="providers.length > 0" class="providers" aria-label="Статус провайдеров">
    <h2 class="providers__title">Провайдеры</h2>
    <ul class="providers__list">
      <li
        v-for="provider in providers"
        :key="provider.id"
        class="providers__item"
        :class="{ 'providers__item--failed': isFailed(provider) }"
      >
        <span class="providers__id">{{ provider.id }}</span>
        <span class="providers__status">{{ provider.status }}</span>
        <span v-if="typeof provider.duration_ms === 'number'" class="providers__duration">
          {{ provider.duration_ms }} мс
        </span>
        <code v-if="provider.error_code !== null" class="providers__error">{{
          provider.error_code
        }}</code>
      </li>
    </ul>
  </section>
</template>

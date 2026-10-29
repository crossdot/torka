<script setup lang="ts">
import { computed } from 'vue';
import { useRouter } from 'vue-router';

import { hasApiToken } from '@/api/config';
import ProviderStatusList from '@/components/ProviderStatusList.vue';
import ResultCard from '@/components/ResultCard.vue';
import SearchForm from '@/components/SearchForm.vue';
import { useSearch } from '@/composables/useSearch';
import type { SearchItem } from '@/types';

const router = useRouter();

const { state, failedProviders, hasProviderFailures, run, retry } = useSearch();

const status = computed(() => state.status);
const error = computed(() => state.error);
const isLoading = computed(() => status.value === 'loading');
const tokenMissing = !hasApiToken();
const failedProviderIds = computed(() =>
  failedProviders.value.map((provider) => provider.id).join(', '),
);

function onSubmit(payload: { q: string; year: number | null }): void {
  void run(payload);
}

function onSelect(item: SearchItem): void {
  void router.push({ name: 'player', params: { sourceId: item.id } });
}
</script>

<template>
  <section class="page">
    <header class="page__header">
      <h1 class="page__title">Поиск</h1>
      <p class="page__lead">
        Поиск по подключённым провайдерам. Метаданные показываются только тогда, когда их вернул
        провайдер.
      </p>
    </header>

    <p v-if="tokenMissing" class="alert alert--warning" role="status">
      API-токен не задан. Если сервер требует авторизацию, запросы вернут 401 — задайте токен в
      шапке.
    </p>

    <SearchForm :loading="isLoading" @submit="onSubmit" />

    <section class="results" aria-live="polite" :aria-busy="isLoading">
      <p v-if="status === 'idle'" class="state state--idle">Введите запрос и нажмите «Найти».</p>

      <div v-else-if="status === 'loading'" class="state state--loading">
        <span class="spinner" aria-hidden="true"></span>
        <p>Идёт поиск…</p>
      </div>

      <div v-else-if="status === 'error' && error !== null" class="state state--error" role="alert">
        <h2 class="state__title">Поиск не выполнен</h2>
        <p>{{ error.message }}</p>
        <dl class="error-meta">
          <dt>Код</dt>
          <dd>
            <code>{{ error.code }}</code>
          </dd>
          <template v-if="error.requestId !== null">
            <dt>request_id</dt>
            <dd>
              <code>{{ error.requestId }}</code>
            </dd>
          </template>
        </dl>
        <button class="button" type="button" @click="retry()">Повторить</button>
      </div>

      <template v-else>
        <ProviderStatusList :providers="state.providers" />

        <p
          v-if="status === 'results' && hasProviderFailures"
          class="alert alert--warning"
          role="status"
        >
          Часть провайдеров не ответила ({{ failedProviderIds }}). Успешные результаты показаны
          ниже.
        </p>

        <p v-if="status === 'empty'" class="state state--empty">
          По запросу «{{ state.query }}» ничего не найдено<template v-if="state.year !== null">
            за {{ state.year }} год</template
          >.
          <template v-if="hasProviderFailures">
            Провайдеры с ошибкой: {{ failedProviderIds }}; их результаты отсутствуют.
          </template>
        </p>

        <template v-else>
          <p class="results__count">Найдено: {{ state.items.length }}</p>
          <ul class="result-grid">
            <li v-for="item in state.items" :key="item.id">
              <ResultCard :item="item" @select="onSelect" />
            </li>
          </ul>
        </template>
      </template>
    </section>
  </section>
</template>

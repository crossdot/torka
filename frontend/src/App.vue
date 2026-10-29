<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router';

import HealthIndicator from '@/components/HealthIndicator.vue';
</script>

<template>
  <div class="app">
    <header class="app__header">
      <RouterLink class="app__brand" to="/">torka</RouterLink>
      <nav class="app__nav" aria-label="Основная навигация">
        <RouterLink to="/">Поиск</RouterLink>
        <RouterLink to="/diagnostics">Диагностика</RouterLink>
      </nav>
      <HealthIndicator />
    </header>

    <main class="app__main">
      <RouterView />
    </main>

    <footer class="app__footer">
      <p>Веб-клиент torka. API требует Bearer-токен, токен не сохраняется в браузере.</p>
    </footer>
  </div>
</template>

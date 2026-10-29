<script setup lang="ts">
import { RouterLink } from 'vue-router';
</script>

<template>
  <section class="page">
    <h1 class="page__title">Страница не найдена</h1>
    <p class="page__lead">Такого маршрута нет.</p>
    <RouterLink class="button" to="/">Вернуться к поиску</RouterLink>
  </section>
</template>

<script setup lang="ts">
import { ref } from 'vue';

const props = defineProps<{
  loading?: boolean;
  initialQuery?: string;
  initialYear?: number | null;
}>();

const emit = defineEmits<{
  submit: [payload: { q: string; year: number | null }];
}>();

const queryText = ref(props.initialQuery ?? '');
const yearText = ref(props.initialYear == null ? '' : String(props.initialYear));
const validationError = ref<string | null>(null);

function onSubmit(): void {
  const q = queryText.value.trim();
  if (q.length === 0) {
    validationError.value = 'Введите поисковый запрос.';
    return;
  }

  const rawYear = yearText.value.trim();
  let year: number | null = null;
  if (rawYear.length > 0) {
    const parsed = Number(rawYear);
    if (!Number.isInteger(parsed)) {
      validationError.value = 'Год должен быть целым числом.';
      return;
    }
    year = parsed;
  }

  validationError.value = null;
  emit('submit', { q, year });
}
</script>

<template>
  <form class="search-form" role="search" @submit.prevent="onSubmit">
    <div class="search-form__fields">
      <div class="field field--grow">
        <label class="field__label" for="search-query">Название</label>
        <input
          id="search-query"
          v-model="queryText"
          class="field__input"
          type="search"
          name="q"
          autocomplete="off"
          enterkeyhint="search"
          placeholder="Например, название фильма"
          :aria-invalid="validationError !== null"
          aria-describedby="search-validation"
        />
      </div>

      <div class="field field--year">
        <label class="field__label" for="search-year"
          >Год <span class="field__hint">(необязательно)</span></label
        >
        <input
          id="search-year"
          v-model="yearText"
          class="field__input"
          type="number"
          name="year"
          inputmode="numeric"
          min="1888"
          max="2100"
          step="1"
          placeholder="Все годы"
          aria-describedby="search-validation"
        />
      </div>

      <button class="button button--primary search-form__submit" type="submit" :disabled="loading">
        {{ loading ? 'Ищем…' : 'Найти' }}
      </button>
    </div>

    <p id="search-validation" class="search-form__error" role="alert">
      {{ validationError ?? '' }}
    </p>
  </form>
</template>

/**
 * Search state machine: idle -> loading -> results | empty | error.
 *
 * Partial provider failures are modelled separately from the overall status so
 * successful results stay visible (WEB_UI.md). Only the latest request may write
 * state; superseded requests are aborted.
 */

import { computed, onScopeDispose, reactive } from 'vue';

import { ApiError, isAbortError } from '@/api/client';
import { searchTitles } from '@/api/search';
import type { ProviderStatus, SearchItem } from '@/types';

export type SearchStatus = 'idle' | 'loading' | 'results' | 'empty' | 'error';

export interface SearchErrorView {
  readonly code: string;
  readonly message: string;
  readonly requestId: string | null;
}

export interface SearchState {
  status: SearchStatus;
  query: string;
  year: number | null;
  items: SearchItem[];
  providers: ProviderStatus[];
  error: SearchErrorView | null;
}

export interface SearchSubmit {
  readonly q: string;
  readonly year: number | null;
}

export function useSearch() {
  const state = reactive<SearchState>({
    status: 'idle',
    query: '',
    year: null,
    items: [],
    providers: [],
    error: null,
  });

  let controller: AbortController | null = null;

  const failedProviders = computed<ProviderStatus[]>(() =>
    state.providers.filter(
      (provider) => provider.status !== 'ok' || provider.error_code !== null,
    ),
  );
  const hasProviderFailures = computed(() => failedProviders.value.length > 0);

  async function run(input: SearchSubmit): Promise<void> {
    const query = input.q.trim();
    if (query.length === 0) {
      return;
    }

    controller?.abort();
    const current = new AbortController();
    controller = current;

    state.status = 'loading';
    state.query = query;
    state.year = input.year;
    state.items = [];
    state.providers = [];
    state.error = null;

    try {
      const response = await searchTitles({ q: query, year: input.year, signal: current.signal });
      if (current.signal.aborted) {
        return;
      }
      const items = Array.isArray(response.items) ? [...response.items] : [];
      state.items = items;
      state.providers = Array.isArray(response.providers) ? [...response.providers] : [];
      state.status = items.length > 0 ? 'results' : 'empty';
    } catch (error) {
      if (current.signal.aborted || isAbortError(error)) {
        return;
      }
      state.error =
        error instanceof ApiError
          ? { code: error.code, message: error.message, requestId: error.requestId }
          : { code: 'INTERNAL_ERROR', message: 'Не удалось выполнить поиск.', requestId: null };
      state.status = 'error';
    } finally {
      if (controller === current) {
        controller = null;
      }
    }
  }

  function retry(): void {
    if (state.query.length > 0) {
      void run({ q: state.query, year: state.year });
    }
  }

  onScopeDispose(() => {
    controller?.abort();
    controller = null;
  });

  return { state, failedProviders, hasProviderFailures, run, retry };
}

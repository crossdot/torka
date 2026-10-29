import { afterEach, describe, expect, it, vi } from 'vitest';
import { effectScope } from 'vue';

vi.mock('@/api/search', () => ({ searchTitles: vi.fn() }));

import { searchTitles } from '@/api/search';
import { ApiError } from '@/api/client';
import { useSearch } from './useSearch';
import type { SearchItem, SearchResponse } from '@/types';

const mockedSearchTitles = vi.mocked(searchTitles);
const scopes: Array<{ stop: () => void }> = [];

function setup() {
  const scope = effectScope();
  const api = scope.run(() => useSearch());
  if (!api) {
    throw new Error('useSearch did not initialise');
  }
  scopes.push(scope);
  return api;
}

function item(overrides: Partial<SearchItem> = {}): SearchItem {
  return {
    id: 'result-1',
    title: 'Example title',
    year: 2020,
    provider_id: 'provider-id',
    size_bytes: 1_234_567_890,
    seeders: null,
    leechers: null,
    info_hash: null,
    magnet_available: true,
    quality: null,
    languages: [],
    source_url: null,
    ...overrides,
  };
}

function response(overrides: Partial<SearchResponse> = {}): SearchResponse {
  return {
    items: [item()],
    providers: [{ id: 'provider-id', status: 'ok', duration_ms: 123, error_code: null }],
    ...overrides,
  };
}

afterEach(() => {
  for (const scope of scopes.splice(0)) {
    scope.stop();
  }
  vi.clearAllMocks();
});

describe('useSearch', () => {
  it('starts idle', () => {
    const search = setup();
    expect(search.state.status).toBe('idle');
    expect(search.hasProviderFailures.value).toBe(false);
  });

  it('moves to results on successful items', async () => {
    mockedSearchTitles.mockResolvedValueOnce(response());
    const search = setup();

    await search.run({ q: '  matrix  ', year: null });

    expect(search.state.status).toBe('results');
    expect(search.state.query).toBe('matrix');
    expect(search.state.items).toHaveLength(1);
    expect(mockedSearchTitles).toHaveBeenCalledWith({
      q: 'matrix',
      year: null,
      signal: expect.any(AbortSignal),
    });
  });

  it('reports empty when a provider answered without items', async () => {
    mockedSearchTitles.mockResolvedValueOnce(response({ items: [] }));
    const search = setup();

    await search.run({ q: 'nothing', year: 1999 });

    expect(search.state.status).toBe('empty');
    expect(search.state.items).toEqual([]);
    expect(search.state.year).toBe(1999);
  });

  it('keeps successful results while flagging failed providers', async () => {
    mockedSearchTitles.mockResolvedValueOnce(
      response({
        providers: [
          { id: 'provider-id', status: 'ok', duration_ms: 50, error_code: null },
          { id: 'other-provider', status: 'error', duration_ms: 8, error_code: 'PROVIDER_TIMEOUT' },
        ],
      }),
    );
    const search = setup();

    await search.run({ q: 'matrix', year: null });

    expect(search.state.status).toBe('results');
    expect(search.state.items).toHaveLength(1);
    expect(search.hasProviderFailures.value).toBe(true);
    expect(search.failedProviders.value.map((provider) => provider.id)).toEqual(['other-provider']);
  });

  it('exposes API failures with code and request id', async () => {
    mockedSearchTitles.mockRejectedValueOnce(
      new ApiError('PROVIDER_UNAVAILABLE', 'providers are down', {
        status: 503,
        requestId: 'req-9',
      }),
    );
    const search = setup();

    await search.run({ q: 'matrix', year: null });

    expect(search.state.status).toBe('error');
    expect(search.state.error).toEqual({
      code: 'PROVIDER_UNAVAILABLE',
      message: 'providers are down',
      requestId: 'req-9',
    });
    expect(search.state.items).toEqual([]);
  });

  it('retries the last query', async () => {
    mockedSearchTitles
      .mockRejectedValueOnce(new ApiError('NETWORK_ERROR', 'offline'))
      .mockResolvedValueOnce(response());
    const search = setup();

    await search.run({ q: 'matrix', year: 2001 });
    search.retry();
    await vi.waitFor(() => expect(search.state.status).toBe('results'));

    expect(mockedSearchTitles).toHaveBeenCalledTimes(2);
    expect(mockedSearchTitles).toHaveBeenLastCalledWith({
      q: 'matrix',
      year: 2001,
      signal: expect.any(AbortSignal),
    });
  });

  it('ignores responses superseded by a newer request', async () => {
    const { promise: firstResponse, resolve: resolveFirst } =
      Promise.withResolvers<SearchResponse>();
    mockedSearchTitles
      .mockImplementationOnce(() => firstResponse)
      .mockResolvedValueOnce(response({ items: [item({ id: 'newer' })] }));
    const search = setup();

    const first = search.run({ q: 'old', year: null });
    await search.run({ q: 'new', year: null });
    resolveFirst(response({ items: [item({ id: 'older' })] }));
    await first;

    expect(search.state.status).toBe('results');
    expect(search.state.items.map((entry) => entry.id)).toEqual(['newer']);
  });
});

import { afterEach, describe, expect, it, vi } from 'vitest';

import { ApiError, parseErrorPayload, requestJson } from './client';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('parseErrorPayload', () => {
  it('reads the documented error envelope', () => {
    expect(
      parseErrorPayload({
        error: { code: 'VALIDATION_ERROR', message: 'bad query', request_id: 'req-1' },
      }),
    ).toEqual({ code: 'VALIDATION_ERROR', message: 'bad query', request_id: 'req-1' });
  });

  it('accepts a flat code/message payload', () => {
    expect(parseErrorPayload({ code: 'NOT_FOUND', message: 'missing', request_id: 'req-2' })).toEqual(
      { code: 'NOT_FOUND', message: 'missing', request_id: 'req-2' },
    );
  });

  it('rejects bodies without a code and message', () => {
    expect(parseErrorPayload(null)).toBeNull();
    expect(parseErrorPayload({ items: [] })).toBeNull();
    expect(parseErrorPayload({ error: { code: 500 } })).toBeNull();
  });
});

describe('requestJson', () => {
  it('encodes query parameters and drops empty filters', async () => {
    const fetchMock = vi.fn(
      async (_input: RequestInfo | URL, _init?: RequestInit) => jsonResponse({ items: [], providers: [] }),
    );
    vi.stubGlobal('fetch', fetchMock);

    await requestJson('http://api.test/api/v1/search', {
      auth: false,
      query: { q: 'матрица', year: null },
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      'http://api.test/api/v1/search?q=%D0%BC%D0%B0%D1%82%D1%80%D0%B8%D1%86%D0%B0',
    );
  });

  it('returns parsed JSON on success', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => jsonResponse({ status: 'ok', version: '0.1.0' })));

    await expect(requestJson('http://api.test/health', { auth: false })).resolves.toEqual({
      status: 'ok',
      version: '0.1.0',
    });
  });

  it('maps a server error envelope to ApiError', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        jsonResponse(
          { error: { code: 'PROVIDER_TIMEOUT', message: 'provider timed out', request_id: 'req-3' } },
          504,
        ),
      ),
    );

    const error = await requestJson('http://api.test/api/v1/search?q=x', { auth: false }).catch(
      (reason: unknown) => reason,
    );

    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({
      code: 'PROVIDER_TIMEOUT',
      message: 'provider timed out',
      status: 504,
      requestId: 'req-3',
    });
  });

  it('falls back to a status-derived code when a non-2xx body is not an error envelope', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('unauthorized', { status: 401 })));

    const error = await requestJson('http://api.test/api/v1/search?q=x', { auth: false }).catch(
      (reason: unknown) => reason,
    );

    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({ code: 'HTTP_401', status: 401 });
  });

  it('maps a transport failure to NETWORK_ERROR and keeps aborts intact', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new TypeError('Failed to fetch');
      }),
    );
    const networkError = await requestJson('http://api.test/health', { auth: false }).catch(
      (reason: unknown) => reason,
    );
    expect(networkError).toBeInstanceOf(ApiError);
    expect((networkError as ApiError).code).toBe('NETWORK_ERROR');

    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new DOMException('aborted', 'AbortError');
      }),
    );
    const abortError = await requestJson('http://api.test/health', { auth: false }).catch(
      (reason: unknown) => reason,
    );
    expect((abortError as Error).name).toBe('AbortError');
    expect(abortError).not.toBeInstanceOf(ApiError);
  });
});

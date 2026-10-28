# API.md — предварительный контракт

Контракт уточняется в реализации; OpenAPI должен соответствовать фактическому поведению.

## Общие правила
- Base path `/api/v1`, JSON UTF-8.
- Ошибка: `{"error":{"code":"VALIDATION_ERROR","message":"Safe message","request_id":"..."}}`.
- Не раскрывать stack traces, filesystem paths, secrets.
- Bearer token для доступа; не выставлять открытый API в интернет.
- Не полагаться на browser cookies для идентификации.
- Не доверять Host header при построении playback URL.

## Endpoints
- `GET /health` — liveness.
- `GET /ready` — готовность обязательных компонентов; не зависит от доступности всех внешних сайтов.
- `GET /api/v1/search?q=...&year=...` — нормализованные результаты и статус каждого провайдера.
- `GET /api/v1/sources/{id}` — сведения об opaque source ID.
- `POST /api/v1/playback/sessions` — создать сессию; body: `{"source_id":"opaque-id","file_id":null,"preferred_mode":"auto"}`; ответ 202 с `session_id`, `status`, `status_url`.
- `GET /api/v1/playback/sessions/{id}` — статус: resolving/buffering/ready/playing/failed/stopped.
- `DELETE /api/v1/playback/sessions/{id}` — отменить задачи, завершить/reap процессы, очистить кэш; повторный DELETE безопасен.
- `GET /api/v1/playback/sessions/{id}/master.m3u8` — HLS playlist.
- `GET /api/v1/playback/sessions/{id}/segments/{segment}` — HLS segment, привязанный к сессии.

## Search response sketch
```json
{
  "items": [{
    "id": "opaque-result-id",
    "title": "Example title",
    "year": 2020,
    "provider": "provider-id",
    "size_bytes": 1234567890,
    "seeders": null,
    "info_hash": null,
    "magnet_available": true,
    "quality": null,
    "languages": [],
    "source_url": null
  }],
  "providers": [{
    "id": "provider-id",
    "status": "ok",
    "duration_ms": 123,
    "error_code": null
  }]
}
```
Не выдумывать неизвестные поля.

## Error codes
`VALIDATION_ERROR`, `NOT_FOUND`, `PROVIDER_TIMEOUT`, `PROVIDER_UNAVAILABLE`, `SOURCE_UNAVAILABLE`, `UNSUPPORTED_MEDIA`, `SESSION_LIMIT_REACHED`, `SESSION_EXPIRED`, `STREAM_START_FAILED`, `INTERNAL_ERROR`.

## Клиентская совместимость
- Bearer auth; API не зависит от cookies/UA.
- Статус сессии можно опрашивать polling в MVP.
- HLS URL и segment URLs должны быть корректными за reverse proxy.
- Клиент обязан завершать сессию; сервер также делает TTL cleanup.

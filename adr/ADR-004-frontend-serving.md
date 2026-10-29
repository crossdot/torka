# ADR-004: Раздача frontend

- Status: Proposed
- Date: 2026-10-29
- Owners: ведущий инженер (Discovery)

## Context

Vue 3 SPA (Vite build) нужно раздавать self-hosted на Synology одним контейнером.
Вариант — отдельный reverse proxy (nginx/Caddy) либо статика из backend.

## Options

1. **Axum раздаёт собранный SPA** через `tower-http ServeDir` + fallback на `index.html`
   для SPA-маршрутов; `/api/*` и `/health` отдаёт сам backend.
2. Отдельный reverse proxy перед контейнером (TLS/termination, кеш).
3. Два контейнера (frontend-nginx + backend).

## Decision

**Модель 1** в MVP: backend раздаёт production-сборку Vue (один контейнер, один порт,
минимум движущихся частей). Reverse proxy остаётся опциональным внешним слоем для TLS
и удалённого доступа (рекомендован VPN или защищённый proxy), но не обязателен для MVP.
Static assets без аутентификации; `/api/*` и playback-артефакты — под Bearer token.

## Consequences

- `tower-http` уже в стеке (0.7.1); `fs`-feature + `ServeDir`.
- SPA fallback должен не перехватывать `/api/*` и не отдавать 404 API как `index.html`.
- Reverse proxy должен корректно проксировать `Range`/HLS (segments) без буферизации
  ответов сегментов — задокументировать в `DOCKER_SYNOLOGY.md`.

## Validation

- `tower-http 0.7.1` (`fs` feature) совместим с axum 0.8.9 (проверено по dependency metadata).

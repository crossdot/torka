# Развёртывание

Самохостимый сервис: поиск torrent-источников + HLS-воспроизведение. Один контейнер
(Rust backend + Vue SPA + FFmpeg), см. `ADR-004`/`ADR-006`.

## Предварительные требования

- Docker 24+ или Synology Container Manager 24.0.2+.
- CPU **x86_64** (Intel/AMD). Для arm64 — отдельный `Dockerfile.arm64` (software-only).
- Для аппаратного ускорения — Intel iGPU/AMD c VA-API (`/dev/dri/renderD128`).

## Быстрый старт (Compose)

Готовый образ из GHCR (вместо локальной сборки) — см. `README.md`.

```bash
cp ENV.example .env
# отредактируйте .env — обязательно задайте APP_API_TOKEN
docker compose up -d --build
```

Откройте `http://<host>:8080`. Каталог `./cache` должен быть доступен uid/gid 10001:

```bash
install -d -o 10001 -g 10001 ./cache
```

## Сборка образа вручную

```bash
docker build -t movie-streamer:0.1.0 .
```

## Ключевые переменные окружения

Полный список — в `ENV.example`.

| Переменная | Назначение | По умолчанию |
|---|---|---|
| `APP_API_TOKEN` | Bearer-токен API (пусто = API открыт) | пусто |
| `APP_PORT` | HTTP-порт | 8080 |
| `PROVIDER_RUTOR_ENABLED` / `PROVIDER_RUTRACKER_ENABLED` | провайдеры | false |
| `PROVIDER_RUTOR_URLS` | зеркала Rutor (через запятую) | `rutor.info,rutor.is` |
| `PROVIDER_RUTRACKER_URLS` | зеркала RuTracker feed | `feed.rutracker.cc` |
| `HWACCEL_MODE` | `auto` / `software` / `vaapi` / `nvenc` | auto |
| `HWACCEL_DEVICE` | device-node бэкенда (пусто = авто) | `/dev/dri/renderD128` |
| `MAX_ACTIVE_SESSIONS` | лимит одновременных сессий | 2 |
| `CACHE_MAX_BYTES` | лимит временного кеша | 2 GiB |
| `TORRENT_RESOLVE_TIMEOUT_SECS` | таймаут резолва magnet | 60 |

## Аутентификация (ADR-005)

- Задайте `APP_API_TOKEN` в `.env` — сервер начнёт требовать `Authorization: Bearer`.
- В UI введите **тот же** токен в поле «API-токен» в шапке (хранится в `sessionStorage`,
  не в `localStorage` и не в cookie).
- Без `APP_API_TOKEN` API открыт (подходит для доверенной локальной сети).

## Synology Container Manager

1. Установите Container Manager из Package Center.
2. Скопируйте репозиторий (или `compose.yaml` + `Dockerfile` + `ENV.example`) на NAS.
3. `cp ENV.example .env`, задайте `APP_API_TOKEN`.
4. Создайте каталог кеша: `install -d -o 10001 -g 10001 ./cache`.
5. Project → Create → укажите `compose.yaml` → Build/Start.
6. Для VA-API раскомментируйте `devices`/`group_add` в `compose.yaml` и подставьте
   GID render-node: `stat -c '%g' /dev/dri/renderD128`.

Ресурсные лимиты задаются в Container Manager (Compose-`deploy:` на DSM может
игнорироваться).

## Аппаратное ускорение (vendor-agnostic, ADR-008)

- Intel iGPU / AMD → VA-API (`HWACCEL_MODE=vaapi` или `auto`), устройство `/dev/dri/renderD128`.
- NVIDIA → NVENC (`HWACCEL_MODE=nvenc`), требуется драйвер NVIDIA в образе.
- `auto`: проба устройства + тестовый encode → при неудаче software fallback
  (при `HWACCEL_REQUIRED=false`).

Проверить VA-API в контейнере:

```bash
docker exec <container> vainfo --display drm --device /dev/dri/renderD128
```

## Upgrade / rollback

- **Upgrade**: `docker compose build --pull && docker compose up -d`. Активные сессии и
  кеш теряются при перезапуске (по дизайну MVP).
- **Rollback**: переключитесь на предыдущий тег образа (`movie-streamer:<old>`) и
  `up -d`.

## Известные ограничения

- `arm64` (отдельный `Dockerfile.arm64`) и реальный VA-API требуют проверки на конкретном железе.
- Принудительный вынос наружу (`APP_HOST=0.0.0.0`) — для удалённого доступа ставьте
  VPN или reverse proxy с TLS/auth впереди.

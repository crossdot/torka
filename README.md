# torka — self-hosted movie streamer

Самохостимый сервис: поиск torrent-источников (RuTracker feed, Rutor) и потоковое
HLS-воспроизведение без предварительной загрузки. Rust (Axum/Tokio) + Vue 3 + FFmpeg +
librqbit; один Docker-контейнер для Synology и любого Linux-хоста.

## Возможности

- Поиск по провайдерам: Rutor (HTML-поиск), RuTracker (Atom-feed последних раздач),
  фейковый провайдер для тестов. Частичные результаты при сбое провайдера.
- Воспроизведение `magnet` → HLS с перемоткой (seekable loopback source + FFmpeg
  remux, без полной загрузки файла).
- Vendor-agnostic аппаратное ускорение (Intel/AMD VA-API, NVIDIA NVENC) + software fallback.
- Единый клиент-независимый API (`/api/v1`) + Vue-UI.

## Быстрый старт

```bash
cp ENV.example .env      # задайте APP_API_TOKEN
install -d -o 10001 -g 10001 ./cache
docker compose up -d --build
# http://localhost:8080
```

Подробная инструкция (в т.ч. Synology Container Manager, VA-API, upgrade) — в
[`DEPLOYMENT.md`](DEPLOYMENT.md).

## Разработка

```bash
# backend
cargo run                       # см. ENV.example (PROVIDER_*_ENABLED, ...)

# frontend
cd frontend && npm install && npm run dev
```

Проверки:

- Backend: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test`.
- Интеграционный playback-тест (нужен ffmpeg): `cargo test --test playback -- --ignored`.
- Frontend: `npm run lint`, `npm run format:check`, `npm run typecheck`, `npm test`,
  `npm run build`.

## Документация

- [`DEPLOYMENT.md`](DEPLOYMENT.md) — развёртывание (Docker/Synology).
- [`adr/`](adr/) — архитектурные решения (ADR-001…008).
- [`DISCOVERY.md`](DISCOVERY.md) — результаты Discovery (библиотеки, лицензии, провайдеры).
- [`PRODUCT.md`](PRODUCT.md), [`ARCHITECTURE.md`](ARCHITECTURE.md), [`API.md`](API.md),
  [`SECURITY.md`](SECURITY.md), [`STREAMING.md`](STREAMING.md), [`WEB_UI.md`](WEB_UI.md).

## Статус

Реализовано (Phases 0–7 + закрытые зазоры): поиск, провайдеры, streaming, playback API,
плеер, аутентификация, idle-timeout, graceful shutdown, Docker-образ. Известные
ограничения: транскод (`preferred_mode`) не реализован (только remux), `arm64` и
реальный VA-API требуют проверки на железе.

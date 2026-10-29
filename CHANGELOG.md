# Changelog

## 0.1.0 — первый функциональный релиз

Самохостимый сервис: поиск torrent-источников и потоковое HLS-воспроизведение без
предварительной загрузки (remux/stream-copy).

### Возможности

- **Поиск**: Rutor (HTML-поиск), RuTracker (Atom-feed последних раздач), фейковый
  провайдер для тестов; частичные результаты при сбое провайдера, dedupe по info hash.
- **Воспроизведение**: `magnet` → метаданные → поток → FFmpeg → HLS с перемоткой
  (seekable loopback source + `-ss`); сессии с жизненным циклом, лимитами и очисткой.
- **Аппаратное ускорение** (vendor-agnostic): Intel/AMD VA-API, NVIDIA NVENC +
  software fallback.
- **Аутентификация**: один Bearer-токен (`APP_API_TOKEN`), runtime-ввод в UI
  (`sessionStorage`); `/health`/`/ready`/SPA публичны.
- **UI**: поиск, диагностика, плеер (hls.js).
- **Docker**: один контейнер (backend + SPA + FFmpeg), amd64; Synology Container Manager.

### Техническая база

- Rust (Axum 0.8.9, Tokio 1.54, edition 2024, MSRV 1.85), Vue 3 + TS + Vite.
- librqbit 9.0.1 (Apache-2.0) — встроенный torrent-движок.
- FFmpeg 9.x (GPL-сборка в образе, с software H.264).
- Решения зафиксированы в ADR-001…008 (`adr/`).

### Известные ограничения

- `arm64` (отдельный `Dockerfile.arm64`) и реальный VA-API не проверены на железе (нужен физический host).
- RuTracker — только feed (полнотекстовый поиск за Cloudflare; обход не выполняется).
- Аутентификация отключена, пока `APP_API_TOKEN` пуст (API открыт для локальной сети).
